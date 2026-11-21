#![recursion_limit = "512"]

mod commands;
pub use medore_core as core;
mod engine;
mod icons;
mod library;
mod logging;
pub mod mpv;
mod paths;
mod player;
mod posters;
mod settings;
mod thumbs;
mod ui;
mod updates;
#[cfg(windows)]
mod win;

use std::sync::{Arc, OnceLock};

use serde_json::json;
use tauri::{Emitter, Manager, RunEvent, WindowEvent};

use library::Library;
use player::Player;
use settings::Settings;

pub struct AppState {
    pub settings: Arc<Settings>,
    pub library: Arc<Library>,
    pub player: Option<Arc<Player>>,
    pub player_error: Option<String>,
    pub ui: ui::Ui,
}

impl AppState {
    /// Save everything before MEDORE closes or restarts.
    pub fn before_exit(&self) {
        if let Some(p) = &self.player {
            p.shutdown();
        }
        self.library.flush();
    }
}

static PREVIEWS: OnceLock<thumbs::Previews> = OnceLock::new();
pub fn previews() -> &'static thumbs::Previews {
    PREVIEWS.get_or_init(thumbs::Previews::new)
}

/// Files and URLs on a command line (Explorer "Open with", drag onto the exe).
fn files_from_args(args: &[String]) -> (Vec<String>, bool) {
    let mut files = Vec::new();
    let mut enqueue = false;
    for a in args.iter().skip(1) {
        match a.as_str() {
            "--enqueue" => enqueue = true,
            s if s.starts_with("--") => {}
            s => files.push(s.to_string()),
        }
    }
    (files, enqueue)
}

fn open_from_args(app: &tauri::AppHandle, args: Vec<String>) {
    let (files, enqueue) = files_from_args(&args);
    let st = app.state::<AppState>();
    if files.is_empty() {
        if let Some(m) = app.get_webview_window("main") {
            let _ = m.unminimize();
            let _ = m.set_focus();
        }
        return;
    }
    if let Some(p) = st.player.clone() {
        // libmpv calls never run on the main window thread (see commands::on_main)
        let h = app.clone();
        std::thread::spawn(move || {
            let subs: Vec<&String> = files.iter().filter(|f| core::formats::is_subtitle(std::path::Path::new(f))).collect();
            if !subs.is_empty() && subs.len() == files.len() {
                for s in subs {
                    let _ = p.add_subtitle(s);
                }
                return;
            }
            let mode = if enqueue { "append" } else { "replace" };
            match p.open(files, 0, mode, true) {
                Ok(_) if !enqueue => {
                    let h2 = h.clone();
                    let _ = h.run_on_main_thread(move || h2.state::<AppState>().ui.set_player(true));
                }
                Ok(_) => {}
                Err(e) => log::warn!("cannot open from command line: {e}"),
            }
        });
    }
    if let Some(m) = app.get_webview_window("main") {
        let _ = m.unminimize();
        let _ = m.set_focus();
    }
}

fn restore_window(app: &tauri::AppHandle, settings: &Settings) {
    let Some(m) = app.get_webview_window("main") else { return };
    let w = settings.get("general.window");
    let (Some(x), Some(y), Some(wd), Some(ht)) = (w["x"].as_i64(), w["y"].as_i64(), w["w"].as_u64(), w["h"].as_u64()) else { return };
    // only when the saved position is still on a connected screen
    let on_screen = m
        .available_monitors()
        .map(|mons| {
            mons.iter().any(|mon| {
                let p = mon.position();
                let s = mon.size();
                x + 80 > p.x as i64 && x < p.x as i64 + s.width as i64 - 80 && y >= p.y as i64 - 8 && y < p.y as i64 + s.height as i64 - 80
            })
        })
        .unwrap_or(false);
    if on_screen && wd >= 600 && ht >= 380 {
        let _ = m.set_size(tauri::PhysicalSize::new(wd as u32, ht as u32));
        let _ = m.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
    }
    if w["max"].as_bool() == Some(true) {
        let _ = m.maximize();
    }
}

fn save_window(app: &tauri::AppHandle) {
    let st = app.state::<AppState>();
    if st.ui.pip() || st.ui.fullscreen() {
        return;
    }
    let Some(m) = app.get_webview_window("main") else { return };
    let max = m.is_maximized().unwrap_or(false);
    let mut v = st.settings.get("general.window");
    if !max {
        if let (Ok(p), Ok(s)) = (m.outer_position(), m.inner_size()) {
            v = json!({"x": p.x, "y": p.y, "w": s.width, "h": s.height, "max": false});
        }
    } else if v.is_object() {
        v["max"] = json!(true);
    } else {
        v = json!({"max": true});
    }
    let _ = st.settings.set("general.window", v);
}

