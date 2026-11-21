#![recursion_limit = "512"]
//! Pure logic of the MEDORE player, without any window or engine code.

pub mod formats;
pub mod languages;
pub mod naming;
pub mod natsort;
pub mod posters_match;
pub mod settings_schema;
pub mod subtitles;

/// Stable 64-bit FNV-1a hash (ids and cache keys must not change between
/// versions, so std's randomised hasher is not used).
pub fn fnv64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Id of a media file: hash of its lower-cased path.
pub fn media_id(path: &str) -> String {
    format!("{:016x}", fnv64(&path.to_lowercase()))
}

pub fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
