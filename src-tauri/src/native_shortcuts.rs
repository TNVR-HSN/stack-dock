// Native dialogs and embedded icon resources. This module never runs the selected file.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use icoextract_rs::IconExtractor;
use std::{fs::File, io::Read, path::Path};
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
                .add_filter("Icons and programs", &["png", "jpg", "jpeg", "gif", "webp", "ico", "exe"])
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
        let (mime, bytes, count) = match ext.as_str() {
            "exe" => {
                let extractor = IconExtractor::from_path(path)
                    .map_err(|e| format!("Could not read embedded EXE icons: {e}"))?;
                let count = extractor.list_group_icons().map_err(|e| e.to_string())?.len();
                let bytes = extractor.icon_by_index(index)
                    .and_then(|icon| icon.to_ico_bytes())
                    .map_err(|e| format!("Could not extract icon {}: {e}", index + 1))?;
                ("image/x-icon", bytes, count)
            }
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" => {
                let mime = match ext.as_str() {
                    "jpg" | "jpeg" => "image/jpeg",
                    "ico" => "image/x-icon",
                    "png" => "image/png",
                    "gif" => "image/gif",
                    _ => "image/webp",
                };
                let mut bytes = Vec::new();
                File::open(path)
                    .map_err(|e| e.to_string())?
                    .take(2_097_153)
                    .read_to_end(&mut bytes)
                    .map_err(|e| e.to_string())?;
                (mime, bytes, 1)
            }
            _ => return Err("Choose PNG, JPEG, GIF, WebP, ICO, or EXE.".into()),
        };
        if bytes.len() > 2_097_152 {
            return Err("That icon image is larger than 2 MB.".into());
        }
        Ok(ShortcutIcon {
            source: format!("data:{mime};base64,{}", STANDARD.encode(bytes)),
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
