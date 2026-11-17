use std::{ffi::c_void, ptr};

type Hwnd = *mut c_void;

#[derive(Clone, Copy, Default, serde::Serialize)]
#[repr(C)]
pub struct TaskbarRect { left: i32, top: i32, right: i32, bottom: i32 }

#[link(name = "user32")]
unsafe extern "system" {
    fn FindWindowExW(parent: Hwnd, after: Hwnd, class: *const u16, title: *const u16) -> Hwnd;
    fn GetWindowRect(hwnd: Hwnd, rect: *mut TaskbarRect) -> i32;
    fn IsWindowVisible(hwnd: Hwnd) -> i32;
    fn ShowWindow(hwnd: Hwnd, command: i32) -> i32;
    fn SetWindowPos(hwnd: Hwnd, insert_after: Hwnd, x: i32, y: i32, w: i32, h: i32, flags: u32) -> i32;
    fn GetAsyncKeyState(virtual_key: i32) -> i16;
}

#[tauri::command]
pub fn dock_drag_button_held(window: tauri::WebviewWindow, raise: bool) -> bool {
    // Explorer can take focus before the pointer reaches the drop target.
    // Only query the physical mouse buttons, and only while the grid loses focus.
    const VK_LBUTTON: i32 = 0x01;
    const VK_RBUTTON: i32 = 0x02;
    let held = unsafe { GetAsyncKeyState(VK_LBUTTON) < 0 || GetAsyncKeyState(VK_RBUTTON) < 0 };
    if held && raise {
        if let Ok(handle) = window.hwnd() {
            // Lift the clipped dock above Explorer without activating it or
            // entering the always-on-top band.
            const FLAGS: u32 = 0x0001 | 0x0002 | 0x0010;
            unsafe { SetWindowPos(handle.0 as Hwnd, ptr::null_mut(), 0, 0, 0, 0, FLAGS); }
        }
    }
    held
}

#[tauri::command]
pub fn dock_taskbar_rects() -> Vec<TaskbarRect> {
    let mut output = Vec::new();
    for class in ["Shell_TrayWnd", "Shell_SecondaryTrayWnd"] {
        let name: Vec<u16> = class.encode_utf16().chain([0]).collect();
        let mut after = ptr::null_mut();
        for _ in 0..32 {
            let hwnd = unsafe { FindWindowExW(ptr::null_mut(), after, name.as_ptr(), ptr::null()) };
            if hwnd.is_null() { break; }
            after = hwnd;
            let mut rect = TaskbarRect::default();
            let visible = unsafe { IsWindowVisible(hwnd) != 0 && GetWindowRect(hwnd, &mut rect) != 0 };
            if visible && rect.right > rect.left && rect.bottom > rect.top { output.push(rect); }
        }
    }
    output
}

#[tauri::command]
pub fn start_desktop_widget(window: tauri::WebviewWindow) -> Result<(), String> {
    // Called once, after JS has measured the work area and clipped the hidden host.
    // Let normal Windows stacking handle subsequent app switches.
    window.clone().run_on_main_thread(move || {
        match window.hwnd() {
            Ok(handle) => unsafe {
                const SW_SHOWNOACTIVATE: i32 = 4;
                const SWP_NOSIZE: u32 = 0x0001;
                const SWP_NOMOVE: u32 = 0x0002;
                const SWP_NOACTIVATE: u32 = 0x0010;
                let hwnd = handle.0 as Hwnd;
                ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                // Bring up the widget once without making it stay above other apps.
                SetWindowPos(hwnd, ptr::null_mut(), 0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            },
            Err(error) => eprintln!("[StackDock] Could not show window: {error}"),
        }
    }).map_err(|e| e.to_string())
}
