// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod native_shortcuts;
mod desktop_widget;
mod file_drop;

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

        if let Some([left, top, right, bottom]) = companion {
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
            // Keep the dock visible behind Settings without exposing the
            // transparent desktop between the two surfaces to hit testing.
            if let Some([left, top, right, bottom]) = companion {
                let dock = CreateRectRgn(left, top, right, bottom);
                if dock.is_null() {
                    DeleteObject(region);
                    return Err("Windows could not create the companion dock region".into());
                }
                const RGN_OR: i32 = 2;
                let status = CombineRgn(region, region, dock, RGN_OR);
                DeleteObject(dock);
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
        let _ = (window, x, y, width, height, radius, clip_top, companion);
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

#[tauri::command]
fn stack_dock_startup(app: tauri::AppHandle, enabled: Option<bool>) -> Result<StartupStatus, String> {
    #[cfg(windows)]
    {
        use tauri_plugin_autostart::ManagerExt;
        let launcher = app.autolaunch();
        let can_enable = !cfg!(debug_assertions);
        if let Some(enabled) = enabled {
            if enabled && !can_enable {
                return Err("Use an installed or release build to enable startup.".into());
            }
            let result = if enabled { launcher.enable() } else { launcher.disable() };
            result.map_err(|e| format!("Could not update Windows startup: {e}"))?;
        }
        let enabled = launcher.is_enabled().map_err(|e| e.to_string())?;
        Ok(StartupStatus { enabled, can_enable })
    }
    #[cfg(not(windows))]
    {
        let _ = (app, enabled);
        Ok(StartupStatus { enabled: false, can_enable: false })
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(windows)]
    let builder = builder.plugin(tauri_plugin_autostart::init(
        tauri_plugin_autostart::MacosLauncher::LaunchAgent,
        None,
    ));
    builder.plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            set_dock_bounds,
            set_dock_region,
            stack_dock_startup,
            launch_program,
            open_shortcut_path,
            open_shortcut_destination,
            native_shortcuts::pick_shortcut_path,
            native_shortcuts::read_shortcut_icon,
            native_shortcuts::list_shortcut_icons,
            native_shortcuts::inspect_shortcut_drops,
            native_shortcuts::exit_stack_dock,
            native_shortcuts::stack_dock_layout_backup,
            desktop_widget::dock_taskbar_rects,
            desktop_widget::dock_drag_button_held,
            file_drop::sync_dock_file_drop,
            desktop_widget::start_desktop_widget
        ])
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                file_drop::unregister_all();
            }
            native_shortcuts::keep_dock_open(window, event);
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                file_drop::unregister_all();
            }
        });
}
