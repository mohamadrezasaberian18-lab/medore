//! Settings schema: every setting with its default (the JSON type of the
//! default is the type of the setting), allowed choices and ranges.

use std::sync::OnceLock;

use serde_json::{json, Map, Value};

pub fn defaults() -> &'static Map<String, Value> {
    static D: OnceLock<Map<String, Value>> = OnceLock::new();
    D.get_or_init(|| {
        let v = json!({
            // General
            "general.language": "auto",          // auto | en | fa
            "general.start_page": "home",        // home | movies | shows | videos | music
            "general.first_run_done": false,
            "general.window": null,
            // Playback
            "playback.resume": "auto",           // auto | ask | never
            "playback.autoplay_next": true,
            "playback.auto_queue_folder": true,  // opening one episode queues the rest of the folder
            "playback.up_next_seconds": 10,
            "playback.click_to_pause": true,
            "playback.double_click_fullscreen": true,
            "playback.wheel": "volume",          // volume | seek
            "playback.seek_step": 5,
            "playback.seek_step_large": 30,
            "playback.volume": 100.0,
            "playback.remember_volume": true,
            "playback.pause_on_minimize": false,
            "playback.screenshot_dir": "",
            "playback.screenshot_format": "png", // png | jpg | webp
            // Video
            "video.quality": "balanced",         // fast | balanced | high
            "video.hwdec": "auto-safe",          // auto-safe | auto-copy | d3d11va | nvdec | no
            "video.deband": false,
            "video.deinterlace": "auto",         // auto | yes | no
            "video.smooth_motion": false,
            "video.hdr": "auto",                 // auto | tonemap | passthrough
            "video.tone_mapping": "auto",
            "video.cache_mb": 256,
            // Audio
            "audio.device": "auto",
            "audio.languages": [],
            "audio.normalize": false,
            "audio.passthrough": false,
            "audio.exclusive": false,
            "audio.channels": "auto-safe",       // auto-safe | stereo
            // Subtitles
            "subs.auto_load": true,
            "subs.auto_select": true,
            "subs.languages": ["fa", "en"],
            "subs.font": "Calibri",
            "subs.size": 46,
            "subs.bold": false,
            "subs.color": "#FFFFFF",
            "subs.border_color": "#000000",
            "subs.border_size": 2.4,
            "subs.shadow": 0.8,
            "subs.back_opacity": 0,
            "subs.position": 100,
            "subs.override_ass": false,
            "subs.encoding": "auto",
            "subs.persian_fix": true,
            "subs.rtl_ass_auto": false,
            // Library
            "library.folders": [],
            "library.scan_on_start": true,
            "library.thumbnails": true,
            "library.posters_online": true,
            "library.sort": {},
            "library.view": {},
            // Appearance
            "appearance.reduce_motion": "system", // system | on | off
            "appearance.controls_timeout": 2.5,
            "appearance.show_remaining": false,
            "appearance.sidebar_collapsed": false,
            // Updates
            "updates.auto_check": true,
            "updates.auto_install": true,
            // Advanced
            "advanced.mpv_options": ""
        });
        match v {
            Value::Object(m) => m,
            _ => unreachable!(),
        }
    })
}

const CHOICES: &[(&str, &[&str])] = &[
    ("general.language", &["auto", "en", "fa"]),
    ("general.start_page", &["home", "movies", "shows", "videos", "music"]),
    ("playback.resume", &["auto", "ask", "never"]),
    ("playback.wheel", &["volume", "seek"]),
    ("playback.screenshot_format", &["png", "jpg", "webp"]),
    ("video.quality", &["fast", "balanced", "high"]),
    ("video.hwdec", &["auto-safe", "auto-copy", "d3d11va", "nvdec", "no"]),
    ("video.deinterlace", &["auto", "yes", "no"]),
    ("video.hdr", &["auto", "tonemap", "passthrough"]),
    ("audio.channels", &["auto-safe", "stereo"]),
    ("appearance.reduce_motion", &["system", "on", "off"]),
];

const RANGES: &[(&str, f64, f64)] = &[
    ("playback.up_next_seconds", 0.0, 60.0),
    ("playback.seek_step", 1.0, 120.0),
    ("playback.seek_step_large", 5.0, 600.0),
    ("playback.volume", 0.0, 130.0),
    ("video.cache_mb", 16.0, 4096.0),
    ("subs.size", 12.0, 120.0),
    ("subs.border_size", 0.0, 10.0),
    ("subs.shadow", 0.0, 10.0),
    ("subs.back_opacity", 0.0, 100.0),
    ("subs.position", 0.0, 150.0),
    ("appearance.controls_timeout", 1.0, 10.0),
];

fn same_type(a: &Value, b: &Value) -> bool {
    matches!(
        (a, b),
        (Value::Bool(_), Value::Bool(_))
            | (Value::Number(_), Value::Number(_))
            | (Value::String(_), Value::String(_))
            | (Value::Array(_), Value::Array(_))
            | (Value::Object(_), Value::Object(_))
            | (Value::Null, _)
    )
}

/// Validate `value` for `key`; returns the value to store or an error.
pub fn coerce(key: &str, value: Value) -> Result<Value, String> {
    let d = defaults().get(key).ok_or_else(|| format!("unknown setting {key}"))?;
    if !same_type(d, &value) {
        return Err(format!("{key}: wrong type"));
    }
    if let Some((_, ch)) = CHOICES.iter().find(|(k, _)| *k == key) {
        let s = value.as_str().unwrap_or("");
        if !ch.contains(&s) {
            return Err(format!("{key}: {s} is not one of {ch:?}"));
        }
    }
    if let Some((_, lo, hi)) = RANGES.iter().find(|(k, _, _)| *k == key) {
        let x = value.as_f64().unwrap_or(*lo).clamp(*lo, *hi);
        let is_int = d.is_i64() || d.is_u64();
        return Ok(if is_int { json!(x.round() as i64) } else { json!(x) });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validation() {
        assert!(coerce("subs.font", json!("Calibri")).is_ok());
        assert!(coerce("subs.font", json!(3)).is_err());
        assert!(coerce("video.quality", json!("ultra")).is_err());
        assert_eq!(coerce("subs.size", json!(500)).unwrap(), json!(120));
        assert_eq!(coerce("playback.volume", json!(50)).unwrap(), json!(50.0));
        assert!(coerce("nope", json!(1)).is_err());
        assert_eq!(defaults()["subs.font"], json!("Calibri"));
    }
}
