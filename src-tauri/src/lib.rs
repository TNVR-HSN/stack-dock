// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod native_shortcuts;
mod layout_store;

#[tauri::command]
fn set_dock_bounds(
    window: tauri::WebviewWindow,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<(), String> {
    use std::ffi::c_void;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn SetWindowPos(
            hwnd: *mut c_void,
            insert_after: *mut c_void,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
    }

    if width <= 0 || height <= 0 {
        return Err("Window dimensions must be positive.".into());
    }

    let hwnd = window.hwnd().map_err(|error| error.to_string())?;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_NOACTIVATE: u32 = 0x0010;

    let result = unsafe {
        SetWindowPos(
            hwnd.0 as *mut c_void,
            std::ptr::null_mut(),
            x,
            y,
            width,
            height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
    };

    if result == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }

    Ok(())
}

#[tauri::command]
fn set_dock_region(
    window: tauri::WebviewWindow,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    radius: Option<i32>,
    clip_top: Option<i32>,
    companion: Option<[i32; 4]>,
    popup: Option<[i32; 4]>,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::ffi::c_void;

        #[link(name = "gdi32")]
        unsafe extern "system" {
            fn CreateRectRgn(left: i32, top: i32, right: i32, bottom: i32) -> *mut c_void;
            fn CreateRoundRectRgn(left: i32, top: i32, right: i32, bottom: i32, ew: i32, eh: i32) -> *mut c_void;
            fn DeleteObject(object: *mut c_void) -> i32;
            fn CombineRgn(dst: *mut c_void, first: *mut c_void, second: *mut c_void, mode: i32) -> i32;
        }

        #[link(name = "user32")]
        unsafe extern "system" {
            fn SetWindowRgn(window: *mut c_void, region: *mut c_void, redraw: i32) -> i32;
        }

        let hwnd = window.hwnd().map_err(|e| e.to_string())?;

        // The frontend sends a zero-sized request to check that this command is available.
        if width == 0 && height == 0 {
            return Ok(());
        }
        if (clip_top.is_none() && (x < 0 || y < 0)) || width <= 0 || height <= 0 {
            return Err("Invalid dock region".into());
        }

        for [left, top, right, bottom] in [companion, popup].into_iter().flatten() {
            if left < 0 || top < 0 || right <= left || bottom <= top {
                return Err("Invalid companion dock region".into());
            }
        }
        let right = x.checked_add(width).ok_or("Dock region is too wide")?;
        let bottom = y.checked_add(height).ok_or("Dock region is too tall")?;

        unsafe {
            let diameter = 2 * radius.unwrap_or(0).clamp(0, width.min(height) / 2);
            let region = if diameter == 0 {
                CreateRectRgn(x, y, right, bottom)
            } else {
                CreateRoundRectRgn(x, y, right, bottom, diameter, diameter)
            };
            if region.is_null() {
                return Err("Windows could not create the dock region".into());
            }
            // Intersect the original rounded dialog region with the safe area.
            // Moving partly past an edge must not create new rounded corners at the cut.
            if let Some(top) = clip_top.filter(|&top| top > y) {
                let clip = CreateRectRgn(x, top.min(bottom), right, bottom);
                if clip.is_null() {
                    DeleteObject(region);
                    return Err("Windows could not create the caption clip".into());
                }
                const RGN_AND: i32 = 1;
                let status = CombineRgn(region, region, clip, RGN_AND);
                DeleteObject(clip);
                if status == 0 {
                    DeleteObject(region);
                    return Err("Windows could not clip the dialog".into());
                }
            }
            // Keep Settings, the dock, and its context menu interactive without
            // exposing the transparent desktop between their regions.
            for [left, top, right, bottom] in [companion, popup].into_iter().flatten() {
                let extra = CreateRectRgn(left, top, right, bottom);
                if extra.is_null() {
                    DeleteObject(region);
                    return Err("Windows could not create the companion dock region".into());
                }
                const RGN_OR: i32 = 2;
                let status = CombineRgn(region, region, extra, RGN_OR);
                DeleteObject(extra);
                if status == 0 {
                    DeleteObject(region);
                    return Err("Windows could not combine the Settings and dock regions".into());
                }
            }
            if SetWindowRgn(hwnd.0 as *mut c_void, region, 1) == 0 {
                DeleteObject(region);
                return Err("Windows could not apply the dock region".into());
            }
            // Windows owns the region after a successful call.
        }
        Ok(())
    }

    #[cfg(not(windows))]
    {
        let _ = (window, x, y, width, height, radius, clip_top, companion, popup);
        Err("Dock regions are only supported on Windows".into())
    }
}

