// A Win32 drop surface for Explorer file drops, independent of WebView2.
use std::{cell::{Cell, RefCell}, path::Path, ptr, rc::Rc, time::{Duration, Instant}};
use tauri::{Emitter, WebviewWindow};
use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, POINTL, WPARAM},
        Graphics::Gdi::ScreenToClient,
        System::{
            Com::{CoTaskMemFree, DVASPECT_CONTENT, FORMATETC, IDataObject, TYMED_HGLOBAL},
            LibraryLoader::GetModuleHandleW,
            Ole::{DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_NONE,
                IDropTarget, IDropTarget_Impl, OleInitialize, OleUninitialize, RegisterDragDrop,
                ReleaseStgMedium, RevokeDragDrop},
            SystemServices::MODIFIERKEYS_FLAGS,
        },
        UI::{
            Input::KeyboardAndMouse::GetAsyncKeyState,
            Shell::{DragQueryFileW, HDROP, IShellItemArray, SHCreateShellItemArrayFromDataObject, SIGDN_FILESYSPATH},
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, GetCursorPos, GetForegroundWindow,
                IsChild, IsWindowVisible, KillTimer, RegisterClassW, SetLayeredWindowAttributes,
                SetTimer, SetWindowPos, ShowWindow, WindowFromPoint, HWND_TOPMOST, LWA_ALPHA,
                SW_HIDE, SWP_NOACTIVATE, SWP_SHOWWINDOW, WNDCLASSW, WS_EX_LAYERED,
                WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
            },
        },
    },
    core::{PCWSTR, Ref, Result, implement, w},
};

#[derive(Clone, serde::Serialize)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Clone, serde::Serialize)]
struct DropEvent {
    #[serde(rename = "type")]
    kind: &'static str,
    position: Option<Position>,
    paths: Vec<String>,
}

fn file_format() -> FORMATETC {
    const CF_HDROP: u16 = 15;
    FORMATETC {
        cfFormat: CF_HDROP,
        ptd: ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    }
}

fn hdrop_paths(data: &IDataObject) -> Vec<String> {
    let Ok(mut medium) = (unsafe { data.GetData(&file_format()) }) else { return Vec::new(); };
    let mut paths = Vec::new();
    if medium.tymed == TYMED_HGLOBAL.0 as u32 {
        let handle = unsafe { medium.u.hGlobal };
        if !handle.0.is_null() {
            let drop = HDROP(handle.0);
            let count = unsafe { DragQueryFileW(drop, u32::MAX, None) };
            for i in 0..count.min(64) {
                let len = unsafe { DragQueryFileW(drop, i, None) };
                if len == 0 || len > 32768 { continue; }
                let mut buffer = vec![0; len as usize + 1];
                let written = unsafe { DragQueryFileW(drop, i, Some(&mut buffer)) };
                if written == 0 || written > len { continue; }
                let Ok(path) = String::from_utf16(&buffer[..written as usize]) else { continue; };
                if Path::new(&path).is_absolute() { paths.push(path); }
            }
        }
    }
    unsafe { ReleaseStgMedium(&mut medium); }
    paths
}

fn paths_from(data: &IDataObject) -> Vec<String> {
    let paths = hdrop_paths(data);
    if !paths.is_empty() { return paths; }
    // Desktop shell items (including some shortcuts) may offer a shell item
    // array instead of a conventional file drop list.
    let Ok(items) = (unsafe { SHCreateShellItemArrayFromDataObject::<_, IShellItemArray>(data) }) else {
        return Vec::new();
    };
    let count = unsafe { items.GetCount() }.unwrap_or(0);
    let mut paths = Vec::new();
    for i in 0..count.min(64) {
        let Ok(item) = (unsafe { items.GetItemAt(i) }) else { continue; };
        let Ok(value) = (unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }) else { continue; };
        let path = unsafe { value.to_string() };
        unsafe { CoTaskMemFree(Some(value.0.cast())); }
        if let Ok(path) = path {
            if Path::new(&path).is_absolute() { paths.push(path); }
        }
    }
    paths
}

