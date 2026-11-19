// Win32 FFI is isolated here. The model and renderer are testable without Windows.
use crate::{
    model::{Entry, Layout, Rect, Saved, bounded, ease, mix},
    paint::{Assets, Canvas, Hit, View, ease_close, ease_open, frame},
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

fn context_menu(hwnd: Handle) {
    let mut pos = Point { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut pos) } == 0 {
        return;
    }
    let menu = unsafe { CreatePopupMenu() };
    if menu.is_null() {
        return;
    }
    let settings_state = APP.with(|cell| {
        let state = cell.try_borrow().ok()?;
        let app = state.as_ref()?;
        app.settings.as_ref().map(|_| app.settings_open)
    });
    let settings = wide("Settings…");
    let exit = wide("Exit native prototype");
    let id = unsafe {
        if let Some(open) = settings_state {
            // Gray out the command while a Settings window is already open.
            AppendMenuW(menu, if open { 1 } else { 0 }, 2, settings.as_ptr());
        }
        AppendMenuW(menu, 0, 1, exit.as_ptr());
        // TPM_RETURNCMD | TPM_RIGHTBUTTON
        let id = TrackPopupMenuEx(menu, 0x0102, pos.x, pos.y, hwnd, null_mut());
        DestroyMenu(menu);
        id
    };
    if id == 1 {
        unsafe {
            DestroyWindow(hwnd);
        }
    } else if id == 2 {
        let result = APP.with(|cell| {
            let mut state = cell.try_borrow_mut().ok()?;
            state.as_mut().map(App::open_settings)
        });
        if let Some(Err(error)) = result {
            message(hwnd, &error);
        }
    }
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
    kind: String,
    target: String,
    args: String,
    cwd: String,
}
impl From<&Entry> for Target {
    fn from(e: &Entry) -> Self {
        Self {
            kind: e.kind.clone(),
            target: e.target.clone(),
            args: e.args.clone(),
            cwd: e.cwd.clone(),
        }
    }
}
fn launch(hwnd: Handle, entry: Target) -> Result<(), String> {
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
    hover: Option<Hit>,
    hover_transitions: Vec<HoverTransition>,
    hover_levels: Vec<(Hit, f32)>,
    tracking: bool,
    settings: Option<SettingsLink>,
    settings_open: bool,
    settings_stamp: Option<(std::time::SystemTime, u64)>,
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
            hover: None,
            hover_transitions: Vec::new(),
            hover_levels: Vec::new(),
            tracking: false,
            settings,
            settings_open: false,
            settings_stamp: None,
        })
    }

    fn open_settings(&mut self) -> Result<(), String> {
        let Some(settings) = &self.settings else {
            return Ok(());
        };
        if self.settings_open {
            return Ok(());
        }

        let initial_stamp = std::fs::metadata(&settings.layout_path)
            .ok()
            .and_then(|m| Some((m.modified().ok()?, m.len())));
        let mut child = Command::new(&settings.executable)
            .arg("--settings-only")
            .spawn()
            .map_err(|error| format!("Could not open StackDock Settings: {error}"))?;
        // Wait on another thread so the native dock stays responsive.
        let hwnd = self.hwnd as usize;
        std::thread::spawn(move || {
            let status = child.wait().is_ok_and(|status| status.success());
            unsafe {
                PostMessageW(hwnd as Handle, WM_SETTINGS_CLOSED, usize::from(status), 0);
            }
        });
        self.settings_open = true;
        self.settings_stamp = initial_stamp;
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

        self.saved = saved;
        self.clear_hover();
        self.assets.clear_layout_images();
        self.dock_scroll = 0.;
        self.settle(0.)?;
        self.settings_stamp = Some(stamp);
        Ok(())
    }

    fn settings_closed(&mut self, success: bool) -> Result<(), String> {
        unsafe { KillTimer(self.hwnd, SETTINGS_TIMER) };
        self.settings_open = false;
        self.settings_stamp = None;
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
        self.assets.clear_layout_images();
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
    fn bounds(&self) -> Rect {
        self.layout.envelope(self.progress, self.motion.is_some())
    }
    fn draw(&mut self) -> io::Result<()> {
        let bounds = self.bounds();
        let (w, h) = (bounds.w as usize, bounds.h as usize);
        if self
            .surface
            .as_ref()
            .is_none_or(|s| s.width != w || s.height != h)
        {
            self.surface = Some(Surface::new(w, h)?);
        }
        let surface = self.surface.as_mut().expect("surface allocated");
        let pixels = unsafe { slice::from_raw_parts_mut(surface.bits, w * h) };
        frame(
            &mut Canvas::new(pixels, w, h, bounds),
            &mut self.assets,
            View {
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
        surface.present(self.hwnd, bounds)
    }
    fn stop(&mut self) {
        self.motion = None;
        unsafe {
            KillTimer(self.hwnd, ANIMATION_TIMER);
        }
    }
    fn settle(&mut self, to: f32) -> io::Result<()> {
        self.stop();
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
        self.motion = Some(Motion {
            start: Instant::now(),
            from: self.progress,
            to,
            duration,
            icon_from: self.icon_progress,
        });
        self.draw()
    }
    fn tick(&mut self) -> io::Result<()> {
        let Some(m) = &self.motion else { return Ok(()) };
        let t = bounded(m.start.elapsed().as_secs_f32() / m.duration.as_secs_f32());
        let to = m.to;
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
        if self.progress <= 0. {
            return self
                .layout
                .dock_tiles
                .iter()
                .find(|t| t.box_.contains(x, y))
                .map(|t| Hit::Dock(t.item));
        }
        if self.path.len() > 1 && self.layout.back.contains(x, y) {
            return Some(Hit::Back);
        }
        if !self.layout.grid_clip.contains(x, y) {
            return None;
        }
        self.layout
            .grid_tiles
            .iter()
            .find(|t| t.box_.contains(x, y))
            .map(|t| Hit::Grid(t.item))
    }
    fn clear_hover(&mut self) {
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
        if self.hover == to {
            return Ok(());
        }

        let now = Instant::now();
        self.sample_hover(now);
        let previous = self.hover;
        self.hover = to;
        let animations = motion_enabled();

        for (hit, target) in [(previous, 0.), (to, 1.)] {
            let Some(hit) = hit else { continue };
            let skin = match hit {
                Hit::Dock(_) => self.layout.ds,
                Hit::Grid(_) => self.layout.gs,
                Hit::Back => continue,
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
            if animations && skin.hover_mode != 0 && skin.hover_duration > 0.
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
    fn click(&mut self, hit: Hit) -> io::Result<Option<Target>> {
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
                    self.go(0.)?;
                    return Ok(Some(result));
                }
                self.path.push(i);
                self.scroll = (0., 0.);
                self.clear_hover();
                self.relayout()?;
                self.draw()?;
            }
            Hit::Back => {
                self.back()?;
            }
        }
        Ok(None)
    }
    fn back(&mut self) -> io::Result<()> {
        if self.path.len() > 1 && self.motion.is_none() {
            self.path.pop();
            self.scroll = (0., 0.);
            self.clear_hover();
            self.relayout()?;
            self.draw()
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
        context_menu(hwnd);
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
    let outcome = APP.with(|cell| {
        // Layered window updates can send synchronous Win32 messages.
        let Ok(mut state) = cell.try_borrow_mut() else {
            return None;
        };
        let a = state.as_mut()?;
        match msg {
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
            WM_LBUTTONUP => Some(if let Some(hit) = a.hit(coord(l, 0), coord(l, 16)) {
                a.click(hit).map(|v| target = v)
            } else {
                Ok(())
            }),
            WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
                Some(a.wheel(coord(w as isize, 16), msg == WM_MOUSEHWHEEL))
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
                    && a.saved.general.collapse_on_focus_loss.unwrap_or(true)
                    && !a.path.is_empty() =>
            {
                Some(a.go(0.))
            }
            WM_DISPLAYCHANGE | WM_DPICHANGED | WM_SETTINGCHANGE => {
                Some(a.relayout().and_then(|()| a.draw()))
            }
            _ => None,
        }
    });
    match outcome {
        Some(Ok(())) => {
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
        app.draw()?;
        APP.with(|c| c.replace(Some(app)));
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
