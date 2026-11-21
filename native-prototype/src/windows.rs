// Win32 FFI is isolated here. The model and renderer are testable without Windows.
use crate::{
    model::{Entry, Layout, Rect, Saved, bounded, ease, mix},
    paint::{Clock, Navigation, Assets, Canvas, Hit, View, ease_close, ease_open, frame},
};
use std::{
    cell::RefCell,
    ffi::c_void,
    io,
    mem::{size_of, zeroed},
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    ptr::{null, null_mut},
    slice,
    time::{Duration, Instant},
};

type Handle = *mut c_void;
type WndProc = unsafe extern "system" fn(Handle, u32, usize, isize) -> isize;

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}
#[repr(C)]
struct Size {
    cx: i32,
    cy: i32,
}
#[repr(C)]
struct WinRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}
#[repr(C)]
struct MonitorInfo {
    size: u32,
    monitor: WinRect,
    work: WinRect,
    flags: u32,
}
#[repr(C)]
struct Msg {
    hwnd: Handle,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    pt: Point,
    private: u32,
}
#[repr(C)]
struct WindowClass {
    size: u32,
    style: u32,
    proc: Option<WndProc>,
    class_extra: i32,
    win_extra: i32,
    instance: Handle,
    icon: Handle,
    cursor: Handle,
    background: Handle,
    menu: *const u16,
    name: *const u16,
    small_icon: Handle,
}
#[repr(C)]
struct BitmapInfo {
    header_size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bit_count: u16,
    compression: u32,
    image_size: u32,
    x_ppm: i32,
    y_ppm: i32,
    colors_used: u32,
    important_colors: u32,
    first_color: u32,
}
#[repr(C)]
struct Blend {
    operation: u8,
    flags: u8,
    opacity: u8,
    alpha_format: u8,
}
#[repr(C)]
struct MouseTrack {
    size: u32,
    flags: u32,
    hwnd: Handle,
    hover_time: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> Handle;
}
#[link(name = "user32")]
unsafe extern "system" {
    fn SetProcessDpiAwarenessContext(context: Handle) -> i32;
    fn GetDpiForWindow(hwnd: Handle) -> u32;
    fn MonitorFromPoint(point: Point, flags: u32) -> Handle;
    fn GetMonitorInfoW(monitor: Handle, info: *mut MonitorInfo) -> i32;
    fn RegisterClassExW(class: *const WindowClass) -> u16;
    fn CreateWindowExW(
        ex_style: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Handle,
        menu: Handle,
        instance: Handle,
        parameter: Handle,
    ) -> Handle;
    fn DestroyWindow(hwnd: Handle) -> i32;
    fn DefWindowProcW(hwnd: Handle, message: u32, wparam: usize, lparam: isize) -> isize;
    fn ShowWindow(hwnd: Handle, cmd: i32) -> i32;
    fn IsWindow(hwnd: Handle) -> i32;
    fn LoadCursorW(instance: Handle, name: *const u16) -> Handle;
    fn MessageBoxW(hwnd: Handle, text: *const u16, caption: *const u16, flags: u32) -> i32;
    fn GetCursorPos(point: *mut Point) -> i32;
    fn CreatePopupMenu() -> Handle;
    fn AppendMenuW(menu: Handle, flags: u32, id: usize, label: *const u16) -> i32;
    fn TrackPopupMenuEx(
        menu: Handle,
        flags: u32,
        x: i32,
        y: i32,
        hwnd: Handle,
        params: Handle,
    ) -> i32;
    fn DestroyMenu(menu: Handle) -> i32;
    fn GetMessageW(msg: *mut Msg, hwnd: Handle, min: u32, max: u32) -> i32;
    fn TranslateMessage(msg: *const Msg) -> i32;
    fn DispatchMessageW(msg: *const Msg) -> isize;
    fn PostMessageW(hwnd: Handle, message: u32, wparam: usize, lparam: isize) -> i32;
    fn PostQuitMessage(exit_code: i32);
    fn SetTimer(hwnd: Handle, id: usize, interval_ms: u32, callback: Handle) -> usize;
    fn KillTimer(hwnd: Handle, id: usize) -> i32;
    fn TrackMouseEvent(event: *mut MouseTrack) -> i32;
    fn GetWindowRect(hwnd: Handle, rect: *mut WinRect) -> i32;
    fn SystemParametersInfoW(action: u32, param: u32, value: Handle, flags: u32) -> i32;
    fn UpdateLayeredWindow(
        hwnd: Handle,
        screen: Handle,
        dst: *const Point,
        size: *const Size,
        source: Handle,
        origin: *const Point,
        key: u32,
        blend: *const Blend,
        flags: u32,
    ) -> i32;
}
#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateCompatibleDC(reference: Handle) -> Handle;
    fn CreateDIBSection(
        dc: Handle,
        info: *const c_void,
        usage: u32,
        pixels: *mut Handle,
        section: Handle,
        offset: u32,
    ) -> Handle;
    fn SelectObject(dc: Handle, object: Handle) -> Handle;
    fn DeleteObject(object: Handle) -> i32;
    fn DeleteDC(dc: Handle) -> i32;
}
#[link(name = "shell32")]
unsafe extern "system" {
    fn ShellExecuteW(
        hwnd: Handle,
        verb: *const u16,
        file: *const u16,
        args: *const u16,
        cwd: *const u16,
        show: i32,
    ) -> Handle;
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

#[path = "native_drop.rs"]
mod native_drop;
#[link(name="user32")]
unsafe extern "system" {fn SetCapture(hwnd:Handle)->Handle;fn ReleaseCapture()->i32;}

struct PopupMenu(Handle);

impl PopupMenu {
    fn new() -> Option<Self> {
        let handle = unsafe { CreatePopupMenu() };
        (!handle.is_null()).then_some(Self(handle))
    }

    fn item(&self, label: &str, id: usize, flags: u32) -> bool {
        unsafe { AppendMenuW(self.0, flags, id, wide(&label.replace('&', "&&")).as_ptr()) != 0 }
    }

    fn separator(&self) {
        unsafe { AppendMenuW(self.0, 0x0800, 0, null()); }
    }

    fn submenu(&self, label: &str, child: Self, enabled: bool) {
        if self.item(label, child.0 as usize, 0x10 | u32::from(!enabled)) {
            std::mem::forget(child);
        }
    }
}

impl Drop for PopupMenu {
    fn drop(&mut self) {
        unsafe { DestroyMenu(self.0); }
    }
}

fn shortcut_is_link(entry: &Entry) -> bool {
    if !entry.kind.is_empty() {
        return matches!(entry.kind.as_str(), "url" | "uri");
    }
    entry.target.split_once("://").is_some_and(|(scheme, _)| {
        scheme.len() >= 2 && scheme.as_bytes()[0].is_ascii_alphabetic()
            && scheme.bytes().all(|ch| ch.is_ascii_alphanumeric() || b"+.-".contains(&ch))
    })
}

fn context_menu(hwnd: Handle, keyboard: bool) {
    let mut pt = Point { x: 0, y: 0 };
    if keyboard {
        let rect = APP.with(|c| c.borrow().as_ref().and_then(App::focus_rect));
        let Some(r) = rect else { return };
        pt = Point { x: r.cx() as i32, y: r.cy() as i32 };
    } else if unsafe { GetCursorPos(&mut pt) } == 0 {
        return;
    }
    let built = APP.with(|cell| {
        let state = cell.try_borrow().ok()?;
        let app = state.as_ref()?;
        if app.motion.is_some() || app.indicator.is_some() { return None; }

        let mut ctx = app.context(&pt);
        let editable = !app.settings_open && app.settings.is_some();
        let menu = PopupMenu::new()?;
        let mut actions = Vec::new();
        let mut add = |parent: &PopupMenu, label: &str, action: &str, enabled: bool, checked: bool| {
            actions.push(action.to_string());
            parent.item(label, actions.len(), u32::from(!enabled) | if checked { 8 } else { 0 });
        };
        if let Some(folder) = app.saved.folder_at(&app.path) {
            let pointed = ctx["entry"].as_str().and_then(|id| folder.slots.iter().find(|e| e.id == id));
            let row = pointed.map_or(ctx["r"].as_u64().unwrap_or(0) as usize, |e| e.r);
            let col = pointed.map_or(ctx["c"].as_u64().unwrap_or(0) as usize, |e| e.c);
            ctx["r"] = row.into();
            ctx["c"] = col.into();
            let entry = folder.slots.iter().find(|e| e.r == row && e.c == col);
            let title = entry.map_or_else(|| format!("{} · Row {}, column {}", folder.name, row + 1, col + 1), |e| e.name.clone());
            menu.item(&title, 0, 1);
            match entry {
                Some(e) if e.folder() => {
                    add(&menu, "Open subfolder", "Open subfolder", true, false);
                    add(&menu, "Folder settings…", "Folder settings…", editable, false);
                    add(&menu, "Remove subfolder…", "Remove subfolder…", editable, false);
                }
                Some(e) => {
                    add(&menu, "Shortcut properties…", "Shortcut properties…", editable, false);
                    let label = if shortcut_is_link(e) { "Open Link" } else { "Open Destination" };
                    add(&menu, label, label, shortcut_is_link(e) || editable, false);
                    add(&menu, "Remove shortcut", "Remove shortcut", editable, false);
                }
                None => {
                    add(&menu, "Add shortcut here…", "Add shortcut here…", editable, false);
                    add(&menu, "New subfolder…", "New subfolder…", editable && app.path.len() < 12, false);
                }
            }
            menu.separator();
            add(&menu, "Insert row below", "Insert row below", editable && folder.rows < 80, false);
            add(&menu, "Insert column right", "Insert column right", editable && folder.cols < 12, false);
            if folder.rows > 1 && !folder.slots.iter().any(|e| e.r == row) {
                add(&menu, "Remove empty row", "Remove empty row", editable, false);
            }
            if folder.cols > 1 && !folder.slots.iter().any(|e| e.c == col) {
                add(&menu, "Remove empty column", "Remove empty column", editable, false);
            }
            menu.separator();
            add(&menu, "Grid Stack settings…", "Grid Stack settings…", editable, false);
            if app.path.len() > 1 {
                add(&menu, "Back to parent", "Back to parent", true, false);
            }
            add(&menu, "Close grid", "Close grid", true, false);
        } else {
            let entries = app.saved.ordered();
            let index = ctx["entry"].as_str().and_then(|id| entries.iter().position(|e| e.id == id));
            let entry = index.map(|i| entries[i]);
            let shortcut = entry.is_some_and(|e| !e.folder());
            menu.item(entry.map_or("Dock", |e| e.name.as_str()), 0, 1);
            if let Some(e) = entry.filter(|e| !e.folder()) {
                add(&menu, "Shortcut properties…", "Shortcut properties…", editable, false);
                let label = if shortcut_is_link(e) { "Open Link" } else { "Open Destination" };
                add(&menu, label, label, shortcut_is_link(e) || editable, false);
                menu.separator();
            }
            if entry.is_some_and(Entry::folder) {
                add(&menu, "Folder settings…", "Folder settings…", editable, false);
            } else {
                add(&menu, "Settings…", "Settings…", editable, false);
                add(&menu, "Dock settings…", "Dock settings…", editable, false);
            }
            if let Some(i) = index {
                let kind = if shortcut { "shortcut" } else { "folder" };
                let earlier = format!("Move {kind} earlier");
                let later = format!("Move {kind} later");
                add(&menu, &earlier, &earlier, editable && i > 0, false);
                add(&menu, &later, &later, editable && i + 1 < entries.len(), false);
                menu.separator();
            }
            let positions = PopupMenu::new()?;
            positions.item("Screen position", 0, 1);
            for (edge, label) in [("left", "Left"), ("right", "Right"), ("top", "Top"), ("bottom", "Bottom")] {
                add(&positions, &format!("{label} Screen Edge"), &format!("position/{edge}/{}", app.saved.align), editable, app.saved.edge == edge);
            }
            positions.separator();
            let alignments = if matches!(app.saved.edge.as_str(), "left" | "right") {
                ["Top", "Middle", "Bottom"]
            } else { ["Left", "Center", "Right"] };
            for (align, label) in ["start", "center", "end"].into_iter().zip(alignments) {
                add(&positions, &format!("Align {label}"), &format!("position/{}/{align}", app.saved.edge), editable, app.saved.align == align);
            }
            menu.submenu("Screen Position", positions, editable);
            add(&menu, "New folder…", "New folder…", editable, false);
            add(&menu, "Add shortcut to dock…", "Add shortcut to dock…", editable && app.saved.dock_shortcuts.len() < 160, false);
            if shortcut || (entry.is_some() && app.saved.folders.len() > 1) {
                menu.separator();
                let label = if shortcut { "Remove shortcut" } else { "Remove folder…" };
                add(&menu, label, label, editable, false);
            }
            menu.separator();
            add(&menu, "Find shortcut…	Ctrl+F", "Find shortcut…", editable, false);
            let stackdock = PopupMenu::new()?;
            // stackdock.item("StackDock", 0, 1);
            // add(&stackdock, "Back", "Back", true, false);
            // stackdock.separator();
            add(&stackdock, "Export layout…", "Export layout…", editable, false);
            add(&stackdock, "Import layout…", "Import layout…", editable, false);
            stackdock.separator();
            add(&stackdock, "Exit", "Exit", true, false);
            menu.submenu("StackDock", stackdock, true);
        }
        Some((menu, actions, ctx))
    });
    let Some((menu, actions, mut ctx)) = built else { return };
    let action = loop {
        let id = unsafe { TrackPopupMenuEx(menu.0, 0x0102, pt.x, pt.y, hwnd, null_mut()) };
        match id.checked_sub(1).and_then(|i| actions.get(i as usize)) {
            Some(action) if action == "Back" => continue,
            Some(action) => break action,
            None => return,
        }
    };
    if action == "Exit" {
        unsafe { DestroyWindow(hwnd); }
        return;
    }
    let link_target = |target: Option<Target>| {
        target.map(|mut target| {
            if action == "Open Link" && target.kind.is_empty() {
                target.kind = "uri".into();
            }
            target
        })
    };
    let result=APP.with(|cell|->Result<Option<Target>,String>{
        let mut st=cell.borrow_mut();let Some(a)=st.as_mut()else{return Ok(None)};
        if matches!(action.as_str(), "Open Link" | "Open subfolder") {
            if let Some(folder) = a.saved.folder_at(&a.path) {
                let row = ctx["r"].as_u64().unwrap_or(0) as usize;
                let col = ctx["c"].as_u64().unwrap_or(0) as usize;
                if let Some(i) = folder.slots.iter().position(|e| e.r == row && e.c == col) {
                    return a.click(Hit::Grid(i)).map(link_target).map_err(|e| e.to_string());
                }
            } else if let Some(i) = a.saved.ordered().iter().position(|e| Some(e.id.as_str()) == ctx["entry"].as_str()) {
                return a.click(Hit::Dock(i)).map(link_target).map_err(|e| e.to_string());
            }
        }
        else if action=="Back to parent"{a.back().map_err(|e|e.to_string())?;}
        else if action=="Close grid"{a.go(0.).map_err(|e|e.to_string())?;}
        else{ctx["action"]=action.clone().into();a.command(ctx)?;}
        Ok(None)
    });
    match result{Ok(Some(target))=>{if let Err(err)=launch(hwnd,target){message(hwnd,&err.to_string());}},Err(err)=>message(hwnd,&err),_=>{}}
}
const WM_DESTROY: u32 = 0x0002;
const WM_ACTIVATE: u32 = 0x0006;
const WM_SETTINGCHANGE: u32 = 0x001A;
const WM_DISPLAYCHANGE: u32 = 0x007E;
const WM_KEYDOWN: u32 = 0x0100;
const WM_CONTEXTMENU: u32 = 0x007B;
const WM_TIMER: u32 = 0x0113;
const WM_MOUSEMOVE: u32 = 0x0200;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_MOUSELEAVE: u32 = 0x02A3;
const WM_DPICHANGED: u32 = 0x02E0;
const WM_SETTINGS_CLOSED: u32 = 0x8001; // WM_APP + 1
const ANIMATION_TIMER: usize = 1;
const HOVER_TIMER: usize = 2;
const SETTINGS_TIMER: usize = 3;

struct HoverTransition {
    hit: Hit,
    from: f32,
    to: f32,
    start: Instant,
    duration_ms: f32,
    curve: u8,
}

impl HoverTransition {
    fn sample(&self, now: Instant) -> (f32, bool) {
        let elapsed_ms = now.duration_since(self.start).as_secs_f32() * 1000.;
        if self.duration_ms <= 0. || elapsed_ms >= self.duration_ms {
            return (self.to, true);
        }

        let t = elapsed_ms / self.duration_ms;
        let progress = match self.curve {
            0 => t,
            2 => ease(t, 0.42, 0., 0.58, 1.),
            3 => ease(t, 0.2, 1.8, 0.35, 1.),
            _ => ease(t, 0., 0., 0.58, 1.),
        };
        (mix(self.from, self.to, progress), false)
    }
}

// Read/write access to the bitmap is exclusively from this Win32 UI thread.
struct Surface {
    dc: Handle,
    bitmap: Handle,
    previous: Handle,
    bits: *mut u32,
    width: usize,
    height: usize,
}

impl Surface {
    fn new(width: usize, height: usize) -> io::Result<Self> {
        let dc = unsafe { CreateCompatibleDC(null_mut()) };
        if dc.is_null() {
            return Err(io::Error::last_os_error());
        }
        let info = BitmapInfo {
            header_size: 40,
            width: width as i32,
            height: -(height as i32),
            planes: 1,
            bit_count: 32,
            compression: 0,
            image_size: 0,
            x_ppm: 0,
            y_ppm: 0,
            colors_used: 0,
            important_colors: 0,
            first_color: 0,
        };
        let mut bits = null_mut();
        let bitmap = unsafe {
            CreateDIBSection(
                dc,
                (&info as *const BitmapInfo).cast(),
                0,
                &mut bits,
                null_mut(),
                0,
            )
        };
        if bitmap.is_null() || bits.is_null() {
            let error = io::Error::last_os_error();
            unsafe {
                if !bitmap.is_null() {
                    DeleteObject(bitmap);
                }
                DeleteDC(dc);
            }
            return Err(error);
        }
        let previous = unsafe { SelectObject(dc, bitmap) };
        if previous.is_null() || previous as isize == -1 {
            let error = io::Error::last_os_error();
            unsafe {
                DeleteObject(bitmap);
                DeleteDC(dc);
            }
            return Err(error);
        }
        Ok(Self {
            dc,
            bitmap,
            previous,
            bits: bits.cast(),
            width,
            height,
        })
    }

    fn present(&self, hwnd: Handle, bounds: Rect) -> io::Result<()> {
        let dst = Point {
            x: bounds.x as i32,
            y: bounds.y as i32,
        };
        let size = Size {
            cx: self.width as i32,
            cy: self.height as i32,
        };
        let origin = Point { x: 0, y: 0 };
        let blend = Blend {
            operation: 0,
            flags: 0,
            opacity: 255,
            alpha_format: 1,
        };
        if unsafe {
            UpdateLayeredWindow(
                hwnd,
                null_mut(),
                &dst,
                &size,
                self.dc,
                &origin,
                0,
                &blend,
                2,
            )
        } == 0
        {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}

struct StripPreview{
    original:Layout,edge:&'static str,x:i32,y:i32,row:usize,col:usize,
}
struct Indicator{
    sprite:Option<crate::paint::TileSprite>,timer_delay:u32,
    item:Entry, source:Rect, from:Rect, card:Rect, previous:Vec<u32>, source_skin:crate::model::Skin,
    start:Instant, travel:f32,hold:f32,return_time:f32
}
struct Motion {
    start: Instant,
    from: f32,
    to: f32,
    duration: Duration,
    icon_from: f32,
}

fn motion_enabled() -> bool {
    let mut enabled: i32 = 1;
    unsafe {
        SystemParametersInfoW(0x1042, 0, (&mut enabled as *mut i32).cast(), 0);
    }
    enabled != 0
}

fn coord(value: isize, shift: u32) -> i32 {
    ((value >> shift) as u16 as i16) as i32
}

#[derive(Clone)]
struct Target {
    artwork:Entry,
    kind: String,
    target: String,
    args: String,
    cwd: String,
}
impl From<&Entry> for Target {
    fn from(e: &Entry) -> Self {
        Self {
            artwork:e.clone(),
            kind: e.kind.clone(),
            target: e.target.clone(),
            args: e.args.clone(),
            cwd: e.cwd.clone(),
        }
    }
}
fn launch(hwnd:Handle,entry:Target)->Result<(),String>{
    launch_target(hwnd,&entry)?;
    APP.with(|cell|{
        let mut state=cell.borrow_mut();let Some(app)=state.as_mut()else{return Ok(())};
        app.start_indicator(entry.artwork).map_err(|e|e.to_string())
    })
}
fn launch_target(hwnd: Handle, entry: &Target) -> Result<(), String> {
    let trim = |x: &str| x.trim().trim_matches('"').to_string();
    let target = trim(&entry.target);
    if entry.kind == "program"
        || entry.kind.is_empty() && target.to_ascii_lowercase().ends_with(".exe")
    {
        let path = Path::new(&target);
        if !path.is_absolute()
            || !path.is_file()
            || !path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
        {
            return Err("Program shortcuts need an existing absolute EXE path.".into());
        }
        let mut command = Command::new(path);
        if !entry.cwd.trim().is_empty() {
            let cwd = PathBuf::from(trim(&entry.cwd));
            if !cwd.is_absolute() || !cwd.is_dir() {
                return Err("The Start in folder does not exist.".into());
            }
            command.current_dir(cwd);
        } else if let Some(parent) = path.parent() {
            command.current_dir(parent);
        }
        if !entry.args.trim().is_empty() {
            command.raw_arg(entry.args.trim());
        }
        command.spawn().map(|_| ()).map_err(|e| e.to_string())
    } else {
        if !["url", "uri"].contains(&entry.kind.as_str()) && !Path::new(&target).is_absolute() {
            return Err("Local shortcuts need an absolute Windows path.".into());
        }
        if ["url", "uri"].contains(&entry.kind.as_str()) {
            let scheme = target.split_once(':').map_or("", |(s, _)| s);
            if !scheme.chars().enumerate().all(|(i, c)| {
                if i == 0 {
                    c.is_ascii_alphabetic()
                } else {
                    c.is_ascii_alphanumeric() || ['+', '-', '.'].contains(&c)
                }
            }) || scheme.len() < 2
                || ["file", "javascript", "data", "vbscript"]
                    .contains(&scheme.to_ascii_lowercase().as_str())
            {
                return Err("The URL shortcut has an invalid or unsupported scheme.".into());
            }
        }
        let target = wide(&target);
        let code =
            unsafe { ShellExecuteW(hwnd, null(), target.as_ptr(), null(), null(), 1) } as isize;
        if code <= 32 {
            Err(format!(
                "Windows could not open this shortcut (ShellExecute status {code})."
            ))
        } else {
            Ok(())
        }
    }
}
fn message(hwnd: Handle, text: &str) {
    let text = wide(text);
    let title = wide("StackDock native preview");
    unsafe {
        MessageBoxW(hwnd, text.as_ptr(), title.as_ptr(), 0x30);
    }
}

pub struct SettingsLink {
    executable: PathBuf,
    pub layout_path: PathBuf,
}

impl SettingsLink {
    pub fn new(executable: PathBuf) -> Result<Self, String> {
        let executable = executable
            .canonicalize()
            .map_err(|error| format!("Could not find the Settings executable: {error}"))?;
        if !executable.is_file()
            || !executable
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            return Err("Point --settings-exe at the Phase2 StackDock.exe.".into());
        }

        let base = std::env::var_os("LOCALAPPDATA")
            .ok_or("Windows did not provide the LOCALAPPDATA folder.")?;
        let layout_path = PathBuf::from(base)
            .join("com.stackdock.launcher")
            .join("layout.json");
        Ok(Self {
            executable,
            layout_path,
        })
    }
}

struct App {
    hwnd: Handle,
    saved: Saved,
    assets: Assets,
    layout: Layout,
    path: Vec<usize>,
    dock_index: Option<usize>,
    scroll: (f32, f32),
    dock_scroll: f32,
    progress: f32,
    icon_progress: f32,
    surface: Option<Surface>,
    motion: Option<Motion>,
    navigation: Option<Navigation>,
    clock: Clock,
    hover: Option<Hit>,
    hover_transitions: Vec<HoverTransition>,
    hover_levels: Vec<(Hit, f32)>,
    tracking: bool,
    settings: Option<SettingsLink>,
    indicator:Option<Indicator>,
    drag_base:Vec<u32>,
    ghost:Vec<u32>,
    ghost_size:usize,
    press:Option<(Hit,i32,i32)>,
    dragging:bool,
    drag_at:Option<(i32,i32)>,
    strip:Option<StripPreview>,
    tip:Option<crate::paint::HoverTip>,
    tip_fade:Instant,
    tip_leaving:bool,
    dwell:Option<(&'static str,Instant)>,
    key_focus: Option<Hit>,
    menu_hit: Option<Hit>,
    search_selection: Option<std::sync::mpsc::Receiver<SearchSelection>>,
    settings_open: bool,
    settings_stamp: Option<(std::time::SystemTime, u64)>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchSelection {
    folder_id: Option<String>,
    entry_id: String,
}

impl SearchSelection {
    fn resolve(&self, saved: &Saved) -> Option<Target> {
        fn find_folder<'a>(entries: &'a [Entry], id: &str) -> Option<&'a Entry> {
            entries.iter().filter(|entry| entry.folder()).find_map(|entry| {
                if entry.id == id {
                    Some(entry)
                } else {
                    find_folder(&entry.slots, id)
                }
            })
        }

        let entries = match self.folder_id.as_deref() {
            Some(id) => &find_folder(&saved.folders, id)?.slots,
            None => &saved.dock_shortcuts,
        };
        entries.iter()
            .find(|entry| !entry.folder() && entry.id == self.entry_id)
            .map(Target::from)
    }
}

impl App {
    fn new(hwnd: Handle, saved: Saved, settings: Option<SettingsLink>) -> io::Result<Self> {
        let (work, s) = desktop_rect(hwnd)?;
        let layout = Layout::new(&saved, work, s, None, (0., 0.), 0.);
        let fonts = std::env::var_os("WINDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("C:\\Windows"))
            .join("Fonts");
        Ok(Self {
            hwnd,
            saved,
            assets: Assets::new(&fonts),
            layout,
            path: vec![],
            dock_index: None,
            scroll: (0., 0.),
            dock_scroll: 0.,
            progress: 0.,
            icon_progress: 0.,
            surface: None,
            motion: None,
            navigation: None,
            clock: Clock { t:0.,opening:true },
            hover: None,
            hover_transitions: Vec::new(),
            hover_levels: Vec::new(),
            tracking: false,
            settings,
            indicator:None,drag_base:Vec::new(),ghost:Vec::new(),ghost_size:0,
            press:None,dragging:false,drag_at:None,dwell:None,strip:None,tip:None,tip_fade:Instant::now(),tip_leaving:false,
            key_focus: None,
            menu_hit: None,
            search_selection: None,
            settings_open: false,
            settings_stamp: None,
        })
    }

    fn clear_drag(&mut self){
        if let Some(preview)=self.strip.take(){self.scroll=preview.original.scroll;self.layout=preview.original;}
        self.drag_base.clear();self.ghost.clear();self.press=None;self.dragging=false;self.drag_at=None;self.dwell=None;unsafe{KillTimer(self.hwnd,4);}}
    fn drag_tick(&mut self)->io::Result<()>{
        let Some((x,y))=self.drag_at else{return Ok(())};let grid=!self.path.is_empty();
        if self.strip.is_some(){return Ok(());}
        let rect=if grid{self.layout.grid_clip}else{self.layout.dock};let scale=self.layout.s;
        let mut edges=[("left",x as f32-rect.x),("right",rect.right()-x as f32),("top",y as f32-rect.y),("bottom",rect.bottom()-y as f32)];
        let outside=edges.iter().any(|(_,d)|*d < -8.*scale);
        edges.sort_by(|a,b|a.1.abs().total_cmp(&b.1.abs()));let (edge,d)=edges[0];
        if outside || d.abs()>18.*scale{if self.dwell.take().is_some(){return self.draw();}return Ok(());}
        if grid && self.dragging {
            let f=self.saved.folder_at(&self.path).expect("active grid");
            let horizontal=matches!(edge,"left"|"right");
            if (horizontal&&f.cols<12)||(!horizontal&&f.rows<80){
                if self.dwell.is_none_or(|v|v.0!=edge){self.dwell=Some((edge,Instant::now()));}
                if self.dwell.is_some_and(|v|v.1.elapsed()>=Duration::from_secs(1)) && rect.contains(x as f32,y as f32){
                    let before=self.context(&Point{x,y});
                    let mut row=before["r"].as_u64().unwrap_or(0) as usize;
                    let mut col=before["c"].as_u64().unwrap_or(0) as usize;
                    let mut draft=f.clone();
                    match edge{
                        "left"=>{draft.cols+=1;col=0;for e in &mut draft.slots{e.c+=1;}},
                        "right"=>{col=draft.cols;draft.cols+=1;},
                        "top"=>{draft.rows+=1;row=0;for e in &mut draft.slots{e.r+=1;}},
                        _=>{row=draft.rows;draft.rows+=1;}
                    }
                    let mut scroll=self.scroll;
                    match edge{"left"=>scroll.0=0.,"top"=>scroll.1=0.,"right"=>scroll.0=f32::MAX,_=>scroll.1=f32::MAX}
                    let (work,dpi)=desktop_rect(self.hwnd)?;
                    let next=Layout::new(&self.saved,work,dpi,Some(&draft),scroll,self.dock_scroll);
                    self.scroll=next.scroll;
                    self.strip=Some(StripPreview{original:std::mem::replace(&mut self.layout,next),edge,x,y,row,col});
                    self.dwell=None;self.drag_base.clear();return self.draw();
                }
            }else{self.dwell=None;}
        }
        let prev=self.scroll;let old_dock=self.dock_scroll;
        if grid{match edge{"left"=>self.scroll.0-=12.*scale,"right"=>self.scroll.0+=12.*scale,"top"=>self.scroll.1-=12.*scale,_=>self.scroll.1+=12.*scale}}
        else{self.dock_scroll+=if matches!(edge,"left"|"top"){-12.*scale}else{12.*scale};}
        self.relayout()?;
        if self.scroll!=prev||self.dock_scroll!=old_dock{self.drag_base.clear();}

        self.draw()
    }
    fn context(&self,pos:&Point)->serde_json::Value {
        let b=self.bounds();
        let hit=self.hit(pos.x-b.x as i32,pos.y-b.y as i32);
        let mut ctx=serde_json::json!({"x":pos.x,"y":pos.y});
        let g=&self.layout;
        if let Some(f)=self.saved.folder_at(&self.path) {
            ctx["folder"]=f.id.clone().into();
            let s=g.gs;
            let cw=if s.labels{s.icon.max(s.label_w)}else{s.icon};
            let ch=s.icon+s.effects_height()+if s.labels{s.font*2.36+4.*g.s}else{0.};
            let col=((pos.x as f32-g.grid_clip.x+g.scroll.0+s.gx/2.)/(cw+s.gx)).floor() as usize;
            let row=((pos.y as f32-g.grid_clip.y+g.scroll.1+s.gy/2.)/(ch+s.gy)).floor() as usize;
            ctx["r"]=row.min(f.rows.saturating_sub(1)).into();ctx["c"]=col.min(f.cols.saturating_sub(1)).into();
            ctx["valid"]=g.grid_clip.contains(pos.x as f32,pos.y as f32).into();
            if let Some(preview)=&self.strip{
                let horizontal=matches!(preview.edge,"left"|"right");
                let near=((pos.x-preview.x) as f32).hypot((pos.y-preview.y) as f32)<10.*g.s;
                ctx["r"]=(if near{preview.row}else{row.min(f.rows-1+usize::from(!horizontal))}).into();
                ctx["c"]=(if near{preview.col}else{col.min(f.cols-1+usize::from(horizontal))}).into();
                ctx["valid"]=(near||g.grid_clip.contains(pos.x as f32,pos.y as f32)).into();
                ctx["grow"]=preview.edge.into();
            }
        } else {
            let vertical=self.saved.orientation=="vertical" || (self.saved.orientation!="horizontal" && matches!(self.saved.edge.as_str(),"left"|"right"));
            let slot=g.dock_tiles.iter().find(|t|if vertical{(pos.y as f32)<t.box_.cy()}else{(pos.x as f32)<t.box_.cx()}).map(|t|t.item).unwrap_or(self.saved.ordered().len());
            ctx["slot"]=slot.into();ctx["valid"]=g.dock.contains(pos.x as f32,pos.y as f32).into();
        }
        if let Some(e)=match hit{Some(Hit::Grid(i))=>self.saved.folder_at(&self.path).and_then(|f|f.slots.get(i)),Some(Hit::Dock(i))=>self.saved.ordered().get(i).copied(),_=>None}{
            ctx["entry"]=e.id.clone().into();ctx["isFolder"]=e.folder().into();ctx["link"]=matches!(e.kind.as_str(),"url"|"uri").into();
        }
        ctx
    }
    fn layout_path(&self) -> io::Result<&Path> {
        self.settings.as_ref()
            .map(|settings| settings.layout_path.as_path())
            .ok_or_else(|| io::Error::other("Missing layout path"))
    }

    fn read_layout(&self) -> io::Result<(Vec<u8>, serde_json::Value)> {
        let original = std::fs::read(self.layout_path()?)?;
        let data = serde_json::from_slice(&original)?;
        Ok((original, data))
    }

    fn move_dock(&mut self, source: &str, slot: usize) -> io::Result<()> {
        let mut ids: Vec<_> = self.saved.ordered().into_iter()
            .map(|entry| entry.id.clone()).collect();
        let from = ids.iter().position(|id| id == source)
            .ok_or_else(|| io::Error::other("Dock entry moved or was removed"))?;
        let to = slot.min(ids.len()).saturating_sub(usize::from(from < slot));
        if from == to {
            return Ok(());
        }

        let (original, mut data) = self.read_layout()?;
        let on_disk = Saved::from_json(&original).map_err(io::Error::other)?;
        if on_disk.ordered().into_iter().map(|entry| &entry.id).ne(ids.iter()) {
            return Err(io::Error::other("Dock order changed during the move"));
        }

        let id = ids.remove(from);
        ids.insert(to, id);
        let positions: std::collections::HashMap<_, _> = ids.iter()
            .enumerate().map(|(index, id)| (id.as_str(), index)).collect();
        let layout = &mut data["layout"];
        for key in ["folders", "dockShortcuts"] {
            if let Some(entries) = layout[key].as_array_mut() {
                entries.sort_by_key(|entry| {
                    entry["id"].as_str().and_then(|id| positions.get(id).copied())
                        .unwrap_or(usize::MAX)
                });
            }
        }
        layout["dockOrder"] = serde_json::to_value(&ids)?;
        self.save_layout(&original, &data)
    }

    fn move_grid(&mut self,ctx:&serde_json::Value)->io::Result<()>{
        let (original, mut data) = self.read_layout()?;
        let (&root,sub)=self.path.split_first().ok_or_else(||io::Error::other("No open folder"))?;
        let mut folder=data["layout"]["folders"].get_mut(root).ok_or_else(||io::Error::other("Root folder changed"))?;
        for &idx in sub{folder=folder["slots"].get_mut(idx).ok_or_else(||io::Error::other("Subfolder changed"))?;}
        if folder["id"]!=ctx["folder"]{return Err(io::Error::other("The open folder changed during this drag"));}
        let r=ctx["r"].as_u64().unwrap_or(0);let c=ctx["c"].as_u64().unwrap_or(0);
        for (axis,dim,early,late,max) in [("r","rows","top","bottom",80),("c","cols","left","right",12)]{
            let n=folder[dim].as_u64().unwrap_or(1);let edge=ctx["grow"].as_str();
            if n<max && (edge==Some(early)||edge==Some(late)){
                let offset=edge==Some(early);
                if offset{if let Some(items)=folder["slots"].as_array_mut(){for item in items{item[axis]=(item[axis].as_u64().unwrap_or(0)+1).into();}}}
                folder[dim]=(n+1).into();
            }
        }
        let items=folder["slots"].as_array_mut().ok_or_else(||io::Error::other("Missing grid slots"))?;
        let src=items.iter().position(|e|e["id"]==ctx["source"]).ok_or_else(||io::Error::other("Shortcut moved or removed"))?;
        if let Some(dst)=items.iter().position(|e|e["r"]==r && e["c"]==c){
            if dst==src && ctx["grow"].is_null(){return Ok(());}
            let old_r=items[src]["r"].clone();let old_c=items[src]["c"].clone();
            items[dst]["r"]=old_r;items[dst]["c"]=old_c;
        }
        items[src]["r"]=r.into();items[src]["c"]=c.into();
        self.save_layout(&original, &data)
    }

    fn save_layout(&mut self, original: &[u8], data: &serde_json::Value) -> io::Result<()> {
        use std::io::Write;

        let bytes = serde_json::to_vec(data)?;
        let saved = Saved::from_json(&bytes).map_err(io::Error::other)?;
        let (work, dpi) = desktop_rect(self.hwnd)?;
        let layout = Layout::new(
            &saved, work, dpi, saved.folder_at(&self.path), self.scroll, self.dock_scroll,
        );
        let path = self.layout_path()?;
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default().as_nanos();
        let temporary = path.with_extension(format!("{}.{timestamp}.tmp", std::process::id()));
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary)?;
        let written = file.write_all(&bytes);
        drop(file);
        let result = written.and_then(|_| {
            if std::fs::read(path)? != original {
                return Err(io::Error::other("Layout changed during the move"));
            }
            std::fs::rename(&temporary, path)
        });
        if let Err(error) = result {
            let _ = std::fs::remove_file(temporary);
            return Err(error);
        }

        self.saved = saved;
        self.scroll = layout.scroll;
        self.dock_scroll = layout.dock_scroll;
        self.layout = layout;
        self.assets.clear_entrance();
        Ok(())
    }
    fn command(&mut self,ctx:serde_json::Value)->Result<(),String>{
        if self.settings_open || self.settings.is_none(){return Err("Close the editor and start the dock with --settings-exe before editing.".into());}
        let step = match ctx["action"].as_str() {
            Some("Move folder earlier" | "Move shortcut earlier") => Some(-1),
            Some("Move folder later" | "Move shortcut later") => Some(2),
            _ => None,
        };
        if let (Some(step), Some(id)) = (step, ctx["entry"].as_str()) {
            let index = self.saved.ordered().iter().position(|entry| entry.id == id)
                .ok_or("Dock entry moved or was removed")?;
            let slot = index.saturating_add_signed(step);
            return self.move_dock(id, slot).and_then(|()| self.draw())
                .map_err(|error| error.to_string());
        }
        self.start_helper(&[("SD_MENU_X",ctx["x"].to_string()),("SD_MENU_Y",ctx["y"].to_string()),("SD_COMMAND",ctx.to_string())])
    }
    fn start_helper(&mut self, env:&[(&str,String)]) -> Result<(),String> {
        let Some(settings) = &self.settings else {
            return Ok(());
        };
        if self.settings_open {
            return Ok(());
        }

        let initial_stamp = std::fs::metadata(&settings.layout_path)
            .ok()
            .and_then(|m| Some((m.modified().ok()?, m.len())));
        let search = env.iter().any(|(key, value)| {
            *key == "SD_COMMAND" && serde_json::from_str::<serde_json::Value>(value)
                .is_ok_and(|context| context["action"] == "Find shortcut…")
        });
        let (send_selection, receive_selection) = std::sync::mpsc::channel();
        let mut child = Command::new(&settings.executable)
            .stdout(if search { std::process::Stdio::piped() } else { std::process::Stdio::inherit() })
            .stdin(if env.is_empty(){std::process::Stdio::null()}else{std::process::Stdio::piped()})
            .envs(env.iter().filter(|(k,_)|*k!="SD_COMMAND").map(|(k,v)|(*k,v)))
            .arg("--settings-only")
            .spawn()
            .map_err(|error| format!("Could not open StackDock Settings: {error}"))?;
        if let Some((_,json))=env.iter().find(|(k,_)|*k=="SD_COMMAND"){
            use std::io::Write;
            if let Some(mut input)=child.stdin.take(){input.write_all(json.as_bytes()).map_err(|e|e.to_string())?;}
        }
        let menu_session=!env.is_empty();
        // Wait on another thread so the native dock stays responsive.
        let hwnd = self.hwnd as usize;
        std::thread::spawn(move || {
            let status = if search {
                match child.wait_with_output() {
                    Ok(output) if output.status.success() => 20,
                    Ok(output) if output.status.code() == Some(25) => {
                        if let Ok(selection) = serde_json::from_slice::<SearchSelection>(&output.stdout) {
                            let _ = send_selection.send(selection);
                        }
                        25
                    }
                    _ => 0,
                }
            } else {
                let code = child.wait().ok().and_then(|status| status.code()).unwrap_or(-1);
                if menu_session && matches!(code, 0 | 21..=24) {
                    if code == 0 { 20 } else { code as usize }
                } else {
                    usize::from(code == 0)
                }
            };
            unsafe {
                PostMessageW(hwnd as Handle, WM_SETTINGS_CLOSED, status, 0);
            }
        });
        self.settings_open = true;
        self.settings_stamp = initial_stamp;
        self.search_selection = search.then_some(receive_selection);
        if unsafe { SetTimer(self.hwnd, SETTINGS_TIMER, 250, null_mut()) } == 0 {
            return Err("Settings opened, but live Apply could not start its timer.".into());
        }
        Ok(())
    }

    fn poll_settings(&mut self) -> io::Result<()> {
        if !self.settings_open {
            return Ok(());
        }

        let Some(link) = &self.settings else {
            return Ok(());
        };
        let Ok(metadata) = std::fs::metadata(&link.layout_path) else {
            return Ok(());
        };
        let Ok(modified) = metadata.modified() else {
            return Ok(());
        };

        let stamp = (modified, metadata.len());
        if self.settings_stamp == Some(stamp) {
            return Ok(());
        }

        // Only replace the active layout after the new file loads successfully.
        let Ok(saved) = Saved::load(&link.layout_path) else {
            return Ok(());
        };

        let keep_open=self.saved.folder_at(&self.path).zip(saved.folder_at(&self.path)).is_some_and(|(a,b)|a.id==b.id);
        self.saved=saved;self.clear_hover();self.assets.clear_layout_images();self.assets.prefetch(&self.saved);
        if keep_open{self.relayout()?;self.settle(1.)?;}else{self.settle(0.)?;}
        self.settings_stamp = Some(stamp);
        Ok(())
    }

    fn settings_closed(&mut self, success: bool) -> Result<(), String> {
        unsafe { KillTimer(self.hwnd, SETTINGS_TIMER) };
        self.settings_open = false;
        self.settings_stamp = None;
        self.search_selection = None;
        if !success {
            return Err(
                "StackDock Settings exited with an error. The dock kept its previous layout."
                    .into(),
            );
        }
        let Some(link) = &self.settings else {
            return Ok(());
        };
        let saved = Saved::load(&link.layout_path)?;
        self.saved = saved;
        self.assets.clear_layout_images();self.assets.prefetch(&self.saved);
        self.clear_hover();
        self.dock_scroll = 0.;
        self.settle(0.).map_err(|error| error.to_string())
    }
    fn relayout(&mut self) -> io::Result<()> {
        let (work, s) = desktop_rect(self.hwnd)?;
        self.layout = Layout::new(
            &self.saved,
            work,
            s,
            self.saved.folder_at(&self.path),
            self.scroll,
            self.dock_scroll,
        );
        self.scroll = self.layout.scroll;
        self.dock_scroll = self.layout.dock_scroll;
        Ok(())
    }
    fn start_indicator(&mut self,item:Entry)->io::Result<()>{
        if !self.saved.dock.launch.launch_enabled.unwrap_or(true){
            return if self.path.is_empty(){Ok(())}else{self.go(0.)};
        }
        let g=&self.layout;let dock=g.dock;let grid=!self.path.is_empty();
        let from=self.bounds();
        let source=if grid{self.saved.folder_at(&self.path).and_then(|f|g.grid_tiles.iter().find(|t|f.slots[t.item].id==item.id))}
            else{let order=self.saved.ordered();g.dock_tiles.iter().find(|t|order[t.item].id==item.id)}.map_or(dock,|tile|tile.icon);
        let source_skin=if grid{g.gs}else{g.ds};
        let (w,h)=(from.w as usize,from.h as usize);
        let mut previous=vec![0;w*h];
        crate::paint::content_frame(&mut Canvas::new(&mut previous,w,h,from),&mut self.assets,
            View{clock:self.clock,nav:None,saved:&self.saved,layout:g,path:&self.path,dock_index:self.dock_index,
                p:self.progress,a:self.icon_progress,hover:self.hover,hover_levels:&self.hover_levels});
        let (work,s)=desktop_rect(self.hwnd)?;
        let style=&self.saved.dock.launch;
        let card=Layout::panel_rect(&self.saved,work,14. * s,style.width()*s,style.height()*s);
        let moving=self.saved.dock.motion.unwrap_or(true)&&motion_enabled();
        let sprite=crate::paint::launch_sprite(&mut self.assets,&item,self.saved.general.image_thumbnails.unwrap_or(true),source.w.max(style.icon()*s));
        let mut indicator=Indicator{sprite,timer_delay:0,item,source,from,card,previous,source_skin,start:Instant::now(),
            travel:if moving{style.travel()}else{0.},hold:style.hold(),return_time:if moving{style.return_time()}else{0.}};
        self.stop();self.navigation=None;self.clear_hover();
        self.clock=Clock{t:0.,opening:true};self.progress=0.;self.icon_progress=0.;
        self.path.clear();self.dock_index=None;self.scroll=(0.,0.);self.relayout()?;
        indicator.start = Instant::now();
        self.indicator = Some(indicator);
        self.indicator_tick()?;
        if let Some(indicator) = &mut self.indicator {
            indicator.start = Instant::now();
            indicator.timer_delay = 16;
            if unsafe { SetTimer(self.hwnd, 7, 16, null_mut()) } == 0 {
                self.indicator = None;
                return self.draw();
            }
        }
        Ok(())
    }
    fn indicator_tick(&mut self)->io::Result<()>{
        let Some(i)=&mut self.indicator else{return Ok(())};let ms=i.start.elapsed().as_secs_f32()*1000.;
        if ms>=i.travel+i.hold+i.return_time{unsafe{KillTimer(self.hwnd,7)};self.indicator=None;return self.draw();}
        let delay=if ms>=i.travel && ms<i.travel+i.hold{(i.travel+i.hold-ms).ceil() as u32}else{16};
        if i.timer_delay!=delay{
            i.timer_delay=delay;
            if unsafe{SetTimer(self.hwnd,7,delay.max(1),null_mut())}==0{self.indicator=None;return self.draw();}
        }
        self.draw()
    }
    fn bounds(&self) -> Rect {
        self.bounds_at(Instant::now())
    }

    fn bounds_at(&self, now: Instant) -> Rect {
        if let Some(i)=&self.indicator{
            let ms=now.duration_since(i.start).as_secs_f32()*1000.;
            return if ms<i.travel{i.from.union(i.card)}else if ms<i.travel+i.hold{i.card}else{i.card.union(self.layout.dock)};
        }
        if let Some(n)=&self.navigation {return n.old.grid.union(self.layout.grid);}
        let panel=self.layout.envelope(self.progress,self.motion.is_some());
        self.tip.as_ref().map_or(panel,|tip|panel.union(tip.rect))
    }
    fn draw(&mut self) -> io::Result<()> {
        let frame_started = Instant::now();
        let bounds = self.bounds_at(frame_started);
        let (w, h) = (bounds.w as usize, bounds.h as usize);
        if self
            .surface
            .as_ref()
            .is_none_or(|s| s.width != w || s.height != h)
        {
            self.surface = Some(Surface::new(w, h)?);
        }
        let focus=self.focus_rect();
        let destination=self.drag_at.map(|(x,y)|self.context(&Point{x,y}));
        let surface = self.surface.as_mut().expect("surface allocated");
        let pixels = unsafe { slice::from_raw_parts_mut(surface.bits, w * h) };
        if let Some(i)=&self.indicator{
            let ms=frame_started.duration_since(i.start).as_secs_f32()*1000.;
            let enter=if i.travel>0.{ease_open((ms/i.travel).clamp(0.,1.))}else{1.};
            let exit=if ms<=i.travel+i.hold{0.}else if i.return_time>0.{ease_open(((ms-i.travel-i.hold)/i.return_time).clamp(0.,1.))}else{1.};
            let mut c=Canvas::new(pixels,w,h,bounds);
            c.launch_card(&mut self.assets,&self.saved,&self.layout,&i.item,i.source,i.from,i.card,enter,exit,i.sprite.as_ref(),i.source_skin,&i.previous);
            return surface.present(self.hwnd,bounds);
        }
        let moving=self.drag_at.is_some();
        if moving && self.drag_base.len()==pixels.len(){
            pixels.copy_from_slice(&self.drag_base);
        }else{
        frame(
            &mut Canvas::new(pixels, w, h, bounds),
            &mut self.assets,
            View {
                clock:self.clock, nav:self.navigation.as_ref(),
                saved: &self.saved,
                layout: &self.layout,
                path: &self.path,
                dock_index: self.dock_index,
                p: self.progress,
                a: self.icon_progress,
                hover: self.hover,
                hover_levels: &self.hover_levels,
            },
        );
        if moving{self.drag_base.clear();self.drag_base.extend_from_slice(pixels);}else{self.drag_base.clear();}
        }
        if let Some((x,y))=self.drag_at{
            let mut overlay=Canvas::overlay(pixels,w,h,bounds);
            let ctx=destination.as_ref().unwrap();let g=&self.layout;
            if ctx["valid"]==true && !self.path.is_empty(){
                let skin=g.gs;let clip=g.grid_clip;overlay.clip=clip;
                let cw=if skin.labels{skin.icon.max(skin.label_w)}else{skin.icon};
                let ch=skin.icon+skin.effects_height()+if skin.labels{skin.font*2.36+4.*g.s}else{0.};
                let col=ctx["c"].as_u64().unwrap_or(0) as f32;let row=ctx["r"].as_u64().unwrap_or(0) as f32;
                overlay.drop_cell(Rect::new(clip.x+col*(cw+skin.gx)-g.scroll.0,clip.y+row*(ch+skin.gy)-g.scroll.1,cw,ch),g.s);
                if let Some((edge,time))=self.dwell{
                    if time.elapsed()<Duration::from_secs(1){
                        let t=6.*g.s;
                        let strip=match edge{"left"=>Rect::new(clip.x,clip.y,t,clip.h),"right"=>Rect::new(clip.right()-t,clip.y,t,clip.h),"top"=>Rect::new(clip.x,clip.y,clip.w,t),_=>Rect::new(clip.x,clip.bottom()-t,clip.w,t)};
                        overlay.rounded(strip,3.*g.s,0xffead1ff);overlay.rounded(strip.inset(g.s),2.*g.s,0x80d99bff);
                    }
                }
            }else if ctx["valid"]==true{
                overlay.clip=g.dock;
                let slot=ctx["slot"].as_u64().unwrap_or(0) as usize;
                let vert=self.saved.orientation=="vertical" || (self.saved.orientation!="horizontal" && matches!(self.saved.edge.as_str(),"left"|"right"));
                if let Some(tile)=g.dock_tiles.iter().find(|t|t.item==slot).or_else(||g.dock_tiles.last()){
                    let r=tile.box_;let after=tile.item<slot;
                    let line=if vert{Rect::new(r.x,if after{r.bottom()}else{r.y},r.w,3.*g.s)}else{Rect::new(if after{r.right()}else{r.x},r.y,3.*g.s,r.h)};
                    overlay.rounded(line,1.,0xceae8ac0);
                }
            }
            if let Some((src,_,_))=self.press{
                let entry=match src{Hit::Grid(i)=>self.saved.folder_at(&self.path).and_then(|f|f.slots.get(i)),Hit::Dock(i)=>self.saved.ordered().get(i).copied(),_=>None};
                if let Some(e)=entry{
                    if self.ghost.is_empty(){
                        let size=if self.path.is_empty(){g.ds.icon}else{g.gs.icon};
                        self.ghost_size=size.round().clamp(1.,512.) as usize;
                        self.ghost.resize(self.ghost_size*self.ghost_size,0);
                        let side=self.ghost_size as f32;
                        Canvas::new(&mut self.ghost,self.ghost_size,self.ghost_size,Rect::new(0.,0.,side,side))
                            .ghost(&mut self.assets,e,side/2.,side/2.,side/36.);
                    }
                    overlay.sprite(&self.ghost,self.ghost_size,x,y);
                }
            }
        }
        if let Some(tip)=&self.tip{
            let phase=if motion_enabled(){(self.tip_fade.elapsed().as_secs_f32()/0.070).min(1.)}else{1.};
            let alpha=if self.tip_leaving{1.-phase}else{phase};
            tip.paint(&mut Canvas::overlay(pixels,w,h,bounds),&mut self.assets,alpha);
        }
        if let Some(r)=focus{
            let mut overlay=Canvas::overlay(pixels,w,h,bounds);
            let stroke=(2.*self.layout.s).round().max(1.);
            for edge in [Rect::new(r.x,r.y,r.w,stroke),Rect::new(r.x,r.bottom()-stroke,r.w,stroke),
                Rect::new(r.x,r.y,stroke,r.h),Rect::new(r.right()-stroke,r.y,stroke,r.h)]{
                overlay.rounded(edge,1.,0xffe4b5ff);
            }
        }
        let paint_ms = frame_started.elapsed().as_secs_f64() * 1000.0;
        let result = surface.present(self.hwnd, bounds);
        let total_ms = frame_started.elapsed().as_secs_f64() * 1000.0;

        if total_ms >= 20.0 {
            eprintln!(
                "[slow frame] total={total_ms:.1} ms  before_present={paint_ms:.1} ms  present={:.1} ms",
                total_ms - paint_ms
            );
        }
        result
    }
    fn stop(&mut self) {
        self.motion = None;
        unsafe {
            KillTimer(self.hwnd, ANIMATION_TIMER);
        }
    }
    fn settle(&mut self, to: f32) -> io::Result<()> {
        self.stop();
        self.navigation=None;
        self.clock=Clock {t:to,opening:true};
        self.progress = to;
        self.icon_progress = to;
        if to == 0. {
            self.path.clear();
            self.dock_index = None;
            self.scroll = (0., 0.);
            self.relayout()?;
        }
        self.draw()
    }
    fn go(&mut self, to: f32) -> io::Result<()> {
        if self.motion.as_ref().is_some_and(|m| m.to == to)
            || (self.motion.is_none() && self.progress == to)
        {
            return Ok(());
        }
        self.navigation=None;
        self.clear_hover();
        let speed = 100.
            / self
                .saved
                .dock
                .motion_speed
                .unwrap_or(100.)
                .clamp(25., 250.);
        let duration = Duration::from_secs_f32(
            ((if to > 0. { 0.420 } else { 0.325 }) * speed * (self.progress - to).abs()).max(0.045),
        );
        if !self.saved.dock.motion.unwrap_or(true)
            || !motion_enabled()
            || unsafe { SetTimer(self.hwnd, ANIMATION_TIMER, 15, null_mut()) } == 0
        {
            return self.settle(to);
        }
        self.clock=Clock {t:0.,opening:to>0.};
        self.motion = Some(Motion {
            start: Instant::now(),
            from: self.progress,
            to,
            duration,
            icon_from: self.icon_progress,
        });
        self.present_motion_start()
    }
    fn present_motion_start(&mut self) -> io::Result<()> {
        self.draw()?;
        if let Some(motion) = &mut self.motion {
            motion.start = Instant::now();
        }
        Ok(())
    }
    fn tick(&mut self) -> io::Result<()> {
        let Some(m) = &self.motion else { return Ok(()) };
        let t = bounded(m.start.elapsed().as_secs_f32() / m.duration.as_secs_f32());
        let to = m.to;
        self.clock=Clock {t,opening:to>0.};
        if t >= 1. {
            return self.settle(to);
        }
        let (panel, icon) = if to > 0. {
            (ease_open(t), ease(t, 0.24, 0.74, 0.20, 1.))
        } else {
            let p = ease_close(t);
            (p, p)
        };
        self.progress = mix(m.from, to, panel);
        self.icon_progress = mix(m.icon_from, to, icon);
        self.draw()
    }
    fn hit(&self, x: i32, y: i32) -> Option<Hit> {
        if self.motion.is_some() {
            return None;
        }
        let b = self.bounds();
        let (x, y) = (b.x + x as f32, b.y + y as f32);
        let grid=self.progress>0.;
        if grid && self.layout.settings_gear.is_some_and(|gear| gear.contains(x, y)) {
            return Some(Hit::SettingsGear);
        }
        if grid && self.path.len()>1 && self.layout.back.contains(x,y){return Some(Hit::Back);}
        let (tiles,clip,style)=if grid{
            (&self.layout.grid_tiles,self.layout.grid_clip,&self.saved.folder_at(&self.path)?.style)
        }else{(&self.layout.dock_tiles,self.layout.dock,&self.saved.dock)};
        if !clip.contains(x,y){return None;}
        let tag=|i|if grid{Hit::Grid(i)}else{Hit::Dock(i)};
        if let Some(tile)=tiles.iter().find(|t|t.box_.contains(x,y)){return Some(tag(tile.item));}
        if !style.hit_expand.unwrap_or(false){return None;}
        let reach=style.hit_padding.unwrap_or(8.).clamp(0.,48.)*self.layout.s;
        tiles.iter().filter_map(|t|{
            let r=t.box_.intersect(clip);if r.w<=0.||r.h<=0.{return None;}
            let dx=(r.x-x).max(x-r.right()).max(0.);let dy=(r.y-y).max(y-r.bottom()).max(0.);
            let distance=dx*dx+dy*dy;
            (distance<=reach*reach).then_some((distance,tag(t.item)))
        }).min_by(|a,b|a.0.total_cmp(&b.0)).map(|v|v.1)
    }
    fn update_tip(&mut self,details:bool)->io::Result<()>{
        let Some(hit)=self.hover else{return Ok(())};
        let grid=matches!(hit,Hit::Grid(_));
        let (entry,tile,style,labels)=match hit{
            Hit::Dock(i)=>{
                let order=self.saved.ordered();let Some(&e)=order.get(i)else{return Ok(())};
                (e,self.layout.dock_tiles.iter().find(|t|t.item==i),&self.saved.dock,self.layout.ds.labels)
            }
            Hit::Grid(i)=>{
                let Some(f)=self.saved.folder_at(&self.path)else{return Ok(())};let Some(e)=f.slots.get(i)else{return Ok(())};
                (e,self.layout.grid_tiles.iter().find(|t|t.item==i),&f.style,self.layout.gs.labels)
            }
            Hit::Back | Hit::SettingsGear=>return Ok(())
        };
        let Some(tile)=tile else{return Ok(())};
        if !style.hover_labels.unwrap_or(true)||((!grid||entry.folder())&&!entry.folder_hover_labels.unwrap_or(true)){return Ok(());}
        let general=&self.saved.general;
        let name=if !labels&&general.hover_names.unwrap_or(true){entry.name.as_str()}else{""};
        let delay=if style.custom_hover_delay.unwrap_or(false){style.hover_details_delay}else{general.hover_delay}.unwrap_or(1500).min(5000);
        let target=if general.hover_targets.unwrap_or(true){entry.target.as_str()}else{""};
        if !details && delay>0 && !target.is_empty(){unsafe{SetTimer(self.hwnd,9,delay as u32,null_mut());}}
        let target=if details||delay==0{target}else{""};
        if name.is_empty()&&target.is_empty(){return Ok(());}
        let (work,dpi)=desktop_rect(self.hwnd)?;
        let was_visible=self.tip.is_some();let bottom=if labels{tile.box_.bottom()}else{tile.icon.bottom()};
        self.tip=crate::paint::HoverTip::new(&mut self.assets,tile.icon,bottom,work,dpi,style.hover_label_size.unwrap_or(12.).clamp(9.,28.)*dpi,style.hover_label_background.unwrap_or(true),name,target);
        if !was_visible&&self.tip.is_some(){self.tip_fade=Instant::now();unsafe{SetTimer(self.hwnd,10,15,null_mut());}}
        Ok(())
    }
    fn clear_hover(&mut self) {
        self.tip=None;self.tip_leaving=false;unsafe{KillTimer(self.hwnd,9);KillTimer(self.hwnd,10);}
        self.hover = None;
        self.hover_levels.clear();
        self.hover_transitions.clear();
        unsafe { KillTimer(self.hwnd, HOVER_TIMER) };
    }

    fn sample_hover(&mut self, now: Instant) {
        let levels = &mut self.hover_levels;
        self.hover_transitions.retain(|transition| {
            let (value, done) = transition.sample(now);
            if let Some(entry) = levels.iter_mut().find(|e| e.0 == transition.hit) {
                entry.1 = value;
            }
            !done
        });
        levels.retain(|e| e.1 != 0. || Some(e.0) == self.hover);
    }

    fn hover(&mut self, to: Option<Hit>) -> io::Result<()> {
        if self.dragging || self.press.is_some() || self.drag_at.is_some(){return Ok(());}
        if self.hover == to {
            return Ok(());
        }

        let now = Instant::now();
        self.sample_hover(now);
        let previous = self.hover;
        self.hover = to;
        unsafe{KillTimer(self.hwnd,9);KillTimer(self.hwnd,10);}
        self.tip_leaving=to.is_none()&&self.tip.is_some()&&motion_enabled();
        if self.tip_leaving{self.tip_fade=now;unsafe{SetTimer(self.hwnd,10,15,null_mut());}}
        else{self.tip=None;self.update_tip(false)?;}
        let animations = motion_enabled();

        for (hit, target) in [(previous, 0.), (to, 1.)] {
            let Some(hit) = hit else { continue };
            let skin = match hit {
                Hit::Dock(_) => self.layout.ds,
                Hit::Grid(_) => self.layout.gs,
                Hit::Back | Hit::SettingsGear => continue,
            };

            self.hover_transitions.retain(|entry| entry.hit != hit);
            let index = match self.hover_levels.iter().position(|e| e.0 == hit) {
                Some(index) => index,
                None => {
                    self.hover_levels.push((hit, 0.));
                    self.hover_levels.len() - 1
                }
            };
            let from = self.hover_levels[index].1;
            if animations && (skin.hover_mode != 0 || skin.highlight_alpha > 0.)
                && skin.hover_duration > 0.
                && from != target
            {
                self.hover_transitions.push(HoverTransition {
                    hit,
                    from,
                    to: target,
                    start: now,
                    duration_ms: skin.hover_duration * (target - from).abs().min(1.),
                    curve: skin.hover_curve,
                });
            } else {
                self.hover_levels[index].1 = target;
            }
        }

        if !self.hover_transitions.is_empty()
            && unsafe { SetTimer(self.hwnd, HOVER_TIMER, 15, null_mut()) } == 0
        {
            for transition in self.hover_transitions.drain(..) {
                if let Some(e) = self.hover_levels.iter_mut().find(|e| e.0 == transition.hit) {
                    e.1 = transition.to;
                }
            }
        }
        self.tick_hover()
    }

    fn tick_hover(&mut self) -> io::Result<()> {
        self.sample_hover(Instant::now());
        if self.hover_transitions.is_empty() {
            unsafe { KillTimer(self.hwnd, HOVER_TIMER) };
        }
        self.draw()
    }
    fn focus_rect(&self)->Option<Rect>{
        let (tiles,index,clip)=match self.key_focus?{
            Hit::Dock(i) if self.path.is_empty()=>(&self.layout.dock_tiles,i,self.layout.dock),
            Hit::Grid(i) if !self.path.is_empty()=>(&self.layout.grid_tiles,i,self.layout.grid_clip),
            _=>return None,
        };
        tiles.iter().find(|t|t.item==index).map(|t|t.box_.intersect(clip))
    }
    fn arrow(&mut self,key:usize)->io::Result<()>{
        self.clear_hover();
        if let Some(f)=self.saved.folder_at(&self.path){
            let current=match self.key_focus{Some(Hit::Grid(i))=>f.slots.get(i),_=>None};
            let next=if let Some(e)=current{
                f.slots.iter().enumerate().filter_map(|(i,n)|{
                    let dx=n.c as i32-e.c as i32;let dy=n.r as i32-e.r as i32;
                    let (forward,side)=match key{37=>(-dx,dy),38=>(-dy,dx),39=>(dx,dy),_=>(dy,dx)};
                    (forward>0).then_some(((side.abs(),forward),i))
                }).min().map(|(_,i)|i)
            }else{f.slots.iter().enumerate().min_by_key(|(_,e)|(e.r,e.c)).map(|(i,_)|i)};
            if let Some(i)=next{
                self.key_focus=Some(Hit::Grid(i));
                let e=&f.slots[i];let g=&self.layout;let s=g.gs;
                let w=if s.labels{s.icon.max(s.label_w)}else{s.icon};
                let h=s.icon+s.effects_height()+if s.labels{s.font*2.36+4.*g.s}else{0.};
                let x=e.c as f32*(w+s.gx);let y=e.r as f32*(h+s.gy);
                self.scroll=((x+w-g.grid_clip.w).max(self.scroll.0.min(x)),(y+h-g.grid_clip.h).max(self.scroll.1.min(y)));
                self.relayout()?;
            }
        }else{
            let count=self.saved.ordered().len();if count==0{return Ok(());}
            let i=match self.key_focus{Some(Hit::Dock(i)) if key==37||key==38=>i.saturating_sub(1),Some(Hit::Dock(i))=>(i+1).min(count-1),_=>0};
            self.key_focus=Some(Hit::Dock(i));
            if self.focus_rect().is_none(){let step=self.layout.ds.icon+self.layout.ds.gx;
                self.dock_scroll=if key==37||key==38{(self.dock_scroll-step).max(0.)}else{self.dock_scroll+step};self.relayout()?;}
        }
        self.draw()
    }
    fn click(&mut self, hit: Hit) -> io::Result<Option<Target>> {
        self.key_focus=None;
        match hit {
            Hit::Dock(i) => {
                let order = self.saved.ordered();
                let Some(e) = order.get(i) else {
                    return Ok(None);
                };
                if !e.folder() {
                    return Ok(Some(Target::from(*e)));
                }
                let Some(root) = self.saved.folders.iter().position(|f| f.id == e.id) else {
                    return Ok(None);
                };
                self.path = vec![root];
                self.dock_index = Some(i);
                self.scroll = (0., 0.);
                self.relayout()?;
                self.go(1.)?;
            }
            Hit::Grid(i) => {
                let Some(e) = self
                    .saved
                    .folder_at(&self.path)
                    .and_then(|e| e.slots.get(i))
                else {
                    return Ok(None);
                };
                if !e.folder() {
                    let result = Target::from(e);
                    return Ok(Some(result));
                }
                self.navigate(i,true)?;
            }
            Hit::Back => {
                self.back()?;
            }
            Hit::SettingsGear => {
                if let (Some(gear), Some(folder)) = (self.layout.settings_gear, self.saved.folder_at(&self.path)) {
                    let context = serde_json::json!({
                        "action": "Grid Stack settings…",
                        "folder": folder.id,
                        "x": gear.cx() as i32,
                        "y": gear.cy() as i32,
                        "r": 0,
                        "c": 0
                    });
                    if !self.settings_open {
                        if let Err(error) = self.command(context) {
                            message(self.hwnd, &error);
                        }
                    }
                }
            }
        }
        Ok(None)
    }
    fn navigate(&mut self, slot:usize, down:bool) -> io::Result<()> {
        let previous=Navigation {old:self.layout.clone(),path:self.path.clone(),down,slot};
        if down {self.path.push(slot);} else {self.path.pop();}
        self.scroll=(0.,0.);
        self.clear_hover();
        self.relayout()?;
        if !self.saved.dock.motion.unwrap_or(true) || !motion_enabled()
            || unsafe {SetTimer(self.hwnd,ANIMATION_TIMER,15,null_mut())}==0 {return self.settle(1.);}
        self.navigation=Some(previous);
        self.clock=Clock {t:0.,opening:true};
        let speed=self.saved.dock.motion_speed.unwrap_or(100.).clamp(25.,250.);
        self.motion=Some(Motion {start:Instant::now(),from:1.,to:1.,icon_from:1.,
            duration:Duration::from_secs_f32(32.5/speed)});
        self.present_motion_start()
    }
    fn back(&mut self) -> io::Result<()> {
        self.key_focus=None;
        if self.path.len() > 1 && self.motion.is_none() {
            let slot=*self.path.last().unwrap();
            self.navigate(slot,false)
        } else {
            self.go(0.)
        }
    }
    fn wheel(&mut self, delta: i32, horizontal: bool) -> io::Result<()> {
        if self.motion.is_some() {
            return Ok(());
        }
        let n = delta as f32 / 120.;
        if self.progress == 0. {
            self.dock_scroll -= n * (self.layout.ds.icon + self.layout.ds.gx) * 2.
        } else if horizontal {
            self.scroll.0 += n * (self.layout.gs.icon + self.layout.gs.gx) * 2.
        } else {
            self.scroll.1 -= n * (self.layout.gs.icon + self.layout.gs.gy) * 2.
        }
        self.clear_hover();
        self.relayout()?;
        self.draw()
    }
    fn smoke(&mut self) -> io::Result<()> {
        self.draw()?;
        self.check_bounds(self.layout.dock)?;
        self.path = vec![0];
        self.relayout()?;
        self.motion = Some(Motion {
            start: Instant::now(),
            from: 0.,
            to: 1.,
            duration: Duration::from_millis(420),
            icon_from: 0.,
        });
        for p in [0.20_f32, 0.45, 0.8] {
            self.progress = p;
            self.icon_progress = p;
            self.draw()?;
            self.check_bounds(self.layout.dock.union(self.layout.grid))?;
        }
        self.settle(1.)?;
        self.check_bounds(self.layout.grid)?;
        self.settle(0.)?;
        self.check_bounds(self.layout.dock)
    }
    fn check_bounds(&self, b: Rect) -> io::Result<()> {
        let mut r: WinRect = unsafe { zeroed() };
        if unsafe { GetWindowRect(self.hwnd, &mut r) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if (r.left, r.top, r.right - r.left, r.bottom - r.top)
            != (b.x as i32, b.y as i32, b.w as i32, b.h as i32)
        {
            return Err(io::Error::other(
                "Layered window bounds did not match the plan.",
            ));
        }
        Ok(())
    }
}
thread_local! {static APP:RefCell<Option<App>>=const{RefCell::new(None)};}
fn desktop_rect(hwnd: Handle) -> io::Result<(Rect, f32)> {
    let monitor = unsafe { MonitorFromPoint(Point { x: 0, y: 0 }, 1) };
    let mut info: MonitorInfo = unsafe { zeroed() };
    info.size = size_of::<MonitorInfo>() as u32;
    if monitor.is_null() || unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let r = info.work;
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    Ok((
        Rect::new(
            r.left as f32,
            r.top as f32,
            (r.right - r.left) as f32,
            (r.bottom - r.top) as f32,
        ),
        if dpi == 0 { 1. } else { dpi as f32 / 96. },
    ))
}
const WM_MOUSEWHEEL: u32 = 0x020A;
const WM_MOUSEHWHEEL: u32 = 0x020E;
unsafe extern "system" fn window_proc(hwnd: Handle, msg: u32, w: usize, l: isize) -> isize {
    if msg == WM_DESTROY {
        unsafe { PostQuitMessage(0) };
        return 0;
    }
    if msg == WM_CONTEXTMENU {
        APP.with(|c|{if let Ok(mut state)=c.try_borrow_mut(){if let Some(a)=state.as_mut(){a.clear_hover();let _=a.draw();}}});
        context_menu(hwnd,l == -1);
        return 0;
    }
    if msg == WM_SETTINGS_CLOSED && w == 25 {
        let selected = APP.with(|cell| -> Result<Target, String> {
            let mut state = cell.borrow_mut();
            let app = state.as_mut().ok_or("StackDock is closing.")?;
            let update = app.poll_settings();
            unsafe { KillTimer(hwnd, SETTINGS_TIMER) };
            app.settings_open = false;
            app.settings_stamp = None;
            let selection = app.search_selection.take().and_then(|receiver| receiver.try_recv().ok());
            update.map_err(|error| error.to_string())?;
            selection.and_then(|selection| selection.resolve(&app.saved))
                .ok_or_else(|| "The selected shortcut is no longer available.".into())
        });
        if let Err(error) = selected.and_then(|target| launch(hwnd, target)) {
            message(hwnd, &error);
        }
        return 0;
    }
    if msg == WM_SETTINGS_CLOSED && (20..=24).contains(&w) {
        let result=APP.with(|cell|->io::Result<()> {
            let mut st=cell.borrow_mut(); let Some(a)=st.as_mut() else{return Ok(())};
            a.search_selection = None;
            let hit=a.menu_hit.take();
            let changed=a.settings_stamp;
            let update=a.poll_settings();
            unsafe{KillTimer(hwnd,SETTINGS_TIMER)};
            a.settings_open=false;
            let same=changed==a.settings_stamp;
            a.settings_stamp=None;
            update?;
            if !same || a.path.is_empty(){return Ok(());}
            match w {
                21=>{if let Some(h)=hit {let _=a.click(h)?;}},
                22=>a.back()?,
                23=>a.go(0.)?,
                _=>{}
            }
            Ok(())
        });
        if let Err(err)=result {message(hwnd,&err.to_string());}
        if w==24 {unsafe{DestroyWindow(hwnd)};}
        return 0;
    }
    if msg == WM_SETTINGS_CLOSED {
        let result = APP.with(|cell| {
            let mut state = cell.try_borrow_mut().ok()?;
            state.as_mut().map(|app| app.settings_closed(w != 0))
        });
        if let Some(Err(error)) = result {
            message(hwnd, &error);
        }
        return 0;
    }
    let mut target = None;
    let mut move_error = None;
    let outcome = APP.with(|cell| {
        // Layered window updates can send synchronous Win32 messages.
        let Ok(mut state) = cell.try_borrow_mut() else {
            return None;
        };
        let a = state.as_mut()?;
        match msg {
            WM_TIMER if w==9=>{unsafe{KillTimer(hwnd,9)};Some(a.update_tip(true).and_then(|_|a.draw()))}
            WM_TIMER if w==10=>{
                if a.tip_fade.elapsed()>=Duration::from_millis(70){
                    unsafe{KillTimer(hwnd,10)};if a.tip_leaving{a.tip=None;a.tip_leaving=false;}
                }
                // The motion timer already paints the tooltip on every hover frame.
                Some(if a.hover_transitions.is_empty(){a.draw()}else{Ok(())})
            }
            WM_TIMER if w==7 =>Some(a.indicator_tick()),
            WM_KEYDOWN if w==27 && a.indicator.is_some()=>{unsafe{KillTimer(hwnd,7)};a.indicator=None;Some(a.draw())}
            0x0201|WM_LBUTTONUP|WM_KEYDOWN|WM_MOUSEMOVE|WM_MOUSEWHEEL|WM_MOUSEHWHEEL if a.indicator.is_some()=>Some(Ok(())),
            0x0201 => {
                a.key_focus=None;
                let (x,y)=(coord(l,0),coord(l,16));let b=a.bounds();
                if let Some(h)=a.hit(x,y){a.clear_hover();a.press=Some((h,x+b.x as i32,y+b.y as i32));unsafe{SetCapture(hwnd)};return Some(a.draw());}
                Some(Ok(()))
            }
            WM_MOUSEMOVE if a.press.is_some() && w&1!=0 =>{
                let b=a.bounds();let (x,y)=(coord(l,0)+b.x as i32,coord(l,16)+b.y as i32);let (_,sx,sy)=a.press.unwrap();
                if matches!(a.press, Some((Hit::Grid(_) | Hit::Dock(_), _, _))) && a.settings.is_some() && !a.settings_open && !a.dragging && ((x-sx) as f32).hypot((y-sy) as f32)>7.*a.layout.s{
                    a.dragging=true;a.clear_hover();unsafe{SetTimer(hwnd,4,45,null_mut());}
                }
                if a.dragging{a.drag_at=Some((x,y));Some(a.draw())}else{Some(Ok(()))}
            }
            WM_TIMER if w==4=>Some(a.drag_tick()),
            WM_LBUTTONUP if a.dragging=>{
                let b=a.bounds();let point=Point{x:coord(l,0)+b.x as i32,y:coord(l,16)+b.y as i32};
                let mut ctx=a.context(&point);let (src,_,_)=a.press.unwrap();
                let source=match src{Hit::Grid(i)=>a.saved.folder_at(&a.path).and_then(|f|f.slots.get(i)),Hit::Dock(i)=>a.saved.ordered().get(i).copied(),_=>None};
                if let Some(e)=source{ctx["source"]=e.id.clone().into();}

                let new_scroll=a.strip.as_ref().filter(|_|ctx["valid"]==true).map(|_|a.scroll);
                a.clear_drag();if let Some(scroll)=new_scroll{a.scroll=scroll;}unsafe{ReleaseCapture()};
                if ctx["valid"] == true {
                    if let Some(source) = ctx["source"].as_str() {
                        let result = if ctx["folder"].is_string() {
                            a.move_grid(&ctx)
                        } else if let Some(slot) = ctx["slot"].as_u64() {
                            a.move_dock(source, slot as usize)
                        } else {
                            Ok(())
                        };
                        if let Err(error) = result {
                            move_error = Some(format!("Could not save the move: {error}"));
                        }
                    }
                }
                Some(a.draw())
            }
            WM_KEYDOWN if w==27 && a.dragging=>{a.clear_drag();unsafe{ReleaseCapture()};Some(a.draw())}
            WM_ACTIVATE if w&0xffff==0 && a.dragging=>{a.clear_drag();unsafe{ReleaseCapture()};Some(a.draw())}
            0x0215|0x001f if a.press.is_some()=>{a.clear_drag();Some(a.draw())}
            WM_MOUSEMOVE => {
                if !a.tracking {
                    let mut track = MouseTrack {
                        size: size_of::<MouseTrack>() as u32,
                        flags: 2,
                        hwnd,
                        hover_time: 0,
                    };
                    a.tracking = unsafe { TrackMouseEvent(&mut track) } != 0;
                }
                let hit = a.hit(coord(l, 0), coord(l, 16));
                Some(a.hover(hit))
            }
            WM_MOUSELEAVE => {
                a.tracking = false;
                Some(a.hover(None))
            }
            WM_LBUTTONUP => {let pressed=a.press.take().map(|p|p.0);unsafe{ReleaseCapture()};Some(if let Some(hit) = a.hit(coord(l, 0), coord(l, 16)).filter(|h|Some(*h)==pressed) {
                a.click(hit).map(|v| target = v)
            } else {
                Ok(())
            })}
            WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
                Some(a.wheel(coord(w as isize, 16), msg == WM_MOUSEHWHEEL))
            }
            WM_KEYDOWN if a.motion.is_none() && !a.dragging && !a.settings_open && (37..=40).contains(&w)=>Some(a.arrow(w)),
            WM_KEYDOWN if w==13 && a.motion.is_none() && !a.dragging && !a.settings_open=>{
                Some(if let Some(hit)=a.key_focus{a.click(hit).map(|v|target=v)}else{Ok(())})
            }
            WM_KEYDOWN if w==70 && !a.settings_open && !a.dragging && a.motion.is_none() &&
                unsafe{windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(17)}<0=>{
                let r=a.focus_rect().unwrap_or_else(||a.bounds());
                let ctx=serde_json::json!({"action":"Find shortcut…","x":r.cx() as i32,"y":r.cy() as i32});
                if let Err(error)=a.command(ctx){message(hwnd,&error);}Some(Ok(()))
            }
            WM_KEYDOWN if w == 27 || w == 8 => Some(a.back()),
            WM_KEYDOWN if (49..=57).contains(&w) && a.path.is_empty() => {
                Some(a.click(Hit::Dock(w - 49)).map(|v| target = v))
            }
            WM_TIMER if w == ANIMATION_TIMER => Some(a.tick()),
            WM_TIMER if w == HOVER_TIMER => Some(a.tick_hover()),
            WM_TIMER if w == SETTINGS_TIMER => Some(a.poll_settings()),
            WM_ACTIVATE
                if w & 0xffff == 0
                    && !a.settings_open
                    && a.saved.general.collapse_on_focus_loss.unwrap_or(true)
                    && !a.path.is_empty() =>
            {
                unsafe{SetTimer(hwnd,6,100,null_mut())};Some(Ok(()))
            }
            WM_TIMER if w==6=>{
                use windows::Win32::UI::{Input::KeyboardAndMouse::GetAsyncKeyState,WindowsAndMessaging::GetForegroundWindow};
                let active=unsafe{GetForegroundWindow()}.0==hwnd;
                let held=unsafe{GetAsyncKeyState(1)}<0;
                if active||a.path.is_empty()||a.settings_open||!a.saved.general.collapse_on_focus_loss.unwrap_or(true){
                    unsafe{KillTimer(hwnd,6)};Some(Ok(()))
                }else if held||a.drag_at.is_some(){Some(Ok(()))}
                else{unsafe{KillTimer(hwnd,6)};Some(a.go(0.))}
            }
            WM_DISPLAYCHANGE | WM_DPICHANGED | WM_SETTINGCHANGE => {
                a.drag_base.clear();a.ghost.clear();a.assets.clear_entrance();
                Some(a.relayout().and_then(|()| a.draw()))
            }
            _ => None,
        }
    });
    match outcome {
        Some(Ok(())) => {
            if let Some(error) = move_error {
                message(hwnd, &error);
            }
            if let Some(target) = target {
                if let Err(err) = launch(hwnd, target) {
                    message(hwnd, &err)
                }
            }
            0
        }
        Some(Err(e)) => {
            message(hwnd, &e.to_string());
            unsafe { PostQuitMessage(1) };
            0
        }
        None => unsafe { DefWindowProcW(hwnd, msg, w, l) },
    }
}

pub fn run(saved: Saved, smoke: bool, settings: Option<SettingsLink>) -> ExitCode {
    match run_window(saved, smoke, settings) {
        Ok(code) => code,
        Err(e) => {
            message(null_mut(), &e.to_string());
            ExitCode::FAILURE
        }
    }
}
pub fn report(error: &str) {
    message(null_mut(), error);
}
fn run_window(saved: Saved, smoke: bool, settings: Option<SettingsLink>) -> io::Result<ExitCode> {
    unsafe {
        SetProcessDpiAwarenessContext(-4isize as Handle);
    }
    let instance = unsafe { GetModuleHandleW(null()) };
    if instance.is_null() {
        return Err(io::Error::last_os_error());
    }
    let name = wide("StackDock.NativePreview");
    let title = wide("StackDock Native Preview");
    let class = WindowClass {
        size: size_of::<WindowClass>() as u32,
        style: 0,
        proc: Some(window_proc),
        class_extra: 0,
        win_extra: 0,
        instance,
        icon: null_mut(),
        cursor: unsafe { LoadCursorW(null_mut(), 32512usize as *const u16) },
        background: null_mut(),
        menu: null(),
        name: name.as_ptr(),
        small_icon: null_mut(),
    };
    if unsafe { RegisterClassExW(&class) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let hwnd = unsafe {
        CreateWindowExW(
            0x0008_0080,
            name.as_ptr(),
            title.as_ptr(),
            0x8000_0000,
            0,
            0,
            1,
            1,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        )
    };
    if hwnd.is_null() {
        return Err(io::Error::last_os_error());
    }
    let run = (|| {
        let mut app = App::new(hwnd, saved, settings)?;
        if smoke {
            app.smoke()?;
            return Ok(ExitCode::SUCCESS);
        }
        app.assets.prefetch(&app.saved);
        app.draw()?;
        APP.with(|c| c.replace(Some(app)));
        let _drop=native_drop::register(hwnd).map_err(io::Error::other)?;
        unsafe {
            ShowWindow(hwnd, 4);
        }
        let mut msg: Msg = unsafe { zeroed() };
        loop {
            match unsafe { GetMessageW(&mut msg, null_mut(), 0, 0) } {
                -1 => return Err(io::Error::last_os_error()),
                0 => {
                    return Ok(if msg.wparam == 0 {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    });
                }
                _ => unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                },
            }
        }
    })();
    if unsafe { IsWindow(hwnd) } != 0 {
        unsafe {
            DestroyWindow(hwnd);
        }
    }
    APP.with(|c| c.replace(None));
    run
}