// Never tell Explorer to MOVE a source file. StackDock only stores a reference.
fn safe_effect(allowed: DROPEFFECT) -> DROPEFFECT {
    if allowed.0 & DROPEFFECT_COPY.0 != 0 { DROPEFFECT_COPY }
    else if allowed.0 & DROPEFFECT_LINK.0 != 0 { DROPEFFECT_LINK }
    else { DROPEFFECT_NONE }
}

#[implement(IDropTarget)]
struct DockTarget {
    window: WebviewWindow,
    root: HWND,
    accepts_files: Cell<bool>,
    source_effect: Cell<DROPEFFECT>,
    last_over: Cell<Option<Instant>>,
    entered: Rc<Cell<bool>>,
}

impl DockTarget {
    fn emit_drop(&self, kind: &'static str, point: Option<&POINTL>, paths: Vec<String>) {
        let position = point.and_then(|p| {
            let mut origin = POINT { x: p.x, y: p.y };
            if !unsafe { ScreenToClient(self.root, &mut origin) }.as_bool() { return None; }
            Some(Position { x: origin.x, y: origin.y })
        });
        let _ = self.window.emit("stackdock-file-drop", DropEvent { kind, position, paths });
    }
}

#[allow(non_snake_case)]
impl IDropTarget_Impl for DockTarget_Impl {
    fn DragEnter(&self, data: Ref<IDataObject>, _: MODIFIERKEYS_FLAGS, point: &POINTL, effect: *mut DROPEFFECT) -> Result<()> {
        let Some(effect) = (unsafe { effect.as_mut() }) else { return Ok(()); };
        let accepted = data.as_ref().is_some_and(|source| {
            (unsafe { source.QueryGetData(&file_format()).is_ok() }) || !paths_from(source).is_empty()
        });
        self.last_over.set(None);
        let allowed = if accepted { safe_effect(*effect) } else { DROPEFFECT_NONE };
        let registered_drag = allowed.0 != 0;
        self.entered.set(registered_drag);
        self.accepts_files.set(registered_drag);
        self.source_effect.set(allowed);
        *effect = allowed;
        if effect.0 != 0 { self.emit_drop("enter", Some(point), Vec::new()); }
        Ok(())
    }
    fn DragOver(&self, _: MODIFIERKEYS_FLAGS, point: &POINTL, effect: *mut DROPEFFECT) -> Result<()> {
        let Some(effect) = (unsafe { effect.as_mut() }) else { return Ok(()); };
        *effect = if self.accepts_files.get() && effect.0 == 0 { self.source_effect.get() }
            else if self.accepts_files.get() { safe_effect(*effect) }
            else { DROPEFFECT_NONE };
        if effect.0 != 0 {
            let now = Instant::now();
            if self.last_over.get().is_none_or(|last| now.duration_since(last) >= Duration::from_millis(16)) {
                self.last_over.set(Some(now));
                self.emit_drop("over", Some(point), Vec::new());
            }
        }
        Ok(())
    }
    fn DragLeave(&self) -> Result<()> {
        self.entered.set(false);
        self.source_effect.set(DROPEFFECT_NONE);
        if self.accepts_files.replace(false) { self.emit_drop("leave", None, Vec::new()); }
        Ok(())
    }
    fn Drop(&self, data: Ref<IDataObject>, _: MODIFIERKEYS_FLAGS, point: &POINTL, effect: *mut DROPEFFECT) -> Result<()> {
        let Some(effect) = (unsafe { effect.as_mut() }) else { return Ok(()); };
        let previous = self.source_effect.replace(DROPEFFECT_NONE);
        let chosen = if effect.0 == 0 { previous } else { safe_effect(*effect) };
        *effect = DROPEFFECT_NONE;
        let accepted = self.accepts_files.replace(false);
        let paths = if accepted && chosen.0 != 0 { data.as_ref().map(paths_from).unwrap_or_default() } else { Vec::new() };
        self.entered.set(false);
        if paths.is_empty() {
            if accepted { self.emit_drop("leave", None, Vec::new()); }
        } else {
            *effect = chosen;
            self.emit_drop("drop", Some(point), paths);
        }
        Ok(())
    }
}

