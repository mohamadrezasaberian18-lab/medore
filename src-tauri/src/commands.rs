//! Commands the interface calls (via `invoke`).

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::player::Player;
use crate::AppState;

type R<T> = Result<T, String>;

/// libmpv must never be called on the main window thread (its video window
/// lives inside our window: a blocking call there can deadlock with it), so
/// player commands run on worker threads and hand window changes back here.
fn on_main(app: &AppHandle, f: impl FnOnce(&AppState) + Send + 'static) {
    let h = app.clone();
    let _ = app.run_on_main_thread(move || f(&h.state::<AppState>()));
}

fn player(st: &AppState) -> R<&std::sync::Arc<Player>> {
    st.player.as_ref().ok_or_else(|| st.player_error.clone().unwrap_or_else(|| "player unavailable".into()))
}

static READY: (parking_lot::Mutex<bool>, parking_lot::Condvar) = (parking_lot::Mutex::new(false), parking_lot::Condvar::new());

/// Called once setup has registered all state.
pub fn mark_ready() {
    *READY.0.lock() = true;
    READY.1.notify_all();
}

/// The web pages load in parallel with setup; they wait here before using
/// any other command (otherwise a cold start can show a blank window).
#[tauri::command]
pub async fn wait_ready() {
    let _ = tauri::async_runtime::spawn_blocking(|| {
        let mut g = READY.0.lock();
        while !*g {
            READY.1.wait_for(&mut g, std::time::Duration::from_millis(250));
        }
    })
    .await;
}

#[tauri::command]
pub fn js_log(level: String, msg: String) {
    match level.as_str() {
        "error" => log::error!(target: "ui", "{msg}"),
        "warn" => log::warn!(target: "ui", "{msg}"),
        _ => log::info!(target: "ui", "{msg}"),
    }
}

#[tauri::command]
pub fn app_info(st: State<AppState>) -> Value {
    let p = crate::paths::get();
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "portable": p.portable,
        "dataDir": p.data.to_string_lossy(),
        "playerError": st.player_error,
        "lightSystem": crate::icons::system_light(),
    })
}

// ---- settings --------------------------------------------------------------

#[tauri::command]
pub fn settings_all(st: State<AppState>) -> Value {
    Value::Object(st.settings.all())
}

#[tauri::command(async)]
pub fn settings_set(app: AppHandle, st: State<'_, AppState>, key: String, value: Value) -> R<Value> {
    let v = st.settings.set(&key, value)?;
    let _ = app.emit("settings://changed", json!({"key": key, "value": v}));
    if let Some(p) = st.player.as_ref() {
        p.apply_setting(&key);
    }
    match key.as_str() {
        "library.folders" => st.library.rescan(),
        "library.posters_online" | "library.thumbnails" => st.library.rescan(),
        _ => {}
    }
    Ok(v)
}

#[tauri::command(async)]
pub fn settings_reset(app: AppHandle, st: State<'_, AppState>, key: String) -> R<Value> {
    let v = st.settings.reset(&key)?;
    let _ = app.emit("settings://changed", json!({"key": key, "value": v}));
    if let Some(p) = st.player.as_ref() {
        p.apply_setting(&key);
    }
    Ok(v)
}

// ---- library ---------------------------------------------------------------

#[tauri::command]
pub fn library_snapshot(st: State<AppState>) -> Value {
    st.library.snapshot()
}

#[tauri::command]
pub fn library_status(st: State<AppState>) -> Value {
    st.library.status()
}

#[tauri::command]
pub fn library_rescan(st: State<AppState>) {
    st.library.rescan();
}

#[tauri::command(async)]
pub fn library_add_folder(app: AppHandle, st: State<'_, AppState>, path: String) -> R<Value> {
    let mut f = st.settings.strings("library.folders");
    if !f.iter().any(|x| x.eq_ignore_ascii_case(&path)) {
        f.push(path);
    }
    settings_set(app, st, "library.folders".into(), json!(f))
}

#[tauri::command(async)]
pub fn library_remove_folder(app: AppHandle, st: State<'_, AppState>, path: String) -> R<Value> {
    let f: Vec<String> = st.settings.strings("library.folders").into_iter().filter(|x| !x.eq_ignore_ascii_case(&path)).collect();
    settings_set(app, st, "library.folders".into(), json!(f))
}

#[tauri::command]
pub fn library_set_watched(st: State<AppState>, ids: Vec<String>, watched: bool) {
    st.library.set_watched(&ids, watched);
}

#[tauri::command]
pub fn library_forget(st: State<AppState>, ids: Vec<String>) {
    st.library.forget_progress(&ids);
}

#[tauri::command]
pub fn library_clear_history(st: State<AppState>) {
    st.library.clear_history();
}

// ---- player ------------------------------------------------------------------

#[tauri::command(async)]
pub fn player_open(app: AppHandle, st: State<'_, AppState>, paths: Vec<String>, start: Option<usize>, mode: Option<String>, sort: Option<bool>) -> R<usize> {
    let mode = mode.unwrap_or_else(|| "replace".into());
    let n = player(&st)?.open(paths, start.unwrap_or(0), &mode, sort.unwrap_or(false))?;
    if mode == "replace" {
        on_main(&app, |st| st.ui.set_player(true));
    }
    Ok(n)
}

#[tauri::command(async)]
pub fn player_command(st: State<'_, AppState>, args: Vec<String>) -> R<()> {
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    player(&st)?.mpv.command(&a)
}

#[tauri::command(async)]
pub fn player_set(st: State<'_, AppState>, name: String, value: Value) -> R<()> {
    let p = player(&st)?;
    p.mpv.set_property_value(&name, &value)?;
    if name == "volume" {
        if let Some(v) = value.as_f64() {
            let _ = st.settings.set("playback.volume", json!(v));
        }
    }
    Ok(())
}

