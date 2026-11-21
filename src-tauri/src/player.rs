//! The player: one libmpv instance for the whole life of the app (libmpv
//! with several WASAPI players in one process crashes at exit), rendering into
//! the main window's video host. Owns automatic subtitles, resume, folder
//! queueing in natural order, watch progress and readable errors.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde_json::{json, Map, Value};
use tauri::{AppHandle, Emitter};

use crate::core::subtitles;
use crate::core::{formats, languages, natsort};
use crate::engine;
use crate::library::Library;
use crate::mpv::{self, Event, Mpv};
use crate::paths;
use crate::settings::Settings;

const PROPS: &[&str] = &[
    "pause", "time-pos", "duration", "volume", "mute", "speed", "track-list", "chapter-list", "chapter", "vid", "aid", "sid",
    "secondary-sid", "sub-delay", "audio-delay", "sub-scale", "sub-pos", "sub-visibility", "video-params", "hwdec-current",
    "paused-for-cache", "cache-buffering-state", "seeking", "eof-reached", "idle-active", "playlist", "playlist-pos",
    "media-title", "path", "ab-loop-a", "ab-loop-b", "loop-file", "loop-playlist", "video-aspect-override", "video-rotate",
    "video-zoom", "deinterlace", "demuxer-cache-time", "audio-device", "audio-device-list", "estimated-vf-fps",
    "container-fps", "file-format", "audio-params", "video-bitrate", "audio-bitrate", "frame-drop-count",
    "decoder-frame-drop-count", "file-size", "metadata", "video-format", "video-codec", "audio-codec-name", "current-vo",
    "playback-abort", "core-idle",
];

/// Properties that change many times a second and the minimum gap between
/// two updates sent to the interface.
fn throttle_of(name: &str) -> Option<Duration> {
    match name {
        "time-pos" => Some(Duration::from_millis(200)),
        "demuxer-cache-time" | "video-bitrate" | "audio-bitrate" | "estimated-vf-fps" | "frame-drop-count" | "decoder-frame-drop-count" => {
            Some(Duration::from_millis(1000))
        }
        _ => None,
    }
}

const HOOK_LOAD: u64 = 1;
const HOOK_PRELOADED: u64 = 2;

#[derive(Default)]
struct State {
    path: String,
    title: String,
    pos: f64,
    duration: f64,
    loaded: bool,
    has_video: bool,
    last_save: Option<Instant>,
    last_emit: HashMap<String, Instant>,
    pending: HashMap<String, Value>,
    props: Map<String, Value>,
    sub_labels: HashMap<String, String>,
}

pub struct Player {
    pub mpv: Arc<Mpv>,
    app: AppHandle,
    settings: Arc<Settings>,
    library: Arc<Library>,
    st: Mutex<State>,
    autoplay_cancelled: AtomicBool,
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

impl Player {
    pub fn start(app: AppHandle, settings: Arc<Settings>, library: Arc<Library>, wid: isize) -> Result<Arc<Player>, String> {
        mpv::load(&paths::libmpv())?;
        let m = Mpv::create()?;
        for (k, v) in engine::init_options(&settings, Some(wid)) {
            if let Err(e) = m.set_option(&k, &v) {
                log::warn!("{e}");
            }
        }
        m.initialize()?;
        let _ = m.request_log("warn");
        for (i, p) in PROPS.iter().enumerate() {
            let _ = m.observe(100 + i as u64, p);
        }
        m.hook_add(HOOK_LOAD, "on_load", 50)?;
        m.hook_add(HOOK_PRELOADED, "on_preloaded", 50)?;
        let p = Arc::new(Player {
            mpv: Arc::new(m),
            app,
            settings,
            library,
            st: Mutex::new(State::default()),
            autoplay_cancelled: AtomicBool::new(false),
        });
        let me = p.clone();
        std::thread::Builder::new().name("mpv-events".into()).spawn(move || me.event_loop()).map_err(|e| e.to_string())?;
        let me = p.clone();
        std::thread::Builder::new().name("mpv-flush".into()).spawn(move || me.flush_loop()).map_err(|e| e.to_string())?;
        Ok(p)
    }

