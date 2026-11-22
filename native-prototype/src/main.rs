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
fn sibling_settings_exe() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let executable = executable.canonicalize().ok()?;
    let settings = executable.with_file_name("StackDock.exe");
    let settings = settings.canonicalize().ok()?;
    (settings.is_file() && settings != executable).then_some(settings)
}

#[cfg(windows)]
fn is_settings_override_for_running_executable(settings: &std::path::Path) -> bool {
    let Ok(executable) = std::env::current_exe() else {
        return false;
    };
    let (Ok(executable), Ok(settings)) = (executable.canonicalize(), settings.canonicalize()) else {
        return false;
    };
    executable == settings
}

#[cfg(windows)]
fn load_shared_layout(path: &std::path::Path) -> Result<Saved, String> {
    use std::{
        fs::{self, OpenOptions},
        io::{self, ErrorKind, Write},
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT_SEED: AtomicU64 = AtomicU64::new(0);

    let seed = || -> io::Result<()> {
        let directory = path.parent().ok_or_else(|| {
            io::Error::new(ErrorKind::InvalidInput, "The layout path has no parent directory")
        })?;
        fs::create_dir_all(directory)?;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let number = NEXT_SEED.fetch_add(1, Ordering::Relaxed);
        let temporary = directory.join(format!(
            "layout-seed-{}-{nonce}-{number}.tmp",
            std::process::id()
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        let written = file
            .write_all(STARTER.as_bytes())
            .and_then(|_| file.sync_all());
        drop(file);

        // Publish the complete file without replacing a layout another process just saved.
        let published = written.and_then(|_| match fs::hard_link(&temporary, path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => Ok(()),
            Err(error) => Err(error),
        });
        let _ = fs::remove_file(&temporary);
        published
    };

    match fs::symlink_metadata(path) {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {
            Saved::from_json(STARTER.as_bytes())?;
            seed().map_err(|error| format!("Could not create {}: {error}", path.display()))?;
        }
        Err(error) => return Err(format!("Could not access {}: {error}", path.display())),
    }
    Saved::load(path).map_err(|error| format!("Could not load the shared StackDock layout: {error}"))
}

#[cfg(windows)]
fn main() -> ExitCode {
    match args().and_then(|p| {
        if p.layout.is_some() && p.settings_exe.is_some() {
            return Err("Use --layout for an exported snapshot, or --settings-exe for the shared layout, not both.".into());
        }
        let settings_exe = match p.settings_exe {
            Some(settings) => {
                if is_settings_override_for_running_executable(&settings) {
                    return Err("The --settings-exe path must refer to a Settings helper, not the running native executable.".into());
                }
                Some(settings)
            }
            None => {
                if p.layout.is_none() && !p.smoke {
                    sibling_settings_exe()
                } else {
                    None
                }
            }
        };
        let settings = settings_exe.map(windows::SettingsLink::new).transpose()?;
        let saved = match &settings {
            Some(link) => load_shared_layout(&link.layout_path)?,
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

#[cfg(all(test, windows))]
mod startup_tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_layout() -> (PathBuf, PathBuf) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "stack-dock-startup-test-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        (directory.join("layout.json"), directory)
    }

    #[test]
    fn seeds_missing_shared_layout() {
        let (path, directory) = temporary_layout();
        let saved = load_shared_layout(&path).unwrap();
        assert!(!saved.folders.is_empty());
        assert!(path.is_file());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn preserves_and_reports_invalid_shared_layout() {
        let (path, directory) = temporary_layout();
        std::fs::write(&path, b"{invalid").unwrap();
        let error = load_shared_layout(&path).unwrap_err();
        assert!(error.contains("Could not load the shared StackDock layout"));
        assert_eq!(std::fs::read(&path).unwrap(), b"{invalid");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