#[tauri::command(async)]
pub fn player_get(st: State<'_, AppState>, name: String) -> R<Value> {
    Ok(player(&st)?.mpv.get(&name).unwrap_or(Value::Null))
}

#[tauri::command]
pub fn player_snapshot(st: State<AppState>) -> R<Value> {
    Ok(player(&st)?.snapshot())
}

#[tauri::command(async)]
pub fn player_stop(app: AppHandle, st: State<'_, AppState>) -> R<()> {
    let p = player(&st)?;
    p.save_progress(false);
    let _ = p.mpv.command(&["stop"]);
    on_main(&app, |st| st.ui.set_player(false));
    Ok(())
}

#[tauri::command(async)]
pub fn player_add_subtitle(st: State<'_, AppState>, path: String) -> R<()> {
    player(&st)?.add_subtitle(&path)
}

#[tauri::command(async)]
pub fn player_screenshot(st: State<'_, AppState>, subtitles: bool) -> R<String> {
    player(&st)?.screenshot(subtitles)
}

#[tauri::command(async)]
pub fn player_cancel_autoplay(st: State<'_, AppState>) -> R<()> {
    player(&st)?.cancel_autoplay();
    Ok(())
}

#[tauri::command(async)]
pub fn playlist_sort(st: State<'_, AppState>, by: String) -> R<()> {
    player(&st)?.playlist_sort(&by)
}

#[tauri::command(async)]
pub fn playlist_move(st: State<'_, AppState>, from: usize, to: usize) -> R<()> {
    player(&st)?.playlist_move(from, to)
}

#[tauri::command]
pub async fn preview_frame(path: String, t: f64, duration: f64) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || crate::previews().request(&path, t, duration)).await.ok().flatten()
}

// ---- windows -------------------------------------------------------------------

#[tauri::command]
pub fn ui_state(st: State<AppState>) -> Value {
    json!({ "player": st.ui.in_player(), "fullscreen": st.ui.fullscreen(), "pip": st.ui.pip() })
}

#[tauri::command]
pub fn ui_player(st: State<AppState>, on: bool) {
    st.ui.set_player(on);
}

#[tauri::command]
pub fn ui_fullscreen(st: State<AppState>, on: Option<bool>) -> bool {
    let target = on.unwrap_or(!st.ui.fullscreen());
    st.ui.set_fullscreen(target);
    target
}

#[tauri::command]
pub fn ui_pip(st: State<AppState>, on: Option<bool>) -> bool {
    let target = on.unwrap_or(!st.ui.pip());
    st.ui.set_pip(target);
    target
}

#[tauri::command]
pub fn ui_on_top(st: State<AppState>, on: bool) {
    st.ui.set_on_top(on);
}

#[tauri::command]
pub fn ui_window(app: AppHandle, action: String) {
    if let Some(m) = app.get_webview_window("main") {
        let _ = match action.as_str() {
            "minimize" => m.minimize(),
            "toggle-maximize" => {
                if m.is_maximized().unwrap_or(false) {
                    m.unmaximize()
                } else {
                    m.maximize()
                }
            }
            "close" => m.close(),
            "focus" => m.set_focus(),
            _ => Ok(()),
        };
    }
}

// ---- system ------------------------------------------------------------------

#[tauri::command]
pub fn reveal(path: String) -> R<()> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err("not found".into());
    }
    std::process::Command::new("explorer").arg(format!("/select,{}", p.display())).spawn().map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_url(url: String) -> R<()> {
    if !url.starts_with("https://") {
        return Err("only https links".into());
    }
    std::process::Command::new("explorer").arg(url).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Installed font family names (for the subtitle font picker).
#[tauri::command]
pub fn fonts() -> Vec<String> {
    #[allow(unused_assignments, unused_mut)]
    let mut out: Vec<String> = Vec::new();
    #[cfg(windows)]
    {
        use std::collections::BTreeSet;
        let mut set = BTreeSet::new();
        for root in [r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts", r"HKCU\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts"] {
            if let Ok(o) = std::process::Command::new("reg").args(["query", root]).creation_flags_hidden().output() {
                for line in String::from_utf8_lossy(&o.stdout).lines() {
                    let line = line.trim();
                    let Some((name, _)) = line.split_once("    REG_") else { continue };
                    let name = name.trim();
                    let base = name.split(" (").next().unwrap_or(name);
                    for part in base.split(" & ") {
                        let fam = part
                            .trim()
                            .trim_end_matches(" Bold Italic")
                            .trim_end_matches(" Bold")
                            .trim_end_matches(" Italic")
                            .trim_end_matches(" Light")
                            .trim_end_matches(" Semibold")
                            .trim_end_matches(" SemiBold")
                            .trim_end_matches(" Regular")
                            .trim_end_matches(" Black")
                            .trim();
                        if !fam.is_empty() && !fam.starts_with('@') {
                            set.insert(fam.to_string());
                        }
                    }
                }
            }
        }
        out = set.into_iter().collect();
    }
    out
}

#[tauri::command]
pub async fn update_check(app: AppHandle) -> R<Value> {
    crate::updates::check(app, true).await
}

#[tauri::command]
pub fn update_install(app: AppHandle) -> R<()> {
    crate::updates::install_now(&app)
}

trait HiddenCmd {
    fn creation_flags_hidden(&mut self) -> &mut Self;
}
impl HiddenCmd for std::process::Command {
    fn creation_flags_hidden(&mut self) -> &mut Self {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            self.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        self
    }
}
