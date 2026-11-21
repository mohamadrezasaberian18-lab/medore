//! File types MEDORE recognises. Playback itself is decided by libmpv; these
//! lists only decide what the library scans and what dialogs show.

use std::path::Path;

pub const VIDEO_EXTS: &[&str] = &[
    "mkv", "mp4", "m4v", "mov", "avi", "webm", "ts", "m2ts", "mts", "flv", "wmv", "mpg", "mpeg", "mpe", "vob", "3gp", "3g2",
    "ogv", "divx", "xvid", "rm", "rmvb", "asf", "f4v", "y4m", "mxf", "dv", "hevc", "h264", "h265", "av1", "ivf", "nut", "qt",
];
pub const AUDIO_EXTS: &[&str] = &[
    "mp3", "aac", "m4a", "m4b", "flac", "alac", "wav", "wave", "aif", "aiff", "ogg", "oga", "opus", "wma", "ape", "wv", "mka",
    "dsf", "dff", "ac3", "eac3", "dts", "mpc", "tta", "caf", "mp2", "amr", "au",
];
pub const SUBTITLE_EXTS: &[&str] = &[
    "srt", "ass", "ssa", "vtt", "webvtt", "sub", "idx", "sup", "smi", "sami", "ttml", "dfxp", "lrc",
];
pub const PLAYLIST_EXTS: &[&str] = &["m3u", "m3u8", "pls"];

pub fn ext_of(path: &Path) -> String {
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

pub fn is_video(path: &Path) -> bool {
    VIDEO_EXTS.contains(&ext_of(path).as_str())
}

pub fn is_audio(path: &Path) -> bool {
    AUDIO_EXTS.contains(&ext_of(path).as_str())
}

pub fn is_media(path: &Path) -> bool {
    let e = ext_of(path);
    VIDEO_EXTS.contains(&e.as_str()) || AUDIO_EXTS.contains(&e.as_str())
}

pub fn is_subtitle(path: &Path) -> bool {
    SUBTITLE_EXTS.contains(&ext_of(path).as_str())
}

pub fn is_playlist(path: &Path) -> bool {
    PLAYLIST_EXTS.contains(&ext_of(path).as_str())
}

pub fn is_url(s: &str) -> bool {
    let head: String = s.trim().chars().take(12).collect::<String>().to_lowercase();
    head.contains("://") && !head.starts_with("file://")
}