#[tauri::command]
fn launch_program(target: String, arguments: String, start_in: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::{os::windows::process::CommandExt, path::Path, process::Command};

        let exe = Path::new(target.trim().trim_matches('"'));
        if !exe.is_absolute()
            || !exe.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            return Err("Enter the full path to a Windows .exe file.".into());
        }
        if !exe.is_file() {
            return Err(format!("Program not found: {}", exe.display()));
        }

        let mut command = Command::new(exe);
        let folder = start_in.trim().trim_matches('"');

        if folder.is_empty() {
            if let Some(parent) = exe.parent() {
                command.current_dir(parent);
            }
        } else {
            let dir = Path::new(folder);
            if !dir.is_absolute() || !dir.is_dir() {
                return Err("Start in must be the full path to an existing folder.".into());
            }
            command.current_dir(dir);
        }

        if !arguments.trim().is_empty() {
            command.raw_arg(arguments.trim());
        }

        match command.spawn() {
            Ok(child) => {
                eprintln!("[StackDock] Started {} (PID {})", exe.display(), child.id());
                Ok(())
            }
            Err(error) => {
                eprintln!("[StackDock] Could not start {}: {}", exe.display(), error);
                Err(error.to_string())
            }
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (target, arguments, start_in);
        Err("Program shortcuts are only supported on Windows.".into())
    }
}

#[tauri::command]
fn set_settings_window_icon(
    window: tauri::WebviewWindow,
    icon_png: String,
) -> Result<(), String> {
    use base64::Engine as _;

    let encoded = icon_png
        .strip_prefix("data:image/png;base64,")
        .ok_or("The Settings icon must be a PNG data URL.")?;
    if encoded.len() > 262_144 {
        return Err("The Settings icon image is too large.".into());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| format!("Could not decode the Settings icon: {error}"))?;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| format!("Could not read the Settings icon: {error}"))?;
    if image.width() != 128 || image.height() != 128 {
        return Err("The Settings icon must be 128 by 128 pixels.".into());
    }
    let rgba = image.into_rgba8();
    window
        .set_icon(tauri::image::Image::new_owned(
            rgba.into_raw(),
            128,
            128,
        ))
        .map_err(|error| format!("Could not update the Settings window icon: {error}"))
}

