#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![cfg_attr(not(windows), allow(dead_code))]

mod model;
mod paint;
#[cfg(windows)]
mod resource_icon;
#[cfg(windows)]
mod windows;
use model::{STARTER, Saved};
use std::{ffi::OsStr, path::PathBuf, process::ExitCode};

struct Args {
    layout: Option<PathBuf>,
    #[cfg(windows)]
    settings_exe: Option<PathBuf>,
    smoke: bool,
    #[cfg(not(windows))]
    render: Option<PathBuf>,
}
fn args() -> Result<Args, String> {
    let mut args = std::env::args_os().skip(1);
    let mut parsed = Args {
        layout: None,
        #[cfg(windows)]
        settings_exe: None,
        smoke: false,
        #[cfg(not(windows))]
        render: None,
    };
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--layout") if parsed.layout.is_none() => {
                parsed.layout = Some(PathBuf::from(
                    args.next().ok_or("Pass a layout path after --layout.")?,
                ))
            }
            #[cfg(windows)]
            Some("--settings-exe") if parsed.settings_exe.is_none() => {
                parsed.settings_exe = Some(PathBuf::from(
                    args.next()
                        .ok_or("Pass the StackDock EXE path after --settings-exe.")?,
                ));
            }
            Some("--smoke-test") => parsed.smoke = true,
            #[cfg(not(windows))]
            Some("--render-preview") if parsed.render.is_none() => {
                parsed.render = Some(PathBuf::from(
                    args.next()
                        .ok_or("Pass a directory after --render-preview.")?,
                ))
            }
            _ if parsed.layout.is_none()
                && PathBuf::from(&arg).extension() == Some(OsStr::new("json")) =>
            {
                parsed.layout = Some(PathBuf::from(arg))
            }
            _ => {
                return Err(format!(
                    "Unrecognized argument {}. Pass --layout followed by a StackDock export.",
                    arg.to_string_lossy()
                ));
            }
        }
    }
    Ok(parsed)
}
fn load(path: Option<PathBuf>) -> Result<Saved, String> {
    if let Some(p) = path {
        return Saved::load(&p);
    }
    if let Some(p) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("stack-dock-layout.json")))
        .filter(|p| p.is_file())
    {
        return Saved::load(&p);
    }
    Saved::from_json(STARTER.as_bytes())
}
#[cfg(windows)]
fn main() -> ExitCode {
    match args().and_then(|p| {
        if p.layout.is_some() && p.settings_exe.is_some() {
            return Err("Use --layout for an exported snapshot, or --settings-exe for the shared layout, not both.".into());
        }
        let settings = p.settings_exe.map(windows::SettingsLink::new).transpose()?;
        let saved = match &settings {
            Some(link) => Saved::load(&link.layout_path).map_err(|error| {
                format!("Could not load the shared StackDock layout. Open the Phase2 dock and save its Settings first.\n\n{error}")
            })?,
            None => load(p.layout)?,
        };
        Ok((saved, p.smoke, settings))
    }) {
        Ok((saved, smoke, settings)) => windows::run(saved, smoke, settings),
        Err(e) => {
            windows::report(&e);
            ExitCode::FAILURE
        }
    }
}
#[cfg(not(windows))]
fn main() -> ExitCode {
    match args().and_then(|p|{
        let out=p.render.ok_or("The desktop window runs on Windows. Use --render-preview DIRECTORY for offline frames.")?;
        render(load(p.layout)?,out).map_err(|e|e.to_string())
    }){
        Ok(())=>ExitCode::SUCCESS,
        Err(e)=>{eprintln!("{e}");ExitCode::FAILURE}
    }
}
#[cfg(not(windows))]
fn render(saved: Saved, out: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use model::{Layout, Rect};
    use paint::{Assets, Canvas, View, frame};
    std::fs::create_dir_all(&out)?;
    let font = std::env::var_os("STACKDOCK_FONT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/share/fonts/truetype/dejavu"));
    let mut assets = Assets::new(&font);
    let work = Rect::new(0., 0., 1280., 780.);
    let index = saved
        .ordered()
        .iter()
        .position(|e| e.id == saved.folders[0].id);
    let l = Layout::new(&saved, work, 1., Some(&saved.folders[0]), (0., 0.), 0.);
    for (name, progress) in [("dock", 0.), ("opening", 0.54), ("grid", 1.)] {
        let b = l.envelope(progress, progress > 0. && progress < 1.);
        let w = b.w as usize;
        let h = b.h as usize;
        let mut pix = vec![0; w * h];
        frame(
            &mut Canvas::new(&mut pix, w, h, b),
            &mut assets,
            View {
                saved: &saved,
                layout: &l,
                path: &[0],
                dock_index: index,
                p: progress,
                a: progress,
                hover: None,
            },
        );
        let mut rgba = Vec::with_capacity(w * h * 4);
        for x in pix {
            let a = x >> 24;
            let straight = |s: u32| {
                if a == 0 {
                    0
                } else {
                    ((((x >> s) & 255u32) * 255u32 / a).min(255u32)) as u8
                }
            };
            rgba.extend([straight(16), straight(8), straight(0), a as u8]);
        }
        image::save_buffer(
            out.join(format!("{name}.png")),
            &rgba,
            w as u32,
            h as u32,
            image::ColorType::Rgba8,
        )?;
    }
    Ok(())
}