pub fn run() {
    paths::init();
    logging::init();
    log::info!("MEDORE {} starting", env!("CARGO_PKG_VERSION"));

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            open_from_args(app, argv);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(updates::Updates::default())
        .invoke_handler(tauri::generate_handler![
            commands::wait_ready,
            commands::js_log,
            commands::app_info,
            commands::settings_all,
            commands::settings_set,
            commands::settings_reset,
            commands::library_snapshot,
            commands::library_status,
            commands::library_rescan,
            commands::library_add_folder,
            commands::library_remove_folder,
            commands::library_set_watched,
            commands::library_forget,
            commands::library_clear_history,
            commands::player_open,
            commands::player_command,
            commands::player_set,
            commands::player_get,
            commands::player_snapshot,
            commands::player_stop,
            commands::player_add_subtitle,
            commands::player_screenshot,
            commands::player_cancel_autoplay,
            commands::playlist_sort,
            commands::playlist_move,
            commands::preview_frame,
            commands::ui_state,
            commands::ui_player,
            commands::ui_fullscreen,
            commands::ui_pip,
            commands::ui_on_top,
            commands::ui_window,
            commands::reveal,
            commands::open_url,
            commands::fonts,
            commands::update_check,
            commands::update_install,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let settings = Arc::new(Settings::load());
            first_run_defaults(&settings);
            restore_window(&handle, &settings);
            let ui = ui::Ui::create(&handle)?;
            let library = Library::load(handle.clone(), settings.clone());
            let (player, player_error) = match Player::start(handle.clone(), settings.clone(), library.clone(), ui.host) {
                Ok(p) => (Some(p), None),
                Err(e) => {
                    log::error!("player unavailable: {e}");
                    (None, Some(e))
                }
            };
            app.manage(AppState { settings: settings.clone(), library: library.clone(), player, player_error, ui });
            library.start();
            icons::apply(&handle);
            icons::watch(handle.clone());
            updates::start(handle.clone());

            let main = app.get_webview_window("main").expect("main window");
            let h2 = handle.clone();
            main.on_window_event(move |e| {
                let st = h2.state::<AppState>();
                match e {
                    WindowEvent::Resized(size) => {
                        st.ui.sync();
                        if size.width == 0 && size.height == 0 && st.settings.bool("playback.pause_on_minimize") {
                            if let Some(p) = st.player.clone() {
                                std::thread::spawn(move || {
                                    let _ = p.mpv.set_property("pause", "yes");
                                });
                            }
                        }
                    }
                    WindowEvent::Moved(_) | WindowEvent::ScaleFactorChanged { .. } => st.ui.sync(),
                    WindowEvent::CloseRequested { .. } => save_window(&h2),
                    WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) => {
                        let _ = h2.emit("app://dropped", paths.iter().map(|p| p.to_string_lossy().into_owned()).collect::<Vec<_>>());
                    }
                    _ => {}
                }
            });
            commands::mark_ready();
            let args: Vec<String> = std::env::args().collect();
            if files_from_args(&args).0.len() > 0 {
                let h3 = handle.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    let h4 = h3.clone();
                    let _ = h3.run_on_main_thread(move || open_from_args(&h4, args));
                });
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building MEDORE");

    app.run(|handle, event| match event {
        RunEvent::ExitRequested { .. } => {
            if let Some(st) = handle.try_state::<AppState>() {
                st.before_exit();
            }
        }
        RunEvent::Exit => {
            updates::install_on_exit(handle);
        }
        _ => {}
    });
}

/// On the very first start, add the usual video folders to the library.
fn first_run_defaults(settings: &Settings) {
    if settings.bool("general.first_run_done") {
        return;
    }
    let mut folders = Vec::new();
    if let Some(home) = std::env::var_os("USERPROFILE").map(std::path::PathBuf::from) {
        for sub in ["Videos", "Downloads"] {
            let p = home.join(sub);
            if p.is_dir() {
                folders.push(p.to_string_lossy().into_owned());
            }
        }
    }
    let _ = settings.set("library.folders", json!(folders));
    let _ = settings.set("general.first_run_done", json!(true));
}
