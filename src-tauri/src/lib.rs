// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod native_shortcuts;
mod desktop_widget;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

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
) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::ffi::c_void;

        #[link(name = "gdi32")]
        unsafe extern "system" {
            fn CreateRectRgn(left: i32, top: i32, right: i32, bottom: i32) -> *mut c_void;
            fn DeleteObject(object: *mut c_void) -> i32;
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
        if x < 0 || y < 0 || width <= 0 || height <= 0 {
            return Err("Invalid dock region".into());
        }

        let right = x.checked_add(width).ok_or("Dock region is too wide")?;
        let bottom = y.checked_add(height).ok_or("Dock region is too tall")?;

        unsafe {
            let region = CreateRectRgn(x, y, right, bottom);
            if region.is_null() {
                return Err("Windows could not create the dock region".into());
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
        let _ = (window, x, y, width, height);
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
                eprintln!("[Stack Dock] Started {} (PID {})", exe.display(), child.id());
                Ok(())
            }
            Err(error) => {
                eprintln!("[Stack Dock] Could not start {}: {}", exe.display(), error);
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            set_dock_bounds,
            set_dock_region,
            launch_program,
            open_shortcut_path,
            native_shortcuts::pick_shortcut_path,
            native_shortcuts::read_shortcut_icon,
            native_shortcuts::inspect_shortcut_drops,
            native_shortcuts::exit_stack_dock,
            native_shortcuts::stack_dock_layout_backup,
            desktop_widget::dock_taskbar_rects,
            desktop_widget::start_desktop_widget
        ])
        .on_window_event(native_shortcuts::keep_dock_open)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