#[tauri::command]
fn open_shortcut_path(app: tauri::AppHandle, target: String) -> Result<(), String> {
    use std::path::Path;
    use tauri_plugin_opener::OpenerExt;

    let target = target.trim().trim_matches('"');
    let path = Path::new(target);

    if !path.is_absolute() {
        return Err("Enter the full path to a file or folder.".into());
    }

    if !path.exists() {
        return Err(format!("File or folder not found: {}", path.display()));
    }

    app.opener()
        .open_path(target, None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn open_shortcut_destination(app: tauri::AppHandle, target: String) -> Result<(), String> {
    use std::path::Path;
    use tauri_plugin_opener::OpenerExt;

    let path = Path::new(target.trim().trim_matches('"'));
    if !path.is_absolute() {
        return Err("Enter a full file or folder path.".into());
    }
    let folder = if path.is_dir() {
        path
    } else {
        path.parent().ok_or("This shortcut has no containing folder.")?
    };
    if !folder.is_dir() {
        return Err(format!("Destination folder not found: {}", folder.display()));
    }
    app.opener().open_path(folder.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|error| error.to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StartupStatus {
    enabled: bool,
    can_enable: bool,
}

#[cfg(windows)]
mod windows_startup {
    use std::{
        mem::size_of,
        path::{Path, PathBuf},
    };
    use windows::{
        Win32::{
            Foundation::WIN32_ERROR,
            System::Registry::{
                HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ, RegCloseKey,
                RegCreateKeyW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
            },
        },
        core::PCWSTR,
    };

    const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    const VALUE_NAME: &str = "stack-dock";

    struct RegistryKey(HKEY);

    impl RegistryKey {
        fn open(
            access: windows::Win32::System::Registry::REG_SAM_FLAGS,
        ) -> Result<Option<Self>, String> {
            let path: Vec<u16> = RUN_KEY.encode_utf16().chain([0]).collect();
            let mut key = HKEY::default();
            let error = unsafe {
                RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    PCWSTR(path.as_ptr()),
                    None,
                    access,
                    &mut key,
                )
            };
            if error.0 == 2 {
                return Ok(None);
            }
            if error.0 != 0 {
                return Err(registry_error(error));
            }
            Ok(Some(Self(key)))
        }

        fn create() -> Result<Self, String> {
            let path: Vec<u16> = RUN_KEY.encode_utf16().chain([0]).collect();
            let mut key = HKEY::default();
            let error =
                unsafe { RegCreateKeyW(HKEY_CURRENT_USER, PCWSTR(path.as_ptr()), &mut key) };
            if error.0 != 0 {
                return Err(registry_error(error));
            }
            Ok(Self(key))
        }
    }

    impl Drop for RegistryKey {
        fn drop(&mut self) {
            let error = unsafe { RegCloseKey(self.0) };
            if error.0 != 0 {
                eprintln!(
                    "[StackDock] Could not close the Windows startup registry key: {}",
                    std::io::Error::from_raw_os_error(error.0 as i32)
                );
            }
        }
    }

    fn registry_error(error: WIN32_ERROR) -> String {
        format!(
            "Windows startup registry operation failed: {}",
            std::io::Error::from_raw_os_error(error.0 as i32)
        )
    }

    fn value_name() -> Vec<u16> {
        VALUE_NAME.encode_utf16().chain([0]).collect()
    }

    fn read_registration() -> Result<Option<String>, String> {
        let Some(key) = RegistryKey::open(KEY_QUERY_VALUE)? else {
            return Ok(None);
        };
        let name = value_name();
        let mut kind = REG_SZ;
        let mut byte_len = 0u32;
        let error = unsafe {
            RegQueryValueExW(
                key.0,
                PCWSTR(name.as_ptr()),
                None,
                Some(&mut kind),
                None,
                Some(&mut byte_len),
            )
        };
        if error.0 == 2 {
            return Ok(None);
        }
        if error.0 != 0 {
            return Err(registry_error(error));
        }
        if kind != REG_SZ || byte_len as usize % size_of::<u16>() != 0 {
            return Err("The StackDock Windows startup entry has an unsupported format.".into());
        }

        let mut bytes = vec![0; byte_len as usize];
        let error = unsafe {
            RegQueryValueExW(
                key.0,
                PCWSTR(name.as_ptr()),
                None,
                Some(&mut kind),
                Some(bytes.as_mut_ptr()),
                Some(&mut byte_len),
            )
        };
        if error.0 != 0 {
            return Err(registry_error(error));
        }
        let wide = bytes[..byte_len as usize]
            .chunks_exact(size_of::<u16>())
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .take_while(|&unit| unit != 0)
            .collect::<Vec<_>>();
        String::from_utf16(&wide)
            .map(Some)
            .map_err(|error| format!("The StackDock Windows startup path is invalid: {error}"))
    }

    fn native_executable() -> Result<Option<PathBuf>, String> {
        let path = match std::env::var_os("SD_NATIVE_EXE").filter(|p| !p.is_empty()) {
            Some(path) => PathBuf::from(path),
            None => {
                let sibling = std::env::current_exe().map_err(|e| e.to_string())?
                    .with_file_name("stack-dock-native.exe");
                if !sibling.is_file() { return Ok(None); }
                sibling
            }
        };
        if !path.is_absolute() { return Err("Expected an absolute startup path.".into()); }
        if !path.is_file()
            || !path.file_name().is_some_and(|name| {
                name.eq_ignore_ascii_case("stack-dock-native.exe")
            })
        {
            return Err("SD_NATIVE_EXE must refer to stack-dock-native.exe.".into());
        }
        Ok(Some(path))
    }

    fn startup_command(path: &Path) -> Result<String, String> {
        let raw = path.to_str().ok_or("Startup path contains invalid Unicode.")?;
        let path = if let Some(share) = raw.strip_prefix(r"\\?\UNC\") {
            format!(r"\\{share}")
        } else {
            raw.strip_prefix(r"\\?\").unwrap_or(raw).to_owned()
        };
        let command = format!("\"{path}\"");
        if command.encode_utf16().count() >= 260 {
            return Err("Move the portable folder to a shorter path to enable Windows startup.".into());
        }
        Ok(command)
    }

    fn register(path: &Path) -> Result<(), String> {
        let key = RegistryKey::create()?;
        let name = value_name();
        let command = startup_command(path)?;
        let data: Vec<u16> = command.encode_utf16().chain([0]).collect();
        let bytes = unsafe {
            std::slice::from_raw_parts(data.as_ptr().cast::<u8>(), data.len() * size_of::<u16>())
        };
        let error =
            unsafe { RegSetValueExW(key.0, PCWSTR(name.as_ptr()), None, REG_SZ, Some(bytes)) };
        if error.0 != 0 {
            return Err(registry_error(error));
        }
        Ok(())
    }

    fn unregister() -> Result<(), String> {
        let Some(key) = RegistryKey::open(KEY_SET_VALUE)? else {
            return Ok(());
        };
        let name = value_name();
        let error = unsafe { RegDeleteValueW(key.0, PCWSTR(name.as_ptr())) };
        if error.0 != 0 && error.0 != 2 {
            return Err(registry_error(error));
        }
        Ok(())
    }

    pub fn update(enabled: Option<bool>) -> Result<(bool, bool), String> {
        let executable = native_executable()?;
        if let Some(enabled) = enabled {
            if enabled {
                if cfg!(debug_assertions) {
                    return Err("Use a release build to enable Windows sign-in startup.".into());
                }
                let path = executable
                    .as_ref()
                    .ok_or("Start Settings from the native dock to enable sign-in startup.")?;
                register(path)?;
            } else {
                unregister()?;
            }
        }
        let registration = read_registration()?;
        let registered = match (registration, executable.as_deref()) {
            (Some(actual), Some(path)) => startup_command(path)
                .is_ok_and(|expected| actual.eq_ignore_ascii_case(&expected)),
            (Some(_), None) => true,
            _ => false,
        };
        Ok((registered, !cfg!(debug_assertions) && executable.is_some()))
    }
}

#[tauri::command]
fn stack_dock_startup(enabled: Option<bool>) -> Result<StartupStatus, String> {
    #[cfg(windows)]
    {
        let (enabled, can_enable) = windows_startup::update(enabled)?;
        Ok(StartupStatus { enabled, can_enable })
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        Ok(StartupStatus { enabled: false, can_enable: false })
    }
}

#[tauri::command]
fn native_menu_window(window: tauri::WebviewWindow, editor: bool, title:String, width:f64, height:f64) -> Result<(),String> {
    if std::env::var_os("SD_MENU_X").is_none(){return Err("No active shortcut menu".into());}
    let x=std::env::var("SD_MENU_X").ok().and_then(|s|s.parse::<i32>().ok()).unwrap_or(0);
    let y=std::env::var("SD_MENU_Y").ok().and_then(|s|s.parse::<i32>().ok()).unwrap_or(0);
    let monitors=window.available_monitors().map_err(|e|e.to_string())?;
    let m=monitors.iter().find(|m|{let p=m.position();let s=m.size();
        x>=p.x && y>=p.y && x<p.x+s.width as i32 && y<p.y+s.height as i32});
    let scale=m.map_or(1.,|m|m.scale_factor());
    let mut w=(width.clamp(100.,1100.)*scale).ceil() as u32;
    let mut h=(height.clamp(40.,850.)*scale).ceil() as u32;
    let(mut px,mut py)=(x,y);
    if let Some(m)=m {let a=m.work_area();w=w.min(a.size.width);h=h.min(a.size.height);
        px=x.clamp(a.position.x,a.position.x+(a.size.width-w) as i32);
        py=y.clamp(a.position.y,a.position.y+(a.size.height-h) as i32);}
    window.set_title(&title).map_err(|e|e.to_string())?;
    window.set_decorations(editor).map_err(|e|e.to_string())?;
    window.set_position(tauri::PhysicalPosition::new(px,py)).map_err(|e|e.to_string())?;
    window.set_size(tauri::PhysicalSize::new(w,h)).map_err(|e|e.to_string())
}
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeSearchSelection {
    folder_id: Option<String>,
    entry_id: String,
}

#[tauri::command]
fn finish_native_menu(
    app: tauri::AppHandle,
    action: u8,
    selection: Option<NativeSearchSelection>,
) -> Result<(), String> {
    if std::env::var_os("SD_MENU_X").is_none() {
        return Ok(());
    }
    let status = if let Some(selection) = selection {
        use std::io::Write;
        let mut output = std::io::stdout().lock();
        serde_json::to_writer(&mut output, &selection).map_err(|error| error.to_string())?;
        output.flush().map_err(|error| error.to_string())?;
        25
    } else {
        match action {
            1..=4 => 20 + i32::from(action),
            _ => 0,
        }
    };
    app.exit(status);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::Manager as _;

    let _settings_only_compat = std::env::args_os()
        .any(|arg| arg.as_os_str() == std::ffi::OsStr::new("--settings-only"))
        || std::env::var("STACKDOCK_SETTINGS_ONLY").as_deref() == Ok("1");

    let mut context = tauri::generate_context!();

    // Always run in settings-only mode for StackDock.exe
    for window in &mut context.config_mut().app.windows {
        window.create = false;
    }

    let builder = tauri::Builder::default();
    builder.plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let mut config = app
                .config()
                .app
                .windows
                .iter()
                .find(|window| window.label == "main")
                .cloned()
                .ok_or_else(|| std::io::Error::other("Missing main window configuration"))?;

            config.title = "StackDock Settings".into();
            config.width = 1000.0;
            config.height = 700.0;
            config.decorations = true;
            config.transparent = false;
            config.skip_taskbar = false;
            config.visible = false;

            let mut init = String::from("window.__STACKDOCK_SETTINGS_ONLY__ = true;");
            if std::env::var_os("SD_MENU_X").is_some() {
                use std::io::Read;
                let mut payload=String::new();
                std::io::stdin().take(16*1024*1024).read_to_string(&mut payload)?;
                let data:serde_json::Value=serde_json::from_str(&payload)?;
                init.push_str(&format!("window.__SD_NATIVE_MENU__={data};"));
            }
            let _settings_window = tauri::WebviewWindowBuilder::from_config(app, &config)?
                .initialization_script(&init)
                .build()?;
            #[cfg(windows)]
            if let (Ok(hwnd), Ok(path)) = (_settings_window.hwnd(), std::env::current_exe()) {
                use std::os::windows::ffi::OsStrExt;
                unsafe extern "C" {
                    fn configure_settings_taskbar(hwnd: *mut std::ffi::c_void, exe: *const u16) -> i32;
                }
                let path: Vec<_> = path.as_os_str().encode_wide().chain([0]).collect();
                unsafe { configure_settings_taskbar(hwnd.0.cast(), path.as_ptr()); }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_dock_bounds,
            set_dock_region,
            set_settings_window_icon,
            stack_dock_startup,
            launch_program,
            open_shortcut_path,
            open_shortcut_destination,
            native_shortcuts::pick_shortcut_path,
            native_shortcuts::read_shortcut_icon,
            native_shortcuts::list_shortcut_icons,
            native_shortcuts::inspect_shortcut_drops,
            native_menu_window, finish_native_menu,
            native_shortcuts::exit_stack_dock,
            native_shortcuts::stack_dock_layout_backup,
            layout_store::load_shared_layout,
            layout_store::save_shared_layout,
        ])
                .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if let Some(webview) = window.get_webview_window("main") {
                    if webview
                        .eval("void window.closeStackDockSettings?.()")
                        .is_ok()
                    {
                        api.prevent_close();
                    }
                }
            }
        })
        .build(context)
        .expect("error while building tauri application")
        .run(|_, _| {});
}