// An ordinary, non-activating Win32 window owns the OLE target. The webview
// is not a drop target dependency. The window is hidden except during an
// outside mouse drag and uses an alpha of 1, so hit testing remains enabled.
#[derive(Clone, Copy, PartialEq, Eq)]
struct DropZone { x: i32, y: i32, width: i32, height: i32 }

struct Surface {
    hwnd: HWND,
    root: HWND,
    entered: Rc<Cell<bool>>,
    zone: Option<DropZone>,
    shown: Option<DropZone>,
    timer: usize,
}

thread_local! {
    static SURFACE: RefCell<Option<Surface>> = const { RefCell::new(None) };
}

unsafe extern "system" fn surface_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, message, w, l) }
}

unsafe extern "system" fn surface_tick(_: HWND, _: u32, _: usize, _: u32) {
    try_update_surface();
}

fn set_surface_visibility(surface: &mut Surface) {
    let root = surface.root;
    let Some(zone) = surface.zone else { hide(surface); return; };
    // Mouse state is only used to arm the surface, not to read or synthesize a drag.
    let held = unsafe { GetAsyncKeyState(0x01) < 0 || GetAsyncKeyState(0x02) < 0 };
    let foreground = unsafe { GetForegroundWindow() };
    if !unsafe { IsWindowVisible(root) }.as_bool() ||
        (!surface.entered.get() && (!held || foreground == root || foreground == surface.hwnd)) {
        hide(surface);
        return;
    }
    let mut top_left = POINT { x: zone.x, y: zone.y };
    let mut cursor = POINT::default();
    if !unsafe { windows::Win32::Graphics::Gdi::ClientToScreen(root, &mut top_left) }.as_bool() ||
        unsafe { GetCursorPos(&mut cursor) }.is_err() {
        hide(surface);
        return;
    }
    // Arm on the held external mouse button even when the cursor is far
    // away. Waiting for proximity can send OLE into the WebView before the
    // registered target is visible. The surface remains confined to the
    // current dock/grid rectangle and the visibility checks below still apply.
    let cursor_inside = cursor.x >= top_left.x && cursor.y >= top_left.y &&
        cursor.x < top_left.x + zone.width && cursor.y < top_left.y + zone.height;
    let hit = |point| {
        let at = unsafe { WindowFromPoint(point) };
        at == root || at == surface.hwnd || (!at.0.is_null() && unsafe { IsChild(root, at) }.as_bool())
    };
    // A transparent pixel in the grid may not hit the host. Check several
    // points inside the panel so a clear background isn't a no-drop zone.
    let panel_visible = (cursor_inside && hit(cursor)) ||
        [(2, 2), (2, 1), (2, 3), (1, 2), (3, 2)].into_iter().any(|(x, y)| {
            hit(POINT { x: top_left.x + zone.width / 4 * x, y: top_left.y + zone.height / 4 * y })
        });
    if !surface.entered.get() && !panel_visible {
        hide(surface);
        return;
    }
    let positioned = DropZone { x: top_left.x, y: top_left.y, ..zone };
    if surface.shown != Some(positioned) &&
        unsafe {
            SetWindowPos(surface.hwnd, Some(HWND_TOPMOST), positioned.x, positioned.y,
                positioned.width, positioned.height, SWP_NOACTIVATE | SWP_SHOWWINDOW)
        }.is_ok() {
        surface.shown = Some(positioned);
    }
}

fn hide(surface: &mut Surface) {
    if surface.shown.take().is_some() { unsafe { let _ = ShowWindow(surface.hwnd, SW_HIDE); } }
}

