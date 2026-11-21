//! Headless frame grabber: a private libmpv instance renders single frames
//! through the software render API into memory. Used for timeline hover
//! previews and library thumbnails. It decodes on the CPU with cheap
//! settings and never touches the main player.

use std::ffi::{c_int, c_void, CString};
use std::ptr;
use std::time::{Duration, Instant};

use serde_json::Value;

use super::{api, ffi, Event, Mpv};

const OPTIONS: &[(&str, &str)] = &[
    ("vo", "libmpv"),
    ("ao", "null"),
    ("aid", "no"),
    ("sid", "no"),
    ("hwdec", "no"),
    ("pause", "yes"),
    ("idle", "yes"),
    ("keep-open", "always"),
    ("hr-seek", "no"),
    ("cache", "no"),
    ("demuxer-readahead-secs", "0"),
    ("demuxer-max-bytes", "16MiB"),
    ("config", "no"),
    ("terminal", "no"),
    ("load-scripts", "no"),
    ("ytdl", "no"),
    ("osc", "no"),
    ("load-stats-overlay", "no"),
    ("load-console", "no"),
    ("load-select", "no"),
    ("load-auto-profiles", "no"),
    ("load-positioning", "no"),
    ("load-context-menu", "no"),
    ("load-commands", "no"),
    ("input-default-bindings", "no"),
    ("vd-lavc-skiploopfilter", "all"),
    ("vd-lavc-fast", "yes"),
    ("vd-lavc-threads", "2"),
    ("sws-scaler", "fast-bilinear"),
    ("audio-display", "embedded-first"),
    ("image-display-duration", "inf"),
    ("osd-level", "0"),
    ("sub-auto", "no"),
    ("audio-file-auto", "no"),
    ("tone-mapping", "hable"),
];

#[derive(Debug, Clone, Default)]
pub struct ProbeInfo {
    pub duration: f64,
    pub width: i64,
    pub height: i64,
    pub dw: i64,
    pub dh: i64,
    pub has_video: bool,
    pub album_art: bool,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub hdr: Option<String>,
    pub tracks: Value,
    pub metadata: Value,
}

pub struct Frame {
    pub w: u32,
    pub h: u32,
    /// Tightly packed B, G, R, X bytes.
    pub bgrx: Vec<u8>,
}

impl Frame {
    pub fn to_jpeg(&self, quality: u8) -> Option<Vec<u8>> {
        let mut out = Vec::with_capacity((self.w * self.h / 4) as usize);
        let enc = jpeg_encoder::Encoder::new(&mut out, quality);
        enc.encode(&self.bgrx, self.w as u16, self.h as u16, jpeg_encoder::ColorType::Bgra).ok()?;
        Some(out)
    }

    /// Mean brightness 0..255, used to skip black frames for thumbnails.
    pub fn brightness(&self) -> f32 {
        let mut sum: u64 = 0;
        let mut n: u64 = 0;
        for px in self.bgrx.chunks_exact(4).step_by(7) {
            sum += (px[0] as u64 * 114 + px[1] as u64 * 587 + px[2] as u64 * 299) / 1000;
            n += 1;
        }
        if n == 0 {
            0.0
        } else {
            sum as f32 / n as f32
        }
    }
}

pub struct FrameGrabber {
    ctx: *mut ffi::mpv_render_context,
    pub m: Mpv,
    path: Option<String>,
    info: Option<ProbeInfo>,
}

unsafe impl Send for FrameGrabber {}

impl FrameGrabber {
    pub fn new() -> Result<FrameGrabber, String> {
        let m = Mpv::create()?;
        for (k, v) in OPTIONS {
            let _ = m.set_option(k, v);
        }
        m.initialize()?;
        let api_type = CString::new("sw").unwrap();
        let mut params = [
            ffi::mpv_render_param { type_: ffi::RENDER_PARAM_API_TYPE, data: api_type.as_ptr() as *mut c_void },
            ffi::mpv_render_param { type_: ffi::RENDER_PARAM_INVALID, data: ptr::null_mut() },
        ];
        let mut ctx: *mut ffi::mpv_render_context = ptr::null_mut();
        let rc = unsafe { (api().render_context_create)(&mut ctx, m.raw(), params.as_mut_ptr()) };
        if rc < 0 || ctx.is_null() {
            return Err(format!("software renderer unavailable ({rc})"));
        }
        Ok(FrameGrabber { ctx, m, path: None, info: None })
    }

