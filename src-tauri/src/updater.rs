//! Updates from the GitHub releases (tauri-plugin-updater): the app reads
//! latest.json of the newest release, checks the installer's signature with
//! the public key in tauri.conf.json (only the maker's private key can sign),
//! then installs it and starts again.

use serde_json::{json, Value};
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

use crate::poe::{send, trace};

/// { available, version, notes, current }
#[tauri::command]
pub async fn vs_update_check(app: AppHandle) -> Result<Value, String> {
    let current = app.package_info().version.to_string();
    let updater = app.updater().map_err(|e| e.to_string())?;
    match updater.check().await {
        Ok(Some(update)) => Ok(json!({
            "available": true,
            "version": update.version,
            "notes": update.body.clone().unwrap_or_default(),
            "current": current,
        })),
        Ok(None) => Ok(json!({ "available": false, "current": current })),
        Err(error) => {
            trace(&app, &format!("update check failed: {error}"));
            Err(error.to_string())
        }
    }
}

/// Downloads the new version, checks its signature, installs it, restarts.
#[tauri::command]
pub async fn vs_update_install(app: AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let Some(update) = updater.check().await.map_err(|e| e.to_string())? else {
        return Err("already up to date".into());
    };
    trace(&app, &format!("installing Vaal Street {}", update.version));
    // how far the download is, for the window's pixel progress bar
    let progress = app.clone();
    let mut downloaded: u64 = 0;
    let done = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                send(&progress, "update-progress", json!({ "downloaded": downloaded, "total": total }));
            },
            move || send(&done, "update-progress", json!({ "downloaded": 1, "total": 1, "installing": true })),
        )
        .await
        .map_err(|e| e.to_string())?;
    app.restart();
}
