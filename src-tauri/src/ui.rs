//! Windows: the main window (library, and the video host while playing) and
//! the player controls — a transparent window owned by the main window and
//! kept exactly over its client area. WebView2 cannot show a native video
//! window *underneath* itself in the same window, so the controls live in
//! their own window, like most players that embed libmpv.

use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use serde_json::json;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Position, Size, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::win;

struct Saved {
    pos: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    maximized: bool,
}

pub struct Ui {
    app: AppHandle,
    pub main_hwnd: isize,
    pub host: isize,
    player: AtomicBool,
    pip: Mutex<Option<Saved>>,
    on_top: AtomicBool,
}

impl Ui {
    pub fn create(app: &AppHandle) -> Result<Ui, String> {
        let main = app.get_webview_window("main").ok_or("no main window")?;
        let main_hwnd = main.hwnd().map_err(|e| e.to_string())?.0 as isize;
        win::set_caption_colors(main_hwnd, 0x00080808, 0x00FFFFFF);
        win::disable_alt_menu(main_hwnd);
        let host = win::create_video_host(main_hwnd)?;
        win::fit_video_host(host, main_hwnd);
        WebviewWindowBuilder::new(app, "overlay", WebviewUrl::App("index.html#overlay".into()))
            .title("MEDORE")
            .transparent(true)
            .decorations(false)
            .shadow(false)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .visible(false)
            .parent(&main)
            .map_err(|e| e.to_string())?
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Ui { app: app.clone(), main_hwnd, host, player: AtomicBool::new(false), pip: Mutex::new(None), on_top: AtomicBool::new(false) })
    }

    fn main(&self) -> Option<WebviewWindow> {
        self.app.get_webview_window("main")
    }
    fn overlay(&self) -> Option<WebviewWindow> {
        self.app.get_webview_window("overlay")
    }

    pub fn in_player(&self) -> bool {
        self.player.load(Ordering::Acquire)
    }

    /// Keep the video host and the controls exactly over the client area.
    pub fn sync(&self) {
        win::fit_video_host(self.host, self.main_hwnd);
        if self.in_player() {
            win::raise_video_host(self.host);
        }
        let (Some(m), Some(o)) = (self.main(), self.overlay()) else { return };
        if let (Ok(p), Ok(s)) = (m.inner_position(), m.inner_size()) {
            let _ = o.set_position(Position::Physical(p));
            let _ = o.set_size(Size::Physical(s));
        }
    }

    pub fn set_player(&self, on: bool) {
        let was = self.player.swap(on, Ordering::AcqRel);
        if was == on {
            if on {
                self.sync();
            }
            return;
        }
        if on {
            win::show_video_host(self.host, true);
            self.sync();
            if let Some(o) = self.overlay() {
                let _ = o.show();
                if self.on_top.load(Ordering::Acquire) {
                    let _ = o.set_always_on_top(true);
                }
            }
        } else {
            if self.pip.lock().is_some() {
                self.set_pip(false);
            }
            if let Some(m) = self.main() {
                if m.is_fullscreen().unwrap_or(false) {
                    let _ = m.set_fullscreen(false);
                }
            }
            win::show_video_host(self.host, false);
            if let Some(o) = self.overlay() {
                let _ = o.hide();
            }
            if let Some(m) = self.main() {
                let _ = m.set_focus();
            }
        }
        let _ = self.app.emit("ui://mode", json!({ "player": on }));
    }

    pub fn fullscreen(&self) -> bool {
        self.main().and_then(|m| m.is_fullscreen().ok()).unwrap_or(false)
    }

    pub fn set_fullscreen(&self, on: bool) {
        if on && self.pip.lock().is_some() {
            self.set_pip(false);
        }
        if let Some(m) = self.main() {
            let _ = m.set_fullscreen(on);
        }
        self.sync();
        let _ = self.app.emit("ui://fullscreen", on);
    }

    pub fn set_on_top(&self, on: bool) {
        self.on_top.store(on, Ordering::Release);
        if let Some(m) = self.main() {
            let _ = m.set_always_on_top(on);
        }
        if let Some(o) = self.overlay() {
            let _ = o.set_always_on_top(on);
        }
        let _ = self.app.emit("ui://ontop", on);
    }

    pub fn pip(&self) -> bool {
        self.pip.lock().is_some()
    }

    /// Picture in picture: a small borderless window, always on top, in the
    /// bottom-right corner of the current screen.
    pub fn set_pip(&self, on: bool) {
        let Some(m) = self.main() else { return };
        let mut g = self.pip.lock();
        if on == g.is_some() {
            return;
        }
        if on {
            if m.is_fullscreen().unwrap_or(false) {
                let _ = m.set_fullscreen(false);
            }
            let maximized = m.is_maximized().unwrap_or(false);
            if maximized {
                let _ = m.unmaximize();
            }
            let saved = Saved {
                pos: m.outer_position().unwrap_or_default(),
                size: m.inner_size().unwrap_or(PhysicalSize::new(1280, 780)),
                maximized,
            };
            *g = Some(saved);
            drop(g);
            let scale = m.scale_factor().unwrap_or(1.0);
            let (w, h) = (480.0, 270.0);
            let _ = m.set_min_size(Some(Size::Logical(LogicalSize::new(240.0, 135.0))));
            let _ = m.set_decorations(false);
            let _ = m.set_size(Size::Logical(LogicalSize::new(w, h)));
            if let Ok(Some(mon)) = m.current_monitor() {
                let wa = mon.work_area();
                let x = wa.position.x + wa.size.width as i32 - (w * scale) as i32 - (24.0 * scale) as i32;
                let y = wa.position.y + wa.size.height as i32 - (h * scale) as i32 - (24.0 * scale) as i32;
                let _ = m.set_position(Position::Physical(PhysicalPosition::new(x, y)));
            }
            let _ = m.set_always_on_top(true);
            if let Some(o) = self.overlay() {
                let _ = o.set_always_on_top(true);
            }
        } else {
            let saved = g.take();
            drop(g);
            let _ = m.set_decorations(true);
            let _ = m.set_min_size(Some(Size::Logical(LogicalSize::new(720.0, 460.0))));
            let top = self.on_top.load(Ordering::Acquire);
            let _ = m.set_always_on_top(top);
            if let Some(o) = self.overlay() {
                let _ = o.set_always_on_top(top);
            }
            if let Some(s) = saved {
                let _ = m.set_size(Size::Physical(s.size));
                let _ = m.set_position(Position::Physical(s.pos));
                if s.maximized {
                    let _ = m.maximize();
                }
            }
        }
        self.sync();
        let _ = self.app.emit("ui://pip", on);
    }
}
