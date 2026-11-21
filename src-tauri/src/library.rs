//! Media library: folder scanning, background probing (duration, codecs,
//! thumbnails that skip black frames, album art, music tags), watch progress
//! and online posters. Data is kept in memory and saved as JSON.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::core::{formats, media_id, naming, now_secs};
use crate::mpv::render::FrameGrabber;
use crate::paths;
use crate::settings::Settings;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Item {
    pub id: String,
    pub path: String,
    pub root: String,
    pub kind: String,
    pub title: String,
    pub year: Option<i32>,
    pub show: Option<String>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub episode_end: Option<i32>,
    pub episode_title: Option<String>,
    pub extra: bool,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track: Option<i32>,
    pub genre: Option<String>,
    pub duration: f64,
    pub width: i64,
    pub height: i64,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub hdr: Option<String>,
    pub size: u64,
    pub mtime: i64,
    pub added: i64,
    pub thumb: Option<String>,
    pub probed: bool,
    pub probe_failed: bool,
    pub offline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Progress {
    pub path: String,
    pub title: String,
    pub pos: f64,
    pub duration: f64,
    pub updated: i64,
    pub finished: bool,
    pub plays: u32,
    pub thumb: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Poster {
    pub path: Option<String>,
    pub imdb: Option<String>,
    pub title: Option<String>,
    pub year: Option<i32>,
    pub fetched: i64,
    pub found: bool,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Data {
    items: HashMap<String, Item>,
    progress: HashMap<String, Progress>,
    posters: HashMap<String, Poster>,
}

pub struct Library {
    data: RwLock<Data>,
    dirty: AtomicBool,
    changed_at: AtomicU64,
    app: AppHandle,
    settings: Arc<Settings>,
    scanning: AtomicBool,
    pub player_busy: AtomicBool,
    wake: (Mutex<u64>, Condvar),
    status: Mutex<Value>,
}

const LIB_FILE: &str = "library.json";

fn mtime_secs(m: &fs::Metadata) -> i64 {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0)
}

pub fn poster_key_movie(title: &str, year: Option<i32>) -> String {
    format!("movie:{}:{}", title.to_lowercase(), year.map(|y| y.to_string()).unwrap_or_default())
}
pub fn poster_key_show(name: &str) -> String {
    format!("show:{}", name.to_lowercase())
}

impl Library {
    pub fn load(app: AppHandle, settings: Arc<Settings>) -> Arc<Library> {
        let path = paths::data_file(LIB_FILE);
        let data: Data = fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Arc::new(Library {
            data: RwLock::new(data),
            dirty: AtomicBool::new(false),
            changed_at: AtomicU64::new(0),
            app,
            settings,
            scanning: AtomicBool::new(false),
            player_busy: AtomicBool::new(false),
            wake: (Mutex::new(0), Condvar::new()),
            status: Mutex::new(json!({"state": "idle"})),
        })
    }

    pub fn start(self: &Arc<Self>) {
        // saver + change notifier
        let me = self.clone();
        std::thread::Builder::new()
            .name("library-saver".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_millis(700));
                let changed = me.changed_at.swap(0, Ordering::AcqRel);
                if changed != 0 {
                    let _ = me.app.emit("library://changed", ());
                }
                if me.dirty.swap(false, Ordering::AcqRel) {
                    me.save();
                }
            })
            .ok();
        // probe worker
        let me = self.clone();
        std::thread::Builder::new().name("library-probe".into()).spawn(move || me.probe_loop()).ok();
        // poster worker
        let me = self.clone();
        std::thread::Builder::new().name("library-posters".into()).spawn(move || me.poster_loop()).ok();
        if self.settings.bool("library.scan_on_start") {
            self.rescan();
        }
    }

    fn touch(&self) {
        self.dirty.store(true, Ordering::Release);
        self.changed_at.store(1, Ordering::Release);
    }

    pub fn save(&self) {
        let bytes = {
            let d = self.data.read();
            serde_json::to_vec(&*d).unwrap_or_default()
        };
        if let Err(e) = paths::write_atomic(&paths::data_file(LIB_FILE), &bytes) {
            log::error!("cannot save library: {e}");
        }
    }

    pub fn flush(&self) {
        if self.dirty.swap(false, Ordering::AcqRel) {
            self.save();
        }
    }

    fn set_status(&self, v: Value) {
        *self.status.lock() = v.clone();
        let _ = self.app.emit("library://status", v);
    }

    pub fn status(&self) -> Value {
        self.status.lock().clone()
    }

    fn notify_workers(&self) {
        let (m, c) = &self.wake;
        *m.lock() += 1;
        c.notify_all();
    }

    /// Everything the interface needs, in one object.
    pub fn snapshot(&self) -> Value {
        let d = self.data.read();
        let items: Vec<&Item> = d.items.values().collect();
        json!({ "items": items, "progress": d.progress, "posters": d.posters })
    }

    pub fn item_by_path(&self, path: &str) -> Option<Item> {
        self.data.read().items.get(&media_id(path)).cloned()
    }

    pub fn progress_of(&self, path: &str) -> Option<Progress> {
        self.data.read().progress.get(&media_id(path)).cloned()
    }

    pub fn set_progress(&self, path: &str, title: &str, pos: f64, duration: f64, finished: bool, new_play: bool) {
        if path.is_empty() || formats::is_url(path) && duration <= 0.0 {
            return;
        }
        let id = media_id(path);
        let mut d = self.data.write();
        let thumb = d.items.get(&id).and_then(|i| i.thumb.clone());
        let p = d.progress.entry(id).or_default();
        p.path = path.to_string();
        if !title.is_empty() {
            p.title = title.to_string();
        }
        p.pos = pos.max(0.0);
        if duration > 0.0 {
            p.duration = duration;
        }
        p.updated = now_secs();
        p.finished = finished;
        if thumb.is_some() {
            p.thumb = thumb;
        }
        if new_play {
            p.plays += 1;
        }
        drop(d);
        self.touch();
    }

    /// A picture for "Continue watching" when the file is not in the library.
    pub fn thumb_for_progress(self: &Arc<Self>, path: &str) {
        let id = media_id(path);
        {
            let d = self.data.read();
            if d.items.get(&id).map(|i| i.thumb.is_some()).unwrap_or(false) {
                return;
            }
            if d.progress.get(&id).and_then(|p| p.thumb.as_ref()).map(|t| Path::new(t).exists()).unwrap_or(false) {
                return;
            }
        }
        if !self.settings.bool("library.thumbnails") {
            return;
        }
        let me = self.clone();
        let path = path.to_string();
        std::thread::spawn(move || {
            lower_thread_priority();
            let Ok(mut g) = FrameGrabber::new() else { return };
            let Some(info) = g.open(&path, Duration::from_secs(10)) else { return };
            if !info.has_video || info.duration <= 0.0 {
                return;
            }
            let mut frame = None;
            for frac in [0.12, 0.25, 0.4] {
                if let Some(f) = g.grab(Some(info.duration * frac), 480, 270, false, Duration::from_secs(4)) {
                    let keep = f.brightness() > 22.0;
                    if frame.is_none() || keep {
                        frame = Some(f);
                    }
                    if keep {
                        break;
                    }
                }
            }
            let Some(jpg) = frame.and_then(|f| f.to_jpeg(82)) else { return };
            let file = paths::cache_dir("thumbs").join(format!("{id}.jpg"));
            if paths::write_atomic(&file, &jpg).is_err() {
                return;
            }
            if let Some(p) = me.data.write().progress.get_mut(&id) {
                p.thumb = Some(file.to_string_lossy().into_owned());
            }
            me.touch();
        });
    }

    pub fn set_watched(&self, ids: &[String], watched: bool) {
        let mut d = self.data.write();
        for id in ids {
            let item = d.items.get(id).cloned();
            let p = d.progress.entry(id.clone()).or_default();
            if let Some(i) = item {
                p.path = i.path.clone();
                p.title = i.title.clone();
                p.duration = i.duration;
                p.thumb = i.thumb.clone();
            }
            p.finished = watched;
            p.pos = 0.0;
            p.updated = now_secs();
        }
        drop(d);
        self.touch();
    }

    pub fn forget_progress(&self, ids: &[String]) {
        let mut d = self.data.write();
        for id in ids {
            d.progress.remove(id);
        }
        drop(d);
        self.touch();
    }

    pub fn clear_history(&self) {
        self.data.write().progress.clear();
        self.touch();
    }

    // ---- scanning ---------------------------------------------------------

    pub fn rescan(self: &Arc<Self>) {
        if self.scanning.swap(true, Ordering::AcqRel) {
            return;
        }
        let me = self.clone();
        std::thread::Builder::new()
            .name("library-scan".into())
            .spawn(move || {
                me.set_status(json!({"state": "scanning"}));
                let started = Instant::now();
                let n = me.scan_now();
                log::info!("library scan: {n} files in {:?}", started.elapsed());
                me.scanning.store(false, Ordering::Release);
                me.set_status(json!({"state": "idle", "files": n}));
                me.notify_workers();
            })
            .ok();
    }

    fn scan_now(&self) -> usize {
        let roots: Vec<String> = self.settings.strings("library.folders");
        let mut seen: HashSet<String> = HashSet::new();
        let mut found: Vec<(String, String, fs::Metadata)> = Vec::new();
        let mut offline_roots: HashSet<String> = HashSet::new();
        for root in &roots {
            let rp = Path::new(root);
            if !rp.is_dir() {
                offline_roots.insert(root.clone());
                continue;
            }
            walk(rp, 0, &mut |p, m| {
                let s = p.to_string_lossy().into_owned();
                if seen.insert(media_id(&s)) {
                    found.push((s, root.clone(), m));
                }
            });
        }
        let now = now_secs();
        let mut d = self.data.write();
        let mut changed = false;
        for (path, root, meta) in &found {
            let id = media_id(path);
            let size = meta.len();
            let mtime = mtime_secs(meta);
            match d.items.get_mut(&id) {
                Some(it) if it.size == size && it.mtime == mtime => {
                    if it.offline || it.root != *root {
                        it.offline = false;
                        it.root = root.clone();
                        changed = true;
                    }
                }
                _ => {
                    let p = Path::new(path);
                    let parsed = naming::parse(p);
                    let kind = if formats::is_audio(p) { "music" } else { parsed.kind };
                    let added = d.items.get(&id).map(|i| i.added).unwrap_or(if mtime > 0 { mtime.min(now) } else { now });
                    d.items.insert(
                        id.clone(),
                        Item {
                            id,
                            path: path.clone(),
                            root: root.clone(),
                            kind: kind.to_string(),
                            title: if kind == "music" { p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default() } else { parsed.title },
                            year: parsed.year,
                            show: parsed.show,
                            season: parsed.season,
                            episode: parsed.episode,
                            episode_end: parsed.episode_end,
                            episode_title: parsed.episode_title,
                            extra: parsed.extra,
                            size,
                            mtime,
                            added,
                            ..Default::default()
                        },
                    );
                    changed = true;
                }
            }
        }
        // Remove files that are gone (but keep those on drives that are not
        // connected right now, marked offline).
        let ids: Vec<String> = d.items.keys().cloned().collect();
        for id in ids {
            if seen.contains(&id) {
                continue;
            }
            let root = d.items[&id].root.clone();
            if offline_roots.contains(&root) {
                if !d.items[&id].offline {
                    d.items.get_mut(&id).unwrap().offline = true;
                    changed = true;
                }
            } else {
                if let Some(it) = d.items.remove(&id) {
                    if let Some(t) = it.thumb {
                        let _ = fs::remove_file(t);
                    }
                }
                changed = true;
            }
        }
        drop(d);
        if changed {
            self.touch();
        }
        found.len()
    }

    // ---- probing ----------------------------------------------------------

    fn next_unprobed(&self) -> Option<Item> {
        let d = self.data.read();
        d.items.values().filter(|i| !i.probed && !i.offline).max_by_key(|i| i.added).cloned()
    }

    fn wait_for_work(&self, timeout: Duration) {
        let (m, c) = &self.wake;
        let mut g = m.lock();
        let seen = *g;
        let _ = c.wait_while_for(&mut g, |x| *x == seen, timeout);
    }

    fn probe_loop(self: Arc<Self>) {
        lower_thread_priority();
        let mut grabber: Option<FrameGrabber> = None;
        let mut done_since_status = 0usize;
        loop {
            let Some(item) = self.next_unprobed() else {
                if done_since_status > 0 {
                    self.set_status(json!({"state": "idle"}));
                    done_since_status = 0;
                }
                grabber = None; // free the decoder while idle
                self.wait_for_work(Duration::from_secs(60));
                continue;
            };
            if grabber.is_none() {
                match FrameGrabber::new() {
                    Ok(g) => grabber = Some(g),
                    Err(e) => {
                        log::error!("library prober unavailable: {e}");
                        self.wait_for_work(Duration::from_secs(300));
                        continue;
                    }
                }
            }
            let remaining = self.data.read().items.values().filter(|i| !i.probed && !i.offline).count();
            if done_since_status % 5 == 0 {
                self.set_status(json!({"state": "probing", "remaining": remaining}));
            }
            let g = grabber.as_mut().unwrap();
            let updated = probe_item(g, &item, self.settings.bool("library.thumbnails"));
            {
                let mut d = self.data.write();
                if let Some(it) = d.items.get_mut(&item.id) {
                    *it = updated;
                }
            }
            self.touch();
            done_since_status += 1;
            // stay out of the way while a video plays
            let pause = if self.player_busy.load(Ordering::Acquire) { 1500 } else { 30 };
            std::thread::sleep(Duration::from_millis(pause));
        }
    }

    // ---- posters ----------------------------------------------------------

    fn wanted_posters(&self) -> Vec<(String, String, Option<i32>, bool)> {
        let d = self.data.read();
        let now = now_secs();
        let mut out: Vec<(String, String, Option<i32>, bool)> = Vec::new();
        let mut keys = HashSet::new();
        for it in d.items.values() {
            if it.extra || it.offline {
                continue;
            }
            let (key, title, year, show) = match (it.kind.as_str(), &it.show) {
                ("movie", _) => (poster_key_movie(&it.title, it.year), it.title.clone(), it.year, false),
                ("episode", Some(s)) => (poster_key_show(s), s.clone(), it.year, true),
                _ => continue,
            };
            if !keys.insert(key.clone()) {
                continue;
            }
            match d.posters.get(&key) {
                Some(p) if p.found && p.path.as_ref().map(|x| Path::new(x).exists()).unwrap_or(false) => continue,
                Some(p) if !p.found && now - p.fetched < 14 * 86400 => continue,
                _ => out.push((key, title, year, show)),
            }
        }
        out
    }

    fn poster_loop(self: Arc<Self>) {
        lower_thread_priority();
        let mut agent: Option<ureq::Agent> = None;
        loop {
            self.wait_for_work(Duration::from_secs(1800));
            if !self.settings.bool("library.posters_online") {
                continue;
            }
            let wanted = self.wanted_posters();
            if wanted.is_empty() {
                continue;
            }
            let a = agent.get_or_insert_with(crate::posters::agent);
            for (key, title, year, show) in wanted {
                if !self.settings.bool("library.posters_online") {
                    break;
                }
                match crate::posters::fetch(a, &title, year, show) {
                    Ok(Some(found)) => {
                        let file = paths::cache_dir("posters").join(format!("{:016x}.jpg", crate::core::fnv64(&key)));
                        let ok = paths::write_atomic(&file, &found.image).is_ok();
                        self.data.write().posters.insert(
                            key,
                            Poster {
                                path: ok.then(|| file.to_string_lossy().into_owned()),
                                imdb: Some(found.imdb),
                                title: Some(found.title),
                                year: found.year,
                                fetched: now_secs(),
                                found: ok,
                            },
                        );
                        self.touch();
                    }
                    Ok(None) => {
                        self.data.write().posters.insert(key, Poster { fetched: now_secs(), found: false, ..Default::default() });
                        self.touch();
                    }
                    Err(e) => {
                        log::info!("posters: network unavailable ({e}); will retry later");
                        break;
                    }
                }
                std::thread::sleep(Duration::from_millis(350));
            }
        }
    }
}

