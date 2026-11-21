//! Minimal file logger: %APPDATA%\MEDORE\logs\medore.log, rotated at 2 MB.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use log::{LevelFilter, Metadata, Record};
use parking_lot::Mutex;

struct FileLog {
    file: Mutex<Option<File>>,
    path: PathBuf,
}

impl log::Log for FileLog {
    fn enabled(&self, m: &Metadata) -> bool {
        m.level() <= log::max_level()
    }
    fn log(&self, r: &Record) {
        if !self.enabled(r.metadata()) {
            return;
        }
        let line = format!("{} {:<5} {}: {}\n", crate::core::now_secs(), r.level(), r.target(), r.args());
        if cfg!(debug_assertions) {
            eprint!("{line}");
        }
        let mut g = self.file.lock();
        if let Some(f) = g.as_mut() {
            let _ = f.write_all(line.as_bytes());
            if f.metadata().map(|m| m.len() > 2_000_000).unwrap_or(false) {
                let _ = std::fs::rename(&self.path, self.path.with_extension("old.log"));
                *g = OpenOptions::new().create(true).append(true).open(&self.path).ok();
            }
        }
    }
    fn flush(&self) {
        if let Some(f) = self.file.lock().as_mut() {
            let _ = f.flush();
        }
    }
}

pub fn init() {
    let dir = crate::paths::get().data.join("logs");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("medore.log");
    let file = OpenOptions::new().create(true).append(true).open(&path).ok();
    let logger = Box::new(FileLog { file: Mutex::new(file), path });
    if log::set_boxed_logger(logger).is_ok() {
        let level = match std::env::var("MEDORE_LOG").as_deref() {
            Ok("debug") => LevelFilter::Debug,
            Ok("trace") => LevelFilter::Trace,
            _ => LevelFilter::Info,
        };
        log::set_max_level(level);
    }
    std::panic::set_hook(Box::new(|info| {
        log::error!("panic: {info}");
    }));
}