    fn emit_event(&self, v: Value) {
        let _ = self.app.emit("player://event", v);
    }

    fn emit_prop(&self, name: &str, value: &Value) {
        let _ = self.app.emit("player://prop", json!({ "name": name, "value": value }));
    }

    /// Sends throttled property values that were held back.
    fn flush_loop(self: Arc<Self>) {
        loop {
            std::thread::sleep(Duration::from_millis(120));
            let due: Vec<(String, Value)> = {
                let mut st = self.st.lock();
                let now = Instant::now();
                let names: Vec<String> = st.pending.keys().cloned().collect();
                let mut out = Vec::new();
                for n in names {
                    let gap = throttle_of(&n).unwrap_or_default();
                    let ok = st.last_emit.get(&n).map(|t| now.duration_since(*t) >= gap).unwrap_or(true);
                    if ok {
                        if let Some(v) = st.pending.remove(&n) {
                            st.last_emit.insert(n.clone(), now);
                            out.push((n, v));
                        }
                    }
                }
                out
            };
            for (n, v) in due {
                self.emit_prop(&n, &v);
            }
        }
    }

    fn event_loop(self: Arc<Self>) {
        loop {
            match self.mpv.wait_event(-1.0) {
                Event::Shutdown => break,
                Event::Property { name, value, .. } => self.on_prop(name, value),
                Event::Hook { user, id, .. } => {
                    match user {
                        HOOK_LOAD => self.on_load(),
                        HOOK_PRELOADED => self.on_preloaded(),
                        _ => {}
                    }
                    self.mpv.hook_continue(id);
                }
                Event::StartFile { .. } => {
                    {
                        let mut st = self.st.lock();
                        st.loaded = false;
                        st.pos = 0.0;
                        st.duration = 0.0;
                        st.last_save = None;
                    }
                    self.autoplay_cancelled.store(false, Ordering::Release);
                    let keep = if self.settings.bool("playback.autoplay_next") { "yes" } else { "always" };
                    let _ = self.mpv.set_property("keep-open", keep);
                    self.emit_event(json!({"type": "start-file"}));
                }
                Event::FileLoaded => self.on_file_loaded(),
                Event::EndFile { reason, error, .. } => self.on_end_file(reason, error),
                Event::PlaybackRestart => self.emit_event(json!({"type": "restart"})),
                Event::Seek => self.emit_event(json!({"type": "seek"})),
                Event::Log { prefix, level, text } => {
                    let t = text.trim_end();
                    if level == "error" || level == "fatal" {
                        log::warn!("mpv[{prefix}] {t}");
                    } else {
                        log::debug!("mpv[{prefix}] {t}");
                    }
                }
                _ => {}
            }
        }
    }

    fn on_prop(&self, name: String, value: Value) {
        let throttled = throttle_of(&name);
        let send_now = {
            let mut st = self.st.lock();
            st.props.insert(name.clone(), value.clone());
            match name.as_str() {
                "time-pos" => st.pos = value.as_f64().unwrap_or(st.pos),
                "duration" => st.duration = value.as_f64().unwrap_or(0.0),
                "media-title" => st.title = s(&value),
                "path" => st.path = s(&value),
                "vid" => st.has_video = value.as_i64().is_some(),
                _ => {}
            }
            match throttled {
                None => true,
                Some(gap) => {
                    let now = Instant::now();
                    let ok = st.last_emit.get(&name).map(|t| now.duration_since(*t) >= gap).unwrap_or(true);
                    if ok {
                        st.last_emit.insert(name.clone(), now);
                        st.pending.remove(&name);
                    } else {
                        st.pending.insert(name.clone(), value.clone());
                    }
                    ok
                }
            }
        };
        if send_now {
            self.emit_prop(&name, &value);
        }
        match name.as_str() {
            "time-pos" => self.maybe_save_progress(),
            "idle-active" if value.as_bool() == Some(true) => {
                self.library.player_busy.store(false, Ordering::Release);
                self.emit_event(json!({"type": "idle"}));
            }
            "pause" if value.as_bool() == Some(true) => self.save_progress(false),
            _ => {}
        }
    }

    fn maybe_save_progress(&self) {
        let due = {
            let st = self.st.lock();
            st.loaded && st.last_save.map(|t| t.elapsed() > Duration::from_secs(5)).unwrap_or(true)
        };
        if due {
            self.save_progress(false);
        }
    }

    pub fn save_progress(&self, finished: bool) {
        let (path, title, pos, dur) = {
            let mut st = self.st.lock();
            if !st.loaded || st.path.is_empty() {
                return;
            }
            st.last_save = Some(Instant::now());
            (st.path.clone(), st.title.clone(), st.pos, st.duration)
        };
        let near_end = dur > 0.0 && (pos >= dur * 0.95 || (dur > 600.0 && dur - pos < 30.0));
        let done = finished || near_end;
        self.library.set_progress(&path, &title, if done { 0.0 } else { pos }, dur, done, false);
    }

    // ---- hooks --------------------------------------------------------------

    fn on_load(&self) {
        let path = self.mpv.get_string("path").unwrap_or_default();
        if path.is_empty() || formats::is_url(&path) {
            return;
        }
        let mode = self.settings.str("playback.resume");
        if mode == "never" {
            return;
        }
        let Some(p) = self.library.progress_of(&path) else { return };
        let ok = !p.finished && p.pos > 15.0 && (p.duration <= 0.0 || p.pos < p.duration - 20.0);
        if !ok {
            return;
        }
        if mode == "auto" {
            let _ = self.mpv.set_property("file-local-options/start", &format!("{:.3}", p.pos));
            self.emit_event(json!({"type": "resumed", "pos": p.pos}));
        } else {
            self.emit_event(json!({"type": "resume-available", "pos": p.pos}));
        }
    }

    fn on_preloaded(&self) {
        let path = self.mpv.get_string("path").unwrap_or_default();
        if !path.is_empty() && !formats::is_url(&path) && self.settings.bool("subs.auto_load") {
            let cache = paths::cache_dir("subs");
            let found = subtitles::find_external(Path::new(&path), &cache, true);
            let persian = self.settings.bool("subs.persian_fix");
            let rtl = self.settings.bool("subs.rtl_ass_auto");
            for c in found.iter().take(16) {
                let prepared = subtitles::prepare_for_playback(Path::new(&c.path), c.lang.as_deref(), &cache, persian, rtl);
                let title = match &c.lang {
                    Some(l) => {
                        let mut t = languages::english_name(Some(l));
                        if c.forced {
                            t.push_str(" (Forced)");
                        }
                        t
                    }
                    None => c.label.clone(),
                };
                let lang = c.lang.clone().unwrap_or_default();
                if self.mpv.command(&["sub-add", &prepared, "auto", &title, &lang]).is_ok() {
                    self.st.lock().sub_labels.insert(prepared.clone(), c.label.clone());
                }
            }
        }
        if self.settings.bool("subs.auto_select") {
            let tracks = self.mpv.get("track-list").and_then(|v| v.as_array().cloned()).unwrap_or_default();
            let choice = subtitles::choose_tracks(&tracks, &self.settings.strings("audio.languages"), &self.settings.strings("subs.languages"), true);
            if let Some(a) = choice.aid {
                let _ = self.mpv.set_property("aid", &a.to_string());
            }
            match choice.sid {
                Some(id) => {
                    let _ = self.mpv.set_property("sid", &id.to_string());
                }
                None => {
                    let _ = self.mpv.set_property("sid", "no");
                }
            }
        }
    }

    fn on_file_loaded(&self) {
        let path = self.mpv.get_string("path").unwrap_or_default();
        let title = self.mpv.get_string("media-title").unwrap_or_default();
        let duration = self.mpv.get_f64("duration").unwrap_or(0.0);
        let has_video = self.mpv.get("vid").map(|v| v.is_i64()).unwrap_or(false);
        {
            let mut st = self.st.lock();
            st.loaded = true;
            st.path = path.clone();
            st.title = title.clone();
            st.duration = duration;
            st.has_video = has_video;
        }
        self.library.player_busy.store(has_video, Ordering::Release);
        // A readable title for files outside the library ("The Upside", "2017")
        if !formats::is_url(&path) {
            let n = crate::core::naming::parse(Path::new(&path));
            let (t, sub) = match (n.kind, &n.show) {
                ("episode", Some(show)) => (
                    show.clone(),
                    [n.episode_tag().unwrap_or_default(), n.episode_title.clone().unwrap_or_default()].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" · "),
                ),
                _ => (n.title.clone(), n.year.map(|y| y.to_string()).unwrap_or_default()),
            };
            let tag_title = !title.is_empty() && title != Path::new(&path).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
            let v = json!({ "title": if tag_title && n.kind != "episode" { title.clone() } else { t }, "sub": sub });
            self.st.lock().props.insert("medore-title".into(), v.clone());
            self.emit_prop("medore-title", &v);
        }
        let pos = self.mpv.get_f64("time-pos").unwrap_or(0.0);
        self.library.set_progress(&path, &title, pos, duration, false, true);
        self.emit_event(json!({"type": "file-loaded", "path": path, "title": title, "hasVideo": has_video}));
    }

    fn on_end_file(&self, reason: i32, error: i32) {
        match reason {
            mpv::ffi::END_FILE_REASON_EOF => self.save_progress(true),
            mpv::ffi::END_FILE_REASON_ERROR => {
                let path = self.st.lock().path.clone();
                let path = if path.is_empty() { self.mpv.get_string("path").unwrap_or_default() } else { path };
                let key = humanize_error(error, &path);
                log::warn!("cannot play {path}: mpv error {error} -> {key}");
                self.emit_event(json!({"type": "error", "key": key, "path": path, "code": error}));
            }
            _ => self.save_progress(false),
        }
        self.st.lock().loaded = false;
    }

    // ---- commands -------------------------------------------------------------

    /// Open files, folders, playlists or URLs.
    /// `mode`: replace | append | next. `sort`: order the given files naturally
    /// (used for drops and Explorer multi-select, which arrive unsorted).
    pub fn open(&self, inputs: Vec<String>, start: usize, mode: &str, sort: bool) -> Result<usize, String> {
        let mut list: Vec<String> = Vec::new();
        for i in inputs {
            let p = PathBuf::from(&i);
            if !formats::is_url(&i) && p.is_dir() {
                let mut files = Vec::new();
                collect_media(&p, 0, &mut files);
                files.sort_by(|a, b| natsort::compare(&a.to_string_lossy(), &b.to_string_lossy()));
                list.extend(files.into_iter().map(|f| f.to_string_lossy().into_owned()));
            } else {
                list.push(i);
            }
        }
        if list.is_empty() {
            return Err("nothing to play".into());
        }
        let mut start = start.min(list.len() - 1);
        if sort && list.len() > 1 {
            let chosen = list[start].clone();
            list.sort_by(|a, b| natsort::compare(file_name(a), file_name(b)));
            start = list.iter().position(|x| *x == chosen).unwrap_or(0);
        }
        // Opening one local episode, personal video or song queues the rest of
        // its folder in natural order (not movies: a Movies folder holds
        // unrelated films that should not play into each other).
        if mode == "replace" && list.len() == 1 && self.settings.bool("playback.auto_queue_folder") {
            let one = PathBuf::from(&list[0]);
            let kind = crate::core::naming::parse(&one).kind;
            let queue_kind = formats::is_audio(&one) || kind == "episode" || kind == "video";
            if queue_kind && !formats::is_url(&list[0]) && formats::is_media(&one) && !formats::is_playlist(&one) {
                if let Some(dir) = one.parent() {
                    let audio = formats::is_audio(&one);
                    let mut sib: Vec<String> = std::fs::read_dir(dir)
                        .map(|rd| {
                            rd.flatten()
                                .map(|e| e.path())
                                .filter(|p| p.is_file() && if audio { formats::is_audio(p) } else { formats::is_video(p) })
                                .filter(|p| !crate::core::naming::parse(p).extra || p == &one)
                                .map(|p| p.to_string_lossy().into_owned())
                                .collect()
                        })
                        .unwrap_or_default();
                    if sib.len() > 1 && sib.len() <= 500 {
                        sib.sort_by(|a, b| natsort::compare(file_name(a), file_name(b)));
                        if let Some(i) = sib.iter().position(|x| Path::new(x) == one) {
                            list = sib;
                            start = i;
                        }
                    }
                }
            }
        }
        match mode {
            "append" => {
                for f in &list {
                    self.mpv.command(&["loadfile", f, "append-play"])?;
                }
            }
            "next" => {
                for f in list.iter().rev() {
                    self.mpv.command(&["loadfile", f, "insert-next"])?;
                }
            }
            _ => {
                self.save_progress(false);
                self.mpv.command(&["loadfile", &list[start], "replace"])?;
                for (i, f) in list.iter().enumerate().take(start) {
                    self.mpv.command(&["loadfile", f, "insert-at", &i.to_string()])?;
                }
                for f in list.iter().skip(start + 1) {
                    self.mpv.command(&["loadfile", f, "append"])?;
                }
                let _ = self.mpv.set_property("pause", "no");
            }
        }
        Ok(list.len())
    }