fn try_update_surface() {
    SURFACE.with(|slot| {
        if let Ok(mut slot) = slot.try_borrow_mut() {
            if let Some(surface) = slot.as_mut() { set_surface_visibility(surface); }
        }
    });
}

fn create_surface(window: &WebviewWindow) -> std::result::Result<Surface, String> {
    let root = HWND(window.hwnd().map_err(|e| e.to_string())?.0 as *mut _);
    let module = unsafe { GetModuleHandleW(PCWSTR::null()) }.map_err(|e| e.to_string())?;
    let instance = HINSTANCE(module.0);
    let name = w!("StackDock.FileDrop.1");
    let class = WNDCLASSW { lpfnWndProc: Some(surface_proc), hInstance: instance, lpszClassName: name, ..Default::default() };
    // Registering a class with this name twice is harmless. CreateWindowExW
    // is the authoritative result when a window needs to be recreated.
    unsafe { let _ = RegisterClassW(&class); }
    unsafe { OleInitialize(None) }.map_err(|e| e.to_string())?;
    let hwnd = match unsafe {
        CreateWindowExW(WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
            name, w!(""), WS_POPUP, 0, 0, 1, 1, Some(root), None, Some(instance), None)
    } {
        Ok(hwnd) => hwnd,
        Err(error) => { unsafe { OleUninitialize(); } return Err(error.to_string()); }
    };
    let entered = Rc::new(Cell::new(false));
    let target: IDropTarget = DockTarget {
        window: window.clone(), root, accepts_files: Cell::new(false), source_effect: Cell::new(DROPEFFECT_NONE), last_over: Cell::new(None),
        entered: entered.clone(),
    }.into();
    if let Err(error) = unsafe {
        SetLayeredWindowAttributes(hwnd, windows::Win32::Foundation::COLORREF(0), 1, LWA_ALPHA)
            .and_then(|_| RegisterDragDrop(hwnd, &target))
    } {
        unsafe { let _ = DestroyWindow(hwnd); OleUninitialize(); }
        return Err(error.to_string());
    }
    let timer = unsafe { SetTimer(Some(hwnd), 1, 16, Some(surface_tick)) };
    if timer == 0 {
        unsafe { let _ = RevokeDragDrop(hwnd); let _ = DestroyWindow(hwnd); OleUninitialize(); }
        return Err("Windows could not start the file drop surface.".into());
    }
    Ok(Surface { hwnd, root, entered, zone: None, shown: None, timer })
}

#[tauri::command]
pub fn sync_dock_file_drop(window: WebviewWindow, x: i32, y: i32, width: i32, height: i32) -> std::result::Result<(), String> {
    if window.label() != "main" { return Err("The dock window is not available.".into()); }
    if x.unsigned_abs() > 131_072 || y.unsigned_abs() > 131_072 || width > 65_536 || height > 65_536 {
        return Err("The file drop area is invalid.".into());
    }
    let target = window.clone();
    window.run_on_main_thread(move || {
        SURFACE.with(|slot| {
            let mut slot = slot.borrow_mut();
            if width <= 0 || height <= 0 {
                if let Some(surface) = slot.as_mut() { surface.zone = None; hide(surface); }
                return;
            }
            if slot.is_none() {
                match create_surface(&target) {
                    Ok(surface) => *slot = Some(surface),
                    Err(error) => { eprintln!("[StackDock] File drop surface: {error}"); return; }
                }
            }
            if let Some(surface) = slot.as_mut() {
                surface.zone = Some(DropZone { x, y, width, height });
                set_surface_visibility(surface);
            }
        });
    }).map_err(|e| e.to_string())
}

pub fn unregister_all() {
    SURFACE.with(|slot| {
        if let Some(surface) = slot.borrow_mut().take() {
            unsafe {
                let _ = KillTimer(Some(surface.hwnd), surface.timer);
                let _ = RevokeDragDrop(surface.hwnd);
                let _ = DestroyWindow(surface.hwnd);
                OleUninitialize();
            }
        }
    });
}
