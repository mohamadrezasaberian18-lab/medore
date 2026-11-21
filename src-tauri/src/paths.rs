//! Where MEDORE keeps its files. A `portable.txt` next to MEDORE.exe keeps
//! everything in a `data` folder beside it; otherwise settings live in
//! %APPDATA%\MEDORE and caches in %LOCALAPPDATA%\MEDORE.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub struct Paths {
    pub data: PathBuf,
    pub cache: PathBuf,
    pub exe_dir: PathBuf,
    pub portable: bool,
}

static PATHS: OnceLock<Paths> = OnceLock::new();

fn env_dir(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from).filter(|p| !p.as_os_str().is_empty())
}

pub fn init() -> &'static Paths {
    PATHS.get_or_init(|| {
        let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_default();
        if let Some(home) = env_dir("MEDORE_HOME") {
            return Paths { data: home.join("data"), cache: home.join("cache"), exe_dir, portable: true };
        }
        if exe_dir.join("portable.txt").exists() {
            let d = exe_dir.join("data");
            return Paths { data: d.join("profile"), cache: d.join("cache"), exe_dir, portable: true };
        }
        let roaming = env_dir("APPDATA").unwrap_or_else(|| exe_dir.join("data"));
        let local = env_dir("LOCALAPPDATA").unwrap_or_else(|| roaming.clone());
        Paths { data: roaming.join("MEDORE"), cache: local.join("MEDORE").join("cache"), exe_dir, portable: false }
    })
}

pub fn get() -> &'static Paths {
    init()
}

pub fn data_file(name: &str) -> PathBuf {
    let p = &get().data;
    let _ = std::fs::create_dir_all(p);
    p.join(name)
}

pub fn cache_dir(sub: &str) -> PathBuf {
    let p = get().cache.join(sub);
    let _ = std::fs::create_dir_all(&p);
    p
}

pub fn libmpv() -> PathBuf {
    if let Some(p) = env_dir("MEDORE_LIBMPV") {
        return p;
    }
    get().exe_dir.join("libmpv-2.dll")
}

/// Write a file atomically (temp file + rename) so a crash never leaves a
/// half-written settings or library file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}
