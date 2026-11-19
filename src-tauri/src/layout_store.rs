use std::{
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use tauri::{AppHandle, Manager};

static NEXT_SAVE: AtomicU64 = AtomicU64::new(0);

fn layout_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?
        .join("layout.json"))
}

#[tauri::command]
pub fn load_shared_layout(app: AppHandle) -> Result<Option<String>, String> {
    let path = layout_path(&app)?;

    match fs::read_to_string(&path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Could not read {}: {error}", path.display())),
    }
}

#[tauri::command]
pub fn save_shared_layout(app: AppHandle, data: String) -> Result<(), String> {
    if data.len() > 8 * 1024 * 1024 {
        return Err("The layout is larger than 8 MiB.".into());
    }

    let backup: serde_json::Value =
        serde_json::from_str(&data).map_err(|error| error.to_string())?;

    if backup["format"].as_str() != Some("stack-dock-layout")
        || backup["version"].as_u64() != Some(1)
        || !backup["layout"]["folders"]
            .as_array()
            .is_some_and(|folders| !folders.is_empty())
    {
        return Err("Expected a version 1 StackDock layout.".into());
    }

    let path = layout_path(&app)?;
    let folder = path.parent().ok_or("Could not find the layout folder.")?;
    fs::create_dir_all(folder).map_err(|error| error.to_string())?;

    let number = NEXT_SAVE.fetch_add(1, Ordering::Relaxed);
    let temporary = folder.join(format!("layout-{}-{number}.tmp", std::process::id()));

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| error.to_string())?;

    let written = file.write_all(data.as_bytes()).and_then(|_| file.sync_all());
    drop(file);

    if let Err(error) = written.and_then(|_| fs::rename(&temporary, &path)) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("Could not save {}: {error}", path.display()));
    }

    Ok(())
}