    pub fn add_subtitle(&self, path: &str) -> Result<(), String> {
        let lang = subtitles::tags_from_name(&Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(), None)
            .0
            .or_else(|| subtitles::sniff_language(Path::new(path)).map(str::to_string));
        let prepared = subtitles::prepare_for_playback(
            Path::new(path),
            lang.as_deref(),
            &paths::cache_dir("subs"),
            self.settings.bool("subs.persian_fix"),
            self.settings.bool("subs.rtl_ass_auto"),
        );
        let title = Path::new(path).file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        self.mpv.command(&["sub-add", &prepared, "select", &title, lang.as_deref().unwrap_or("")])
    }

    pub fn snapshot(&self) -> Value {
        Value::Object(self.st.lock().props.clone())
    }

    pub fn cancel_autoplay(&self) {
        self.autoplay_cancelled.store(true, Ordering::Release);
        let _ = self.mpv.set_property("keep-open", "always");
    }

    pub fn playlist_sort(&self, by: &str) -> Result<(), String> {
        let pl = self.mpv.get("playlist").and_then(|v| v.as_array().cloned()).unwrap_or_default();
        if pl.len() < 2 {
            return Ok(());
        }
        let mut names: Vec<String> = pl.iter().map(|e| s(&e["filename"])).collect();
        match by {
            "name" => names.sort_by(|a, b| natsort::compare(file_name(a), file_name(b))),
            "name-desc" => names.sort_by(|a, b| natsort::compare(file_name(b), file_name(a))),
            "date" | "date-desc" => {
                let mtime = |p: &String| std::fs::metadata(p).and_then(|m| m.modified()).ok();
                names.sort_by_key(mtime);
                if by == "date-desc" {
                    names.reverse();
                }
            }
            "reverse" => names.reverse(),
            "shuffle" => {
                // Fisher-Yates with a time-seeded xorshift; keeps the playing item first
                let cur = pl.iter().position(|e| e["current"].as_bool().unwrap_or(false)).unwrap_or(0);
                let playing = names.remove(cur);
                let mut x = (crate::core::now_secs() as u64) ^ 0x9E3779B97F4A7C15;
                for i in (1..names.len()).rev() {
                    x ^= x << 13;
                    x ^= x >> 7;
                    x ^= x << 17;
                    names.swap(i, (x % (i as u64 + 1)) as usize);
                }
                names.insert(0, playing);
            }
            _ => return Err(format!("unknown order {by}")),
        }
        // apply with moves: bring each wanted entry into place
        let mut current: Vec<String> = pl.iter().map(|e| s(&e["filename"])).collect();
        for (i, want) in names.iter().enumerate() {
            if let Some(j) = current.iter().skip(i).position(|x| x == want).map(|k| k + i) {
                if j != i {
                    self.mpv.command(&["playlist-move", &j.to_string(), &i.to_string()])?;
                    let item = current.remove(j);
                    current.insert(i, item);
                }
            }
        }
        Ok(())
    }

