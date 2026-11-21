//! Translate MEDORE settings into libmpv options. `init_options` is used when
//! the player starts; `runtime_updates` gives the properties to change when a
//! setting changes during playback, so settings apply immediately.

use crate::settings::Settings;

type Opts = Vec<(String, String)>;

fn push(o: &mut Opts, k: &str, v: impl ToString) {
    o.push((k.to_string(), v.to_string()));
}

fn yn(b: bool) -> &'static str {
    if b {
        "yes"
    } else {
        "no"
    }
}

fn quality(o: &mut Opts, q: &str) {
    let (scale, dscale, cscale, extra): (&str, &str, &str, &[(&str, &str)]) = match q {
        "fast" => ("bilinear", "bilinear", "bilinear", &[("correct-downscaling", "no"), ("linear-downscaling", "no"), ("sigmoid-upscaling", "no"), ("hdr-compute-peak", "no")]),
        "high" => (
            "ewa_lanczossharp",
            "mitchell",
            "ewa_lanczossharp",
            &[("correct-downscaling", "yes"), ("linear-downscaling", "yes"), ("sigmoid-upscaling", "yes"), ("hdr-compute-peak", "yes"), ("hdr-contrast-recovery", "0.3")],
        ),
        _ => ("lanczos", "hermite", "lanczos", &[("correct-downscaling", "yes"), ("linear-downscaling", "yes"), ("sigmoid-upscaling", "yes"), ("hdr-compute-peak", "auto")]),
    };
    push(o, "scale", scale);
    push(o, "dscale", dscale);
    push(o, "cscale", cscale);
    for (k, v) in extra {
        push(o, k, v);
    }
}

pub fn video(s: &Settings) -> Opts {
    let mut o = Opts::new();
    push(&mut o, "hwdec", s.str("video.hwdec"));
    quality(&mut o, &s.str("video.quality"));
    push(&mut o, "deband", yn(s.bool("video.deband")));
    push(&mut o, "deinterlace", s.str("video.deinterlace"));
    let smooth = s.bool("video.smooth_motion");
    push(&mut o, "video-sync", if smooth { "display-resample" } else { "audio" });
    push(&mut o, "interpolation", yn(smooth));
    if smooth {
        push(&mut o, "tscale", "oversample");
    }
    push(&mut o, "target-colorspace-hint", match s.str("video.hdr").as_str() {
        "tonemap" => "no",
        "passthrough" => "yes",
        _ => "auto",
    });
    push(&mut o, "tone-mapping", s.str("video.tone_mapping"));
    let mb = s.f64("video.cache_mb") as i64;
    push(&mut o, "demuxer-max-bytes", format!("{mb}MiB"));
    push(&mut o, "demuxer-max-back-bytes", format!("{}MiB", (mb / 2).max(16)));
    o
}

pub fn audio(s: &Settings) -> Opts {
    let mut o = Opts::new();
    let dev = s.str("audio.device");
    push(&mut o, "audio-device", if dev.is_empty() { "auto".to_string() } else { dev });
    push(&mut o, "audio-exclusive", yn(s.bool("audio.exclusive")));
    push(&mut o, "audio-spdif", if s.bool("audio.passthrough") { "ac3,eac3,dts,dts-hd,truehd" } else { "" });
    push(&mut o, "audio-channels", s.str("audio.channels"));
    push(&mut o, "af", if s.bool("audio.normalize") { "lavfi=[dynaudnorm=f=250:g=15:p=0.9]" } else { "" });
    o
}

pub fn subs(s: &Settings) -> Opts {
    let mut o = Opts::new();
    let font = s.str("subs.font");
    push(&mut o, "sub-font", if font.trim().is_empty() { "Calibri".to_string() } else { font });
    push(&mut o, "sub-font-size", s.f64("subs.size") as i64);
    push(&mut o, "sub-bold", yn(s.bool("subs.bold")));
    push(&mut o, "sub-color", s.str("subs.color"));
    push(&mut o, "sub-border-color", s.str("subs.border_color"));
    push(&mut o, "sub-border-size", s.f64("subs.border_size"));
    push(&mut o, "sub-shadow-offset", s.f64("subs.shadow"));
    push(&mut o, "sub-ass-override", if s.bool("subs.override_ass") { "force" } else { "scale" });
    let enc = s.str("subs.encoding");
    push(&mut o, "sub-codepage", if enc.is_empty() { "auto".to_string() } else { enc });
    let back = s.f64("subs.back_opacity");
    if back > 0.0 {
        let a = ((back / 100.0) * 255.0).round() as u8;
        push(&mut o, "sub-border-style", "background-box");
        push(&mut o, "sub-back-color", format!("#{a:02X}0B1120"));
    } else {
        push(&mut o, "sub-border-style", "outline-and-shadow");
    }
    o
}

pub fn init_options(s: &Settings, wid: Option<isize>) -> Opts {
    let mut o = Opts::new();
    for (k, v) in [
        ("config", "no"),
        ("terminal", "no"),
        ("input-default-bindings", "no"),
        ("input-vo-keyboard", "no"),
        ("input-cursor", "no"),
        ("input-media-keys", "no"),
        ("osc", "no"),
        ("load-stats-overlay", "yes"),
        ("load-console", "no"),
        ("load-select", "no"),
        ("load-auto-profiles", "no"),
        ("load-positioning", "no"),
        ("load-context-menu", "no"),
        ("load-commands", "no"),
        ("cursor-autohide", "no"),
        ("osd-level", "1"),
        ("osd-bar", "no"),
        ("osd-on-seek", "no"),
        ("osd-font", "Segoe UI"),
        ("osd-font-size", "22"),
        ("idle", "yes"),
        ("keep-open", "yes"),
        ("hr-seek", "yes"),
        ("hr-seek-framedrop", "yes"),
        ("save-position-on-quit", "no"),
        ("volume-max", "130"),
        ("sub-auto", "no"),       // MEDORE finds subtitles itself (smarter matching)
        ("audio-file-auto", "no"),
        ("audio-display", "embedded-first"),
        ("image-display-duration", "inf"),
        (
            "reset-on-next-file",
            "sub-delay,audio-delay,video-zoom,video-pan-x,video-pan-y,video-rotate,video-aspect-override,ab-loop-a,ab-loop-b,secondary-sid,speed",
        ),
        ("embeddedfonts", "yes"),
        ("sub-scale-with-window", "yes"),
        ("sub-use-margins", "yes"),
        ("ytdl", "yes"),
        ("network-timeout", "20"),
        ("screenshot-template", "MEDORE %F %wH-%wM-%wS"),
        ("vo", "gpu-next"),
        ("gpu-api", "d3d11"),
        ("background-color", "#000000"),
    ] {
        push(&mut o, k, v);
    }
    let vol = if s.bool("playback.remember_volume") { s.f64("playback.volume") } else { 100.0 };
    push(&mut o, "volume", vol);
    let shot = s.str("playback.screenshot_dir");
    let shot = if shot.is_empty() { default_screenshot_dir() } else { shot };
    push(&mut o, "screenshot-directory", shot);
    push(&mut o, "screenshot-format", s.str("playback.screenshot_format"));
    if !s.bool("playback.autoplay_next") {
        push(&mut o, "keep-open", "always");
    }
    if let Some(w) = wid {
        push(&mut o, "wid", w);
    }
    o.extend(video(s));
    o.extend(audio(s));
    o.extend(subs(s));
    o.extend(advanced(s));
    o
}

pub fn default_screenshot_dir() -> String {
    let base = std::env::var("USERPROFILE").unwrap_or_default();
    std::path::Path::new(&base).join("Pictures").join("MEDORE").to_string_lossy().into_owned()
}

/// The expert "extra mpv options" box: one name=value per line.
pub fn advanced(s: &Settings) -> Opts {
    let mut o = Opts::new();
    for line in s.str("advanced.mpv_options").lines() {
        let line = line.trim().trim_start_matches("--");
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (k, v) = line.split_once('=').map(|(k, v)| (k.trim(), v.trim())).unwrap_or((line, "yes"));
        let v = v.trim_matches(|c| c == '"' || c == '\'');
        if !k.is_empty() {
            push(&mut o, k, v);
        }
    }
    o
}

/// Properties to set on a running player when setting `key` changes.
pub fn runtime_updates(s: &Settings, key: &str) -> Opts {
    if key.starts_with("video.") {
        return video(s);
    }
    if key.starts_with("audio.") && key != "audio.languages" {
        return audio(s);
    }
    if key.starts_with("subs.") && !["subs.languages", "subs.auto_select", "subs.auto_load", "subs.rtl_ass_auto", "subs.persian_fix"].contains(&key) {
        return subs(s);
    }
    match key {
        "playback.autoplay_next" => vec![("keep-open".into(), if s.bool(key) { "yes" } else { "always" }.into())],
        "playback.screenshot_format" => vec![("screenshot-format".into(), s.str(key))],
        "playback.screenshot_dir" => {
            let d = s.str(key);
            vec![("screenshot-directory".into(), if d.is_empty() { default_screenshot_dir() } else { d })]
        }
        _ => vec![],
    }
}
