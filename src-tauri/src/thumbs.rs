//! Timeline hover previews: a private frame grabber on its own thread.
//! The newest request wins (older ones get no picture), and results are
//! cached per file in up to ~300 buckets.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use base64::Engine;
use parking_lot::Mutex;

use crate::mpv::render::FrameGrabber;

struct Job {
    path: String,
    bucket: i64,
    t: f64,
    reply: Sender<Option<String>>,
}

pub struct Previews {
    tx: Mutex<Option<Sender<Job>>>,
    cache: Mutex<(HashMap<(String, i64), String>, VecDeque<(String, i64)>)>,
    failed: Mutex<HashSet<String>>,
}

const CAP: usize = 500;

fn step(duration: f64) -> f64 {
    (duration / 300.0).max(1.0)
}

impl Previews {
    pub fn new() -> Previews {
        Previews { tx: Mutex::new(None), cache: Mutex::new((HashMap::new(), VecDeque::new())), failed: Mutex::new(HashSet::new()) }
    }

    fn cached(&self, key: &(String, i64)) -> Option<String> {
        self.cache.lock().0.get(key).cloned()
    }

    fn store(&self, key: (String, i64), v: String) {
        let mut g = self.cache.lock();
        if g.0.insert(key.clone(), v).is_none() {
            g.1.push_back(key);
        }
        while g.1.len() > CAP {
            if let Some(old) = g.1.pop_front() {
                g.0.remove(&old);
            }
        }
    }

    fn sender(&'static self) -> Sender<Job> {
        let mut g = self.tx.lock();
        if let Some(tx) = g.as_ref() {
            return tx.clone();
        }
        let (tx, rx) = channel::<Job>();
        std::thread::Builder::new().name("previews".into()).spawn(move || self.run(rx)).ok();
        *g = Some(tx.clone());
        tx
    }

    /// Blocking: returns a data: URL with a JPEG, or None.
    pub fn request(&'static self, path: &str, t: f64, duration: f64) -> Option<String> {
        if path.is_empty() || crate::core::formats::is_url(path) || self.failed.lock().contains(path) || duration <= 0.0 {
            return None;
        }
        let s = step(duration);
        let bucket = (t / s).floor() as i64;
        let key = (path.to_string(), bucket);
        if let Some(v) = self.cached(&key) {
            return Some(v);
        }
        let (rtx, rrx) = channel();
        self.sender().send(Job { path: path.to_string(), bucket, t: bucket as f64 * s + s / 2.0, reply: rtx }).ok()?;
        rrx.recv_timeout(Duration::from_secs(8)).ok().flatten()
    }

    fn run(&'static self, rx: Receiver<Job>) {
        let mut g = match FrameGrabber::new() {
            Ok(g) => g,
            Err(e) => {
                log::error!("preview grabber unavailable: {e}");
                return;
            }
        };
        while let Ok(mut job) = rx.recv() {
            // the newest request wins
            while let Ok(newer) = rx.try_recv() {
                let _ = job.reply.send(None);
                job = newer;
            }
            let key = (job.path.clone(), job.bucket);
            if let Some(v) = self.cached(&key) {
                let _ = job.reply.send(Some(v));
                continue;
            }
            if g.open(&job.path, Duration::from_secs(8)).is_none() {
                self.failed.lock().insert(job.path.clone());
                let _ = job.reply.send(None);
                continue;
            }
            let url = g
                .grab(Some(job.t), 320, 180, false, Duration::from_secs(4))
                .and_then(|f| f.to_jpeg(78))
                .map(|jpg| format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(jpg)));
            if let Some(u) = &url {
                self.store(key, u.clone());
            }
            let _ = job.reply.send(url);
        }
    }
}
