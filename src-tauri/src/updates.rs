//! Automatic updates from GitHub Releases. MEDORE checks shortly after start
//! and every six hours, downloads a new version in the background, then
//! offers "Restart to update"; if the person keeps watching, the update is
//! installed when MEDORE closes. Updates are signed: the app refuses any file
//! not signed with the MEDORE release key.

use std::time::Duration;

use parking_lot::Mutex;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

#[derive(Default)]
pub struct Updates {
    ready: Mutex<Option<(Update, Vec<u8>)>>,
    busy: Mutex<bool>,
}

pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio_sleep(Duration::from_secs(20)).await;
        loop {
            let auto = app.state::<crate::AppState>().settings.bool("updates.auto_check");
            if auto {
                let _ = check(app.clone(), false).await;
            }
            tokio_sleep(Duration::from_secs(6 * 3600)).await;
        }
    });
}

async fn tokio_sleep(d: Duration) {
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(d)).await;
}

/// Returns {available, version?, notes?}. Downloads the update when found.
pub async fn check(app: AppHandle, manual: bool) -> Result<serde_json::Value, String> {
    {
        let st = app.state::<Updates>();
        let mut b = st.busy.lock();
        if *b {
            return Ok(json!({"available": false, "busy": true}));
        }
        if let Some((u, _)) = st.ready.lock().as_ref() {
            return Ok(json!({"available": true, "ready": true, "version": u.version, "notes": u.body}));
        }
        *b = true;
    }
    let res = do_check(&app).await;
    *app.state::<Updates>().busy.lock() = false;
    match &res {
        Err(e) if manual => log::info!("update check failed: {e}"),
        Err(e) => log::debug!("update check failed: {e}"),
        _ => {}
    }
    res
}

async fn do_check(app: &AppHandle) -> Result<serde_json::Value, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let Some(update) = updater.check().await.map_err(|e| e.to_string())? else {
        return Ok(json!({"available": false}));
    };
    let version = update.version.clone();
    let notes = update.body.clone();
    let _ = app.emit("update://downloading", json!({"version": version}));
    let a2 = app.clone();
    let mut got: u64 = 0;
    let bytes = update
        .download(
            move |chunk, total| {
                got += chunk as u64;
                if let Some(t) = total {
                    let _ = a2.emit("update://progress", json!({"done": got, "total": t}));
                }
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;
    *app.state::<Updates>().ready.lock() = Some((update, bytes));
    let _ = app.emit("update://ready", json!({"version": version, "notes": notes}));
    Ok(json!({"available": true, "ready": true, "version": version, "notes": notes}))
}

/// Install the downloaded update now (MEDORE closes and reopens).
pub fn install_now(app: &AppHandle) -> Result<(), String> {
    let taken = app.state::<Updates>().ready.lock().take();
    let Some((update, bytes)) = taken else { return Err("no update downloaded".into()) };
    if let Some(st) = app.try_state::<crate::AppState>() {
        st.before_exit();
    }
    update.install(bytes).map_err(|e| e.to_string())?;
    app.restart();
}

/// Called when MEDORE is closing: install a downloaded update if allowed.
pub fn install_on_exit(app: &AppHandle) {
    let allowed = app.state::<crate::AppState>().settings.bool("updates.auto_install");
    if !allowed {
        return;
    }
    let taken = app.state::<Updates>().ready.lock().take();
    if let Some((update, bytes)) = taken {
        log::info!("installing update {} on exit", update.version);
        if let Err(e) = update.install(bytes) {
            log::error!("update install failed: {e}");
        }
    }
}
