// Native dialogs and embedded icon resources. This module never runs the selected file.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use icoextract_rs::IconExtractor;
use image::{ImageFormat, ImageReader};
use std::{fs::File, io::{Cursor, Read}, path::Path};
use tauri::async_runtime::spawn_blocking;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn pick_shortcut_path(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    mode: String,
) -> Result<Option<String>, String> {
    spawn_blocking(move || {
        let picker = app.dialog().file().set_parent(&window);
        let picked = match mode.as_str() {
            "exe" => picker
                .set_title("Choose a program")
                .add_filter("Windows programs", &["exe"])
                .blocking_pick_file(),
            "file" => picker.set_title("Choose a file").blocking_pick_file(),
            "folder" => picker.set_title("Choose a folder").blocking_pick_folder(),
            "icon" => picker
                .set_title("Choose an icon image or program")
                .add_filter("Icons and programs", &["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "exe", "dll", "mun"])
                .blocking_pick_file(),
            _ => return Err("Unknown picker type.".into()),
        };
        picked
            .map(|item| {
                item.into_path()
                    .map_err(|e| e.to_string())?
                    .into_os_string()
                    .into_string()
                    .map_err(|_| "Windows returned a path this app cannot display.".into())
            })
            .transpose()
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(windows)]
mod windows_icons {
    use std::{ffi::c_void, os::windows::ffi::OsStrExt, path::Path, ptr};
    use image::RgbaImage;
    use windows::{
        Win32::{
            System::Com::{CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, STGM_READ},
            UI::Shell::{IShellLinkW, SLGP_RAWPATH, ShellLink},
        },
        core::{Interface, PCWSTR},
    };

    type Handle = *mut c_void;
    #[repr(C)]
    struct BitmapInfo {
        header_size: u32, width: i32, height: i32, planes: u16, bit_count: u16,
        compression: u32, image_size: u32, x_resolution: i32, y_resolution: i32,
        used_colors: u32, important_colors: u32, colors: [u32; 1],
    }
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn ExtractIconExW(file: *const u16, index: i32, large: *mut Handle, small: *mut Handle, count: u32) -> u32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MultiByteToWideChar(code_page: u32, flags: u32, source: *const u8, len: i32, out: *mut u16, capacity: i32) -> i32;
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn PrivateExtractIconsW(file: *const u16, index: i32, cx: i32, cy: i32,
            icons: *mut Handle, ids: *mut u32, count: u32, flags: u32) -> u32;
        fn DestroyIcon(icon: Handle) -> i32;
        fn DrawIconEx(dc: Handle, x: i32, y: i32, icon: Handle, cx: i32, cy: i32,
            step: u32, brush: Handle, flags: u32) -> i32;
    }
    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn CreateCompatibleDC(dc: Handle) -> Handle;
        fn CreateDIBSection(dc: Handle, info: *const BitmapInfo, usage: u32,
            bits: *mut *mut c_void, section: Handle, offset: u32) -> Handle;
        fn SelectObject(dc: Handle, object: Handle) -> Handle;
        fn DeleteObject(object: Handle) -> i32;
        fn DeleteDC(dc: Handle) -> i32;
        fn GdiFlush() -> i32;
    }
    fn wide(path: &Path) -> Vec<u16> { path.as_os_str().encode_wide().chain([0]).collect() }

    pub fn decode_ansi(bytes: &[u8]) -> Option<String> {
        let len = i32::try_from(bytes.len()).ok()?;
        let capacity = unsafe { MultiByteToWideChar(0, 0, bytes.as_ptr(), len, ptr::null_mut(), 0) };
        if capacity <= 0 { return None; }
        let mut out = vec![0; capacity as usize];
        let len = unsafe { MultiByteToWideChar(0, 0, bytes.as_ptr(), len, out.as_mut_ptr(), capacity) };
        if len <= 0 { return None; }
        String::from_utf16(&out[..len as usize]).ok()
    }

    pub fn link_source(path: &Path) -> Option<(Option<(String, i32)>, Option<String>)> {
        struct Apartment(bool);
        impl Drop for Apartment {
            fn drop(&mut self) { if self.0 { unsafe { CoUninitialize(); } } }
        }
        fn text(buffer: &[u16]) -> Option<String> {
            let end = buffer.iter().position(|&ch| ch == 0)?;
            if end == 0 { return None; }
            String::from_utf16(&buffer[..end]).ok()
        }
        let file = wide(path);
        if file[..file.len() - 1].contains(&0) { return None; }
        unsafe {
            let com = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            if com.is_err() && com.0 as u32 != 0x80010106 { return None; }
            let _apartment = Apartment(com.is_ok());
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
            let persisted: IPersistFile = link.cast().ok()?;
            persisted.Load(PCWSTR(file.as_ptr()), STGM_READ).ok()?;
            let mut buffer = vec![0u16; 32768];
            let mut index = 0i32;
            let icon = link.GetIconLocation(&mut buffer, &mut index).ok().and_then(|()| text(&buffer)).map(|path| (path, index));
            buffer.fill(0);
            let target = link.GetPath(&mut buffer, ptr::null_mut(), SLGP_RAWPATH.0 as u32).ok().and_then(|()| text(&buffer))
                .or_else(|| {
                    buffer.fill(0);
                    link.GetPath(&mut buffer, ptr::null_mut(), 0).ok().and_then(|()| text(&buffer))
                });
            Some((icon, target))
        }
    }
    pub fn count(path: &Path) -> usize {
        let file = wide(path);
        let n = unsafe { ExtractIconExW(file.as_ptr(), -1, ptr::null_mut(), ptr::null_mut(), 0) };
        if n == u32::MAX { 0 } else { n as usize }
    }
    pub fn read(path: &Path, index: i32, size: i32) -> Option<RgbaImage> {
        if !(-65535..=65535).contains(&index) { return None; }
        let file = wide(path);
        unsafe {
            let mut icon = ptr::null_mut();
            PrivateExtractIconsW(file.as_ptr(), index, size, size, &mut icon, ptr::null_mut(), 1, 0);
            if icon.is_null() {
                ExtractIconExW(file.as_ptr(), index, &mut icon, ptr::null_mut(), 1);
            }
            if icon.is_null() { return None; }
            let result = pixels(icon, size);
            DestroyIcon(icon);
            result
        }
    }
    unsafe fn pixels(icon: Handle, size: i32) -> Option<RgbaImage> {
        let info = BitmapInfo { header_size: 40, width: size, height: -size, planes: 1,
            bit_count: 32, compression: 0, image_size: 0, x_resolution: 0, y_resolution: 0,
            used_colors: 0, important_colors: 0, colors: [0] };
        unsafe {
            let dc = CreateCompatibleDC(ptr::null_mut());
            if dc.is_null() { return None; }
            let mut bits: *mut c_void = ptr::null_mut();
            let bitmap = CreateDIBSection(dc, &info, 0, &mut bits, ptr::null_mut(), 0);
            if bitmap.is_null() || bits.is_null() {
                if !bitmap.is_null() { DeleteObject(bitmap); }
                DeleteDC(dc);
                return None;
            }
            let old = SelectObject(dc, bitmap);
            let out = if old.is_null() || old as isize == -1 {
                None
            } else {
                // Recover straight-alpha RGBA by measuring compositing against black
                // and white. This also handles older icons with a binary AND mask.
                let data = std::slice::from_raw_parts_mut(bits.cast::<u8>(), (size * size * 4) as usize);
                data.fill(0);
                let black_ok = DrawIconEx(dc, 0, 0, icon, size, size, 0, ptr::null_mut(), 3) != 0;
                GdiFlush();
                let black = data.to_vec();
                data.fill(255);
                let white_ok = DrawIconEx(dc, 0, 0, icon, size, size, 0, ptr::null_mut(), 3) != 0;
                GdiFlush();
                let result = if black_ok && white_ok {
                    let mut rgba = Vec::with_capacity(data.len());
                    for (b, w) in black.chunks_exact(4).zip(data.chunks_exact(4)) {
                        let clear = (0..3).map(|i| w[i].saturating_sub(b[i]) as u16).sum::<u16>() / 3;
                        let alpha = 255 - clear;
                        let channel = |i: usize| if alpha == 0 { 0 } else {
                            ((u16::from(b[i]) * 255 + alpha / 2) / alpha).min(255) as u8
                        };
                        rgba.extend([channel(2), channel(1), channel(0), alpha as u8]);
                    }
                    if rgba.iter().skip(3).step_by(4).all(|&a| a == 0) { None }
                    else { RgbaImage::from_raw(size as u32, size as u32, rgba) }
                } else { None };
                SelectObject(dc, old);
                result
            };
            DeleteObject(bitmap);
            DeleteDC(dc);
            out
        }
    }
}

#[derive(serde::Serialize)]
pub struct ShortcutIcon {
    source: String,
    count: usize,
    index: i32,
}

#[derive(serde::Serialize)]
pub struct IconThumbnail {
    index: usize,
    source: String,
}

#[derive(serde::Serialize)]
pub struct IconPage {
    count: usize,
    entries: Vec<IconThumbnail>,
}

fn icon_path(target: &str) -> Result<(&Path, String), String> {
    let path = Path::new(target);
    if !path.is_absolute() || !path.is_file() {
        return Err("Choose an existing icon file.".into());
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    Ok((path, ext))
}

fn png_source(image: image::DynamicImage, max: u32) -> Result<String, String> {
    let icon = if image.width() > max || image.height() > max { image.thumbnail(max, max) } else { image };
    let mut buffer = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(icon.into_rgba8()).write_to(&mut buffer, ImageFormat::Png)
        .map_err(|e| format!("Could not save the selected icon: {e}"))?;
    Ok(format!("data:image/png;base64,{}", STANDARD.encode(buffer.into_inner())))
}

struct ResourceIcons<'a> {
    path: &'a Path,
    native_count: usize,
    fallback: Option<IconExtractor>,
    fallback_count: usize,
}
impl<'a> ResourceIcons<'a> {
    fn open(path: &'a Path) -> Self {
        #[cfg(windows)]
        let native_count = windows_icons::count(path).min(65536);
        #[cfg(not(windows))]
        let native_count = 0;
        let mut icons = Self { path, native_count, fallback: None, fallback_count: 0 };
        if native_count == 0 { icons.open_fallback(); }
        icons
    }
    fn open_fallback(&mut self) {
        self.fallback = IconExtractor::from_path(self.path).ok();
        self.fallback_count = self.fallback.as_ref().and_then(|ar| ar.list_group_icons().ok())
            .map(|groups| groups.len().min(65536)).unwrap_or(0);
    }
    fn count(&self) -> usize {
        if self.native_count > 0 { self.native_count } else { self.fallback_count }
    }
    fn read(&mut self, index: usize, size: u32) -> Option<image::DynamicImage> {
        if index >= self.count() { return None; }
        #[cfg(windows)]
        if self.native_count > 0 {
            if let Some(icon) = windows_icons::read(self.path, index as i32, size as i32) {
                return Some(image::DynamicImage::ImageRgba8(icon));
            }
            // Keep the numbering consistent within a file. Mixing the index order
            // of two independent readers could show the wrong selected icon.
            return None;
        }
        let bytes = self.fallback.as_ref()?.icon_by_index(index).ok()?.to_ico_bytes().ok()?;
        ImageReader::with_format(Cursor::new(bytes), ImageFormat::Ico).decode().ok()
    }
}

#[tauri::command]
pub async fn list_shortcut_icons(target: String, offset: usize) -> Result<IconPage, String> {
    spawn_blocking(move || {
        let (path, ext) = icon_path(&target)?;
        if !matches!(ext.as_str(), "exe" | "dll" | "mun") {
            return Err("Choose an EXE, DLL, or MUN to browse embedded icons.".into());
        }
        let mut icons = ResourceIcons::open(path);
        let count = icons.count();
        let entries = (offset..offset.saturating_add(24).min(count))
            .filter_map(|index| {
                let source = png_source(icons.read(index, 48)?, 48).ok()?;
                Some(IconThumbnail { index, source })
            }).collect();
        Ok(IconPage { count, entries })
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn read_shortcut_icon(target: String, index: i32, automatic: Option<bool>) -> Result<ShortcutIcon, String> {
    spawn_blocking(move || {
        if automatic == Some(true) && !is_drive_path(Path::new(&target)) {
            return Err("The automatic icon source is not a local drive path.".into());
        }
        let (path, ext) = icon_path(&target)?;
        if matches!(ext.as_str(), "exe" | "dll" | "mun") {
            let missing = || "No usable icon was found. Choose another file or keep the current icon.".to_owned();
            #[cfg(windows)]
            if index < 0 || automatic == Some(true) {
                // A negative index denotes a resource ID. Don't turn it into a
                // gallery offset or silently replace the requested icon.
                if let Some(icon) = windows_icons::read(path, index, 128) {
                    return Ok(ShortcutIcon { source: png_source(image::DynamicImage::ImageRgba8(icon), 256)?, count: 1, index });
                }
            }
            let mut icons = ResourceIcons::open(path);
            let count = icons.count();
            let icon = usize::try_from(index).ok().and_then(|index| icons.read(index, 128)).ok_or_else(missing)?;
            return Ok(ShortcutIcon { source: png_source(icon, 256)?, count, index });
        }
        let format = match ext.as_str() {
            "png" => ImageFormat::Png,
            "jpg" | "jpeg" => ImageFormat::Jpeg,
            "gif" => ImageFormat::Gif,
            "webp" => ImageFormat::WebP,
            "bmp" => ImageFormat::Bmp,
            "ico" => ImageFormat::Ico,
            _ => return Err("Choose a supported image, ICO, EXE, DLL, or MUN.".into()),
        };
        let mut reader = ImageReader::open(path).map_err(|e| e.to_string())?;
        reader.set_format(format);
        let icon = reader.decode().map_err(|_| "Could not read this image. Try a PNG or ICO.".to_string())?;
        Ok(ShortcutIcon { source: png_source(icon, 256)?, count: 1, index: 0 })
    }).await.map_err(|e| e.to_string())?
}

#[derive(serde::Serialize)]
struct IconSource {
    path: String,
    index: i32,
}

// Only consult an explicitly named file on a drive. Don't treat a URL,
// network share, device name, or working-directory-relative path as an icon.
fn is_drive_path(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        path.is_absolute() && matches!(path.components().next(),
            Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
    }
    #[cfg(not(windows))]
    { path.is_absolute() }
}

fn expand_path(value: &str) -> String {
    let mut result = String::new();
    let mut rest = value.trim().trim_matches('"');
    while let Some((prefix, after)) = rest.split_once('%') {
        result.push_str(prefix);
        let Some((name, tail)) = after.split_once('%') else {
            result.push('%'); result.push_str(after);
            return result;
        };
        if !name.is_empty() && !name.contains('=') && !name.contains('\0') {
            if let Ok(value) = std::env::var(name) { result.push_str(&value); rest = tail; continue; }
        }
        result.push('%'); result.push_str(name); result.push('%');
        rest = tail;
    }
    result.push_str(rest);
    result
}

fn checked_source(raw: &str, index: i32, exe_only: bool) -> Option<IconSource> {
    let path = expand_path(raw);
    if path.is_empty() || path.len() > 32768 || path.contains('\0') { return None; }
    let source = Path::new(&path);
    if !is_drive_path(source) { return None; }
    let ext = source.extension()?.to_str()?.to_ascii_lowercase();
    if (exe_only && ext != "exe") || !matches!(ext.as_str(), "exe" | "dll" | "mun" | "ico" | "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp") {
        return None;
    }
    source.is_file().then_some(IconSource { path, index })
}

fn decode_shortcut_text(bytes: &[u8]) -> Option<String> {
    if let Some((data, little_endian)) = bytes.strip_prefix(&[0xff, 0xfe]).map(|b| (b, true))
        .or_else(|| bytes.strip_prefix(&[0xfe, 0xff]).map(|b| (b, false))) {
        if data.len() % 2 != 0 { return None; }
        let units: Vec<_> = data.chunks_exact(2).map(|b| {
            if little_endian { u16::from_le_bytes([b[0], b[1]]) } else { u16::from_be_bytes([b[0], b[1]]) }
        }).collect();
        return String::from_utf16(&units).ok();
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    if let Ok(text) = std::str::from_utf8(bytes) { return Some(text.to_owned()); }
    #[cfg(windows)]
    { windows_icons::decode_ansi(bytes) }
    #[cfg(not(windows))]
    { None }
}

fn internet_icon_fields(text: &str) -> Option<(&str, i32)> {
    let (mut file, mut index, mut in_section) = (None, 0, false);
    for line in text.trim_end_matches('\0').lines().map(str::trim) {
        if line.starts_with('[') && line.ends_with(']') {
            in_section = line[1..line.len() - 1].trim().eq_ignore_ascii_case("InternetShortcut");
            continue;
        }
        if !in_section || line.starts_with(';') || line.starts_with('#') { continue; }
        let Some((key, value)) = line.split_once('=') else { continue; };
        if key.trim().eq_ignore_ascii_case("IconFile") { file = Some(value.trim()); }
        if key.trim().eq_ignore_ascii_case("IconIndex") { index = value.trim().parse::<i32>().unwrap_or(0); }
    }
    file.filter(|path| !path.is_empty()).map(|path| (path, index))
}

#[cfg(test)]
mod icon_source_tests {
    use super::{decode_shortcut_text, internet_icon_fields};

    #[test]
    fn reads_steam_icon_without_using_the_launch_url() {
        let shortcut = "[{000214A0-0000-0000-C000-000000000046}]\r\nProp3=19,0\r\n[InternetShortcut]\r\nIDList=\r\nIconIndex=0\r\nURL=steam://rungameid/228300\r\nIconFile=C:\\Program Files (x86)\\Steam\\steam\\games\\0c0db50e00344f8c5d7ae9e889a4afc51ec5a1b8.ico\r\n";
        assert_eq!(internet_icon_fields(shortcut), Some(("C:\\Program Files (x86)\\Steam\\steam\\games\\0c0db50e00344f8c5d7ae9e889a4afc51ec5a1b8.ico", 0)));
        assert_eq!(internet_icon_fields("[Other]\nIconFile=ignore.ico\n[InternetShortcut]\nURL=steam://rungameid/228300"), None);
    }

    #[test]
    fn reads_signed_indexes_and_unicode_paths() {
        let text = "[internetshortcut]\niconfile=C:\\游戏\\a=b.dll\niconindex=-42\n";
        let bytes: Vec<_> = [0xff, 0xfe].into_iter().chain(text.encode_utf16().flat_map(u16::to_le_bytes)).collect();
        let decoded = decode_shortcut_text(&bytes).unwrap();
        assert_eq!(internet_icon_fields(&decoded), Some(("C:\\游戏\\a=b.dll", -42)));
    }
}

fn discover_icon_source(path: &Path) -> Option<IconSource> {
    if !is_drive_path(path) { return None; }
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "exe" => checked_source(path.to_str()?, 0, true),
        "url" => {
            let mut bytes = Vec::new();
            File::open(path).ok()?.take(65537).read_to_end(&mut bytes).ok()?;
            if bytes.len() > 65536 { return None; }
            let text = decode_shortcut_text(&bytes)?;
            let (icon, index) = internet_icon_fields(&text)?;
            checked_source(icon, index, false)
        }
        #[cfg(windows)]
        "lnk" => {
            let (icon, target) = windows_icons::link_source(path)?;
            if let Some(source) = icon.and_then(|(icon, index)| checked_source(&icon, index, false)) {
                // If a shortcut's explicit embedded icon disappeared from a
                // replaced binary, try the shortcut target as the fallback.
                let ext = Path::new(&source.path).extension().and_then(|ext| ext.to_str()).unwrap_or("");
                let is_resource = ["exe", "dll", "mun"].iter().any(|kind| ext.eq_ignore_ascii_case(kind));
                if !is_resource || windows_icons::read(Path::new(&source.path), source.index, 32).is_some() {
                    return Some(source);
                }
            }
            checked_source(&target?, 0, true)
        }
        _ => None,
    }
}

#[derive(serde::Serialize)]
pub struct DroppedTarget {
    target: String,
    name: String,
    kind: &'static str,
    #[serde(rename = "localType")]
    local_type: &'static str,
    #[serde(rename = "iconSource", skip_serializing_if = "Option::is_none")]
    icon_source: Option<IconSource>,
}

#[tauri::command]
pub async fn inspect_shortcut_drops(paths: Vec<String>, include_icon_source: Option<bool>) -> Result<Vec<DroppedTarget>, String> {
    spawn_blocking(move || {
        paths.into_iter().take(64).filter_map(|target| {
            let path = Path::new(&target);
            if !path.is_absolute() { return None; }
            let meta = path.metadata().ok()?;
            let is_program = meta.is_file() && path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"));
            let is_link = meta.is_file() && path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("url") || ext.eq_ignore_ascii_case("lnk"));
            let name = (if is_program || is_link { path.file_stem() } else { path.file_name() })
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| target.clone());
            let icon_source = if include_icon_source == Some(true) && meta.is_file() { discover_icon_source(path) } else { None };
            Some(DroppedTarget {
                target,
                name,
                kind: if is_program { "program" } else { "local" },
                local_type: if meta.is_dir() { "folder" } else { "file" },
                icon_source,
            })
        }).collect()
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn exit_stack_dock(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub async fn stack_dock_layout_backup(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    data: Option<String>,
) -> Result<Option<String>, String> {
    const LIMIT: u64 = 8 * 1024 * 1024;
    spawn_blocking(move || {
        let picker = app.dialog().file().set_parent(&window).add_filter("JSON layout", &["json"]);
        if let Some(text) = data {
            if text.len() as u64 > LIMIT {
                return Err("The layout is too large to export.".into());
            }
            let Some(item) = picker
                .set_title("Export Stack Dock layout")
                .set_file_name("stack-dock-layout.json")
                .blocking_save_file()
            else {
                return Ok(None);
            };
            let path = item.into_path().map_err(|e| e.to_string())?;
            std::fs::write(&path, text).map_err(|e| e.to_string())?;
            Ok(Some(path.to_string_lossy().into_owned()))
        } else {
            let Some(item) = picker.set_title("Import Stack Dock layout").blocking_pick_file() else {
                return Ok(None);
            };
            let path = item.into_path().map_err(|e| e.to_string())?;
            let mut text = String::new();
            File::open(path)
                .map_err(|e| e.to_string())?
                .take(LIMIT + 1)
                .read_to_string(&mut text)
                .map_err(|e| e.to_string())?;
            if text.len() as u64 > LIMIT {
                return Err("That layout file is larger than 8 MB.".into());
            }
            Ok(Some(text))
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

pub fn keep_dock_open(window: &tauri::Window, event: &tauri::WindowEvent) {
    if window.label() == "main" {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
        }
    }
}
