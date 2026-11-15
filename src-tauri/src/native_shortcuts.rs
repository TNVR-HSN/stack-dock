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
                .add_filter("Icons and programs", &["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "exe"])
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

#[derive(serde::Serialize)]
pub struct ShortcutIcon {
    source: String,
    count: usize,
}

#[tauri::command]
pub async fn read_shortcut_icon(target: String, index: usize) -> Result<ShortcutIcon, String> {
    spawn_blocking(move || {
        let path = Path::new(&target);
        if !path.is_absolute() || !path.is_file() {
            return Err("Choose an existing icon image or EXE.".into());
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        // Decode on the worker and send only a small PNG across the bridge.
        // There is no source-file size cutoff. The decoder's memory protections
        // still apply to invalid or exceptionally large decoded images.
        let (decoded, count) = if ext == "exe" {
            let extractor = IconExtractor::from_path(path)
                .map_err(|e| format!("Could not read program icons: {e}"))?;
            let count = extractor.list_group_icons().map_err(|e| e.to_string())?.len();
            let bytes = extractor.icon_by_index(index)
                .and_then(|icon| icon.to_ico_bytes())
                .map_err(|e| format!("Could not extract icon {}: {e}", index + 1))?;
            (ImageReader::with_format(Cursor::new(bytes), ImageFormat::Ico).decode(), count)
        } else {
            let format = match ext.as_str() {
                "png" => ImageFormat::Png,
                "jpg" | "jpeg" => ImageFormat::Jpeg,
                "gif" => ImageFormat::Gif,
                "webp" => ImageFormat::WebP,
                "bmp" => ImageFormat::Bmp,
                "ico" => ImageFormat::Ico,
                _ => return Err("Choose a PNG, JPEG, GIF, WebP, BMP, ICO, or EXE.".into()),
            };
            let mut reader = ImageReader::open(path).map_err(|e| e.to_string())?;
            reader.set_format(format);
            (reader.decode(), 1)
        };
        let decoded = decoded.map_err(|e| format!("Could not decode this icon: {e}"))?;
        let icon = if decoded.width() > 256 || decoded.height() > 256 {
            decoded.thumbnail(256, 256)
        } else { decoded };
        let mut buffer = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(icon.into_rgba8())
            .write_to(&mut buffer, ImageFormat::Png)
            .map_err(|e| format!("Could not save the selected icon: {e}"))?;
        Ok(ShortcutIcon {
            source: format!("data:image/png;base64,{}", STANDARD.encode(buffer.into_inner())),
            count,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(serde::Serialize)]
pub struct DroppedTarget {
    target: String,
    name: String,
    kind: &'static str,
}

#[tauri::command]
pub async fn inspect_shortcut_drops(paths: Vec<String>) -> Result<Vec<DroppedTarget>, String> {
    spawn_blocking(move || {
        paths.into_iter().take(64).filter_map(|target| {
            let path = Path::new(&target);
            if !path.is_absolute() { return None; }
            let meta = path.metadata().ok()?;
            let is_program = meta.is_file() && path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"));
            let name = (if is_program { path.file_stem() } else { path.file_name() })
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| target.clone());
            Some(DroppedTarget {
                target,
                name,
                kind: if is_program { "program" } else { "local" },
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