    pub fn playlist_move(&self, from: usize, to: usize) -> Result<(), String> {
        if from == to {
            return Ok(());
        }
        let target = if from < to { to + 1 } else { to };
        self.mpv.command(&["playlist-move", &from.to_string(), &target.to_string()])
    }

    pub fn screenshot(&self, subtitles: bool) -> Result<String, String> {
        let dir = {
            let d = self.settings.str("playback.screenshot_dir");
            if d.is_empty() {
                engine::default_screenshot_dir()
            } else {
                d
            }
        };
        let _ = std::fs::create_dir_all(&dir);
        let (title, pos) = {
            let st = self.st.lock();
            (st.title.clone(), st.pos)
        };
        let clean: String = title.chars().map(|c| if "\\/:*?\"<>|".contains(c) { ' ' } else { c }).collect::<String>().trim().chars().take(80).collect();
        let t = pos as i64;
        let ext = match self.settings.str("playback.screenshot_format").as_str() {
            "jpg" => "jpg",
            "webp" => "webp",
            _ => "png",
        };
        let mut file = PathBuf::from(&dir).join(format!("MEDORE {} {:02}-{:02}-{:02}.{ext}", clean, t / 3600, t / 60 % 60, t % 60));
        let mut n = 2;
        while file.exists() {
            file = PathBuf::from(&dir).join(format!("MEDORE {} {:02}-{:02}-{:02} ({n}).{ext}", clean, t / 3600, t / 60 % 60, t % 60));
            n += 1;
        }
        let fs = file.to_string_lossy().into_owned();
        self.mpv.command(&["screenshot-to-file", &fs, if subtitles { "subtitles" } else { "video" }])?;
        Ok(fs)
    }

    pub fn apply_setting(&self, key: &str) {
        for (k, v) in engine::runtime_updates(&self.settings, key) {
            if let Err(e) = self.mpv.set_property(&k, &v) {
                log::debug!("runtime setting {k}={v}: {e}");
            }
        }
    }

    pub fn shutdown(&self) {
        self.save_progress(false);
        let vol = self.mpv.get_f64("volume");
        if let Some(v) = vol {
            let _ = self.settings.set("playback.volume", json!(v));
        }
        let _ = self.mpv.command(&["quit"]);
    }
}

fn file_name(p: &str) -> &str {
    p.rsplit(['\\', '/']).next().unwrap_or(p)
}

fn collect_media(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 6 || out.len() > 5000 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() && !ft.is_symlink() {
            collect_media(&p, depth + 1, out);
        } else if ft.is_file() && formats::is_media(&p) && !crate::core::naming::parse(&p).extra {
            out.push(p);
        }
    }
}

/// A short key the interface turns into a sentence in the person's language.
pub fn humanize_error(code: i32, path: &str) -> &'static str {
    if formats::is_url(path) {
        return "error.network";
    }
    if !path.is_empty() && !Path::new(path).exists() {
        return if path.starts_with("\\\\") { "error.share_offline" } else { "error.missing" };
    }
    if let Ok(m) = std::fs::metadata(path) {
        if m.len() == 0 {
            return "error.empty";
        }
    }
    match code {
        -17 | -18 => "error.format",
        -13 => "error.corrupt",
        -14 => "error.audio_device",
        -15 => "error.video_output",
        _ => "error.generic",
    }
}