    fn new_frame(&self) -> bool {
        unsafe { (api().render_context_update)(self.ctx) & ffi::RENDER_UPDATE_FRAME != 0 }
    }

    /// Any frame mpv offers must be consumed, or its pipeline waits on us.
    fn service_frames(&self) {
        if self.new_frame() {
            let _ = self.render(16, 16);
        }
    }

    fn drain(&self) {
        while !matches!(self.m.wait_event(0.0), Event::None) {}
    }

    fn wait_loaded(&self, timeout: Duration) -> bool {
        let end = Instant::now() + timeout;
        loop {
            let left = end.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            self.service_frames();
            match self.m.wait_event(left.as_secs_f64().min(0.01)) {
                Event::FileLoaded => return true,
                Event::EndFile { reason, .. } if reason == ffi::END_FILE_REASON_ERROR => return false,
                _ => {}
            }
        }
    }

    pub fn open(&mut self, path: &str, timeout: Duration) -> Option<ProbeInfo> {
        if self.path.as_deref() == Some(path) {
            if let Some(i) = &self.info {
                return Some(i.clone());
            }
        }
        self.path = None;
        self.info = None;
        self.drain();
        self.m.command(&["loadfile", path, "replace"]).ok()?;
        if !self.wait_loaded(timeout) {
            return None;
        }
        let tracks = self.m.get("track-list").unwrap_or(Value::Array(vec![]));
        let list = tracks.as_array().cloned().unwrap_or_default();
        let v = list.iter().find(|t| t["type"] == "video");
        let a = list.iter().find(|t| t["type"] == "audio");
        let mut info = ProbeInfo {
            duration: self.m.get_f64("duration").unwrap_or(0.0),
            metadata: self.m.get("metadata").unwrap_or(Value::Null),
            tracks: tracks.clone(),
            ..Default::default()
        };
        if let Some(v) = v {
            let art = v["albumart"].as_bool().unwrap_or(false);
            let image = v["image"].as_bool().unwrap_or(false);
            info.has_video = !art && !image;
            info.album_art = art;
            info.width = v["demux-w"].as_i64().unwrap_or(0);
            info.height = v["demux-h"].as_i64().unwrap_or(0);
            info.video_codec = v["codec"].as_str().map(str::to_string);
            // wait for the first decoded frame so video-params is known
            let end = Instant::now() + Duration::from_secs(4);
            let mut vp = Value::Null;
            while Instant::now() < end {
                self.service_frames();
                vp = self.m.get("video-params").unwrap_or(Value::Null);
                if vp["dw"].as_i64().unwrap_or(0) > 0 {
                    break;
                }
                std::thread::sleep(Duration::from_millis(4));
            }
            self.drain();
            info.dw = vp["dw"].as_i64().filter(|x| *x > 0).unwrap_or(info.width);
            info.dh = vp["dh"].as_i64().filter(|x| *x > 0).unwrap_or(info.height);
            match vp["gamma"].as_str() {
                Some("pq") => info.hdr = Some("HDR10".into()),
                Some("hlg") => info.hdr = Some("HLG".into()),
                _ => {}
            }
            if info.hdr.is_some() && vp.to_string().to_lowercase().contains("dolby") {
                info.hdr = Some("Dolby Vision".into());
            }
        }
        if let Some(a) = a {
            info.audio_codec = a["codec"].as_str().map(str::to_string);
        }
        self.path = Some(path.to_string());
        self.info = Some(info.clone());
        Some(info)
    }

    pub fn grab(&mut self, t: Option<f64>, max_w: u32, max_h: u32, exact: bool, timeout: Duration) -> Option<Frame> {
        let info = self.info.clone()?;
        if !info.has_video && !info.album_art {
            return None;
        }
        let dw = if info.dw > 0 { info.dw } else if info.width > 0 { info.width } else { 16 } as f64;
        let dh = if info.dh > 0 { info.dh } else if info.height > 0 { info.height } else { 9 } as f64;
        let scale = (max_w as f64 / dw).min(max_h as f64 / dh);
        let w = ((dw * scale) as u32).max(2) & !1;
        let h = ((dh * scale) as u32).max(2) & !1;
        if let (Some(t), true) = (t, info.has_video) {
            self.drain();
            let ts = format!("{:.3}", t.max(0.0));
            let flags = if exact { "absolute+exact" } else { "absolute+keyframes" };
            self.m.command(&["seek", &ts, flags]).ok()?;
            return self.pump(timeout, w, h);
        }
        unsafe { (api().render_context_update)(self.ctx) };
        self.render(w, h)
    }

    /// Render frames as soon as mpv offers them until the seek completes.
    /// The libmpv video output waits for the application to render before it
    /// reports playback-restart, so prompt rendering keeps grabs fast.
    fn pump(&self, timeout: Duration, w: u32, h: u32) -> Option<Frame> {
        let end = Instant::now() + timeout;
        let mut img: Option<Frame> = None;
        let mut restarted = false;
        while Instant::now() < end {
            let mut got = false;
            if self.new_frame() {
                img = self.render(w, h);
                got = true;
            }
            loop {
                match self.m.wait_event(0.0) {
                    Event::None => break,
                    Event::PlaybackRestart => restarted = true,
                    Event::EndFile { reason, .. } if reason == ffi::END_FILE_REASON_ERROR => return None,
                    _ => {}
                }
            }
            if restarted && img.is_some() {
                break;
            }
            if !got {
                std::thread::sleep(Duration::from_millis(3));
            }
        }
        if img.is_none() {
            unsafe { (api().render_context_update)(self.ctx) };
            img = self.render(w, h);
        }
        img
    }

    fn render(&self, w: u32, h: u32) -> Option<Frame> {
        let stride: usize = ((w as usize * 4 + 63) / 64) * 64;
        // over-allocate so the start can be aligned to 64 bytes
        let mut raw = vec![0u8; stride * h as usize + 64];
        let base = raw.as_mut_ptr() as usize;
        let aligned = (base + 63) & !63;
        let offset = aligned - base;
        let mut size: [c_int; 2] = [w as c_int, h as c_int];
        let fmt = CString::new("bgr0").unwrap();
        let mut st: usize = stride;
        let mut params = [
            ffi::mpv_render_param { type_: ffi::RENDER_PARAM_SW_SIZE, data: size.as_mut_ptr() as *mut c_void },
            ffi::mpv_render_param { type_: ffi::RENDER_PARAM_SW_FORMAT, data: fmt.as_ptr() as *mut c_void },
            ffi::mpv_render_param { type_: ffi::RENDER_PARAM_SW_STRIDE, data: &mut st as *mut usize as *mut c_void },
            ffi::mpv_render_param { type_: ffi::RENDER_PARAM_SW_POINTER, data: aligned as *mut c_void },
            ffi::mpv_render_param { type_: ffi::RENDER_PARAM_INVALID, data: ptr::null_mut() },
        ];
        let rc = unsafe { (api().render_context_render)(self.ctx, params.as_mut_ptr()) };
        if rc < 0 {
            return None;
        }
        let row = w as usize * 4;
        let mut out = Vec::with_capacity(row * h as usize);
        for y in 0..h as usize {
            let s = offset + y * stride;
            out.extend_from_slice(&raw[s..s + row]);
        }
        Some(Frame { w, h, bgrx: out })
    }
}

impl Drop for FrameGrabber {
    fn drop(&mut self) {
        if !self.ctx.is_null() {
            unsafe { (api().render_context_free)(self.ctx) };
            self.ctx = ptr::null_mut();
        }
        // `m` is dropped after this and terminates the instance.
    }
}