fn walk(dir: &Path, depth: usize, f: &mut dyn FnMut(&Path, fs::Metadata)) {
    if depth > 14 {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name.starts_with('$') {
            continue;
        }
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_symlink() {
            continue; // never follow links or junctions (loops)
        }
        let p = e.path();
        if ft.is_dir() {
            let low = name.to_lowercase();
            if ["system volume information", "node_modules", "windows", "program files", "program files (x86)", "appdata"].contains(&low.as_str()) {
                continue;
            }
            walk(&p, depth + 1, f);
        } else if ft.is_file() && formats::is_media(&p) {
            if let Ok(m) = e.metadata() {
                if m.len() >= 64 * 1024 {
                    f(&p, m);
                }
            }
        }
    }
}

fn meta_str(meta: &Value, keys: &[&str]) -> Option<String> {
    let obj = meta.as_object()?;
    for (k, v) in obj {
        if keys.contains(&k.to_lowercase().as_str()) {
            if let Some(s) = v.as_str() {
                let s = s.trim();
                if !s.is_empty() {
                    return Some(s.to_string());
                }
            }
        }
    }
    None
}

fn probe_item(g: &mut FrameGrabber, item: &Item, thumbs: bool) -> Item {
    let mut it = item.clone();
    it.probed = true;
    let Some(info) = g.open(&item.path, Duration::from_secs(10)) else {
        it.probe_failed = true;
        return it;
    };
    it.duration = info.duration;
    it.width = if info.dw > 0 { info.dw } else { info.width };
    it.height = if info.dh > 0 { info.dh } else { info.height };
    it.video_codec = info.video_codec.clone();
    it.audio_codec = info.audio_codec.clone();
    it.hdr = info.hdr.clone();
    if it.kind == "music" {
        if let Some(t) = meta_str(&info.metadata, &["title"]) {
            it.title = t;
        }
        it.artist = meta_str(&info.metadata, &["artist", "album_artist", "albumartist"]);
        it.album = meta_str(&info.metadata, &["album"]);
        it.genre = meta_str(&info.metadata, &["genre"]);
        it.track = meta_str(&info.metadata, &["track", "tracknumber"]).and_then(|t| t.split('/').next().and_then(|n| n.trim().parse().ok()));
        if it.year.is_none() {
            it.year = meta_str(&info.metadata, &["date", "year"]).and_then(|d| d.get(..4).and_then(|y| y.parse().ok()));
        }
    } else if !info.has_video && info.audio_codec.is_some() && it.kind == "video" {
        it.kind = "music".into();
    }
    if thumbs {
        let dir = paths::cache_dir("thumbs");
        let file: PathBuf = dir.join(format!("{}.jpg", it.id));
        let mut frame = None;
        if info.has_video && info.duration > 0.0 {
            for frac in [0.12, 0.25, 0.4, 0.06] {
                if let Some(f) = g.grab(Some(info.duration * frac), 480, 270, false, Duration::from_secs(4)) {
                    let bright = f.brightness();
                    let keep = bright > 22.0;
                    if frame.is_none() || keep {
                        frame = Some(f);
                    }
                    if keep {
                        break;
                    }
                }
            }
        } else if info.album_art {
            frame = g.grab(None, 400, 400, false, Duration::from_secs(3));
        }
        if let Some(jpg) = frame.and_then(|f| f.to_jpeg(82)) {
            if paths::write_atomic(&file, &jpg).is_ok() {
                it.thumb = Some(file.to_string_lossy().into_owned());
            }
        }
    }
    it
}

#[cfg(windows)]
fn lower_thread_priority() {
    use windows::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL};
    unsafe {
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
    }
}
#[cfg(not(windows))]
fn lower_thread_priority() {}
