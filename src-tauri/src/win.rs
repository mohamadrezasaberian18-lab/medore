//! Win32 pieces: the native window libmpv draws video into, placed under the
//! (transparent) web view inside the main window, and title-bar colours.

use std::sync::Once;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
use windows::Win32::Graphics::Gdi::{GetStockObject, BLACK_BRUSH, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetClientRect, RegisterClassW, SetWindowPos, ShowWindow, CS_HREDRAW, CS_VREDRAW,
    HWND_BOTTOM, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_SHOWNA, WINDOW_EX_STYLE, WNDCLASSW, WS_CHILD,
    WS_CLIPCHILDREN, WS_CLIPSIBLINGS,
};

const CLASS: PCWSTR = w!("MedoreVideoHost");
static REGISTER: Once = Once::new();

unsafe extern "system" fn host_proc(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    DefWindowProcW(h, msg, w, l)
}

pub fn hwnd_from(raw: isize) -> HWND {
    HWND(raw as *mut _)
}

/// Create the black child window libmpv renders into, at the bottom of the
/// main window's z-order so the web view stays on top of it.
pub fn create_video_host(parent: isize) -> Result<isize, String> {
    unsafe {
        let inst = GetModuleHandleW(None).map_err(|e| e.to_string())?;
        REGISTER.call_once(|| {
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(host_proc),
                hInstance: inst.into(),
                hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
                lpszClassName: CLASS,
                ..Default::default()
            };
            RegisterClassW(&wc);
        });
        let parent = hwnd_from(parent);
        let mut rc = RECT::default();
        let _ = GetClientRect(parent, &mut rc);
        let h = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            CLASS,
            w!(""),
            WS_CHILD | WS_CLIPSIBLINGS | WS_CLIPCHILDREN,
            0,
            0,
            rc.right - rc.left,
            rc.bottom - rc.top,
            Some(parent),
            None,
            Some(inst.into()),
            None,
        )
        .map_err(|e| e.to_string())?;
        let _ = SetWindowPos(h, Some(HWND_BOTTOM), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        Ok(h.0 as isize)
    }
}

/// Make the video host fill the parent's client area and stay underneath.
pub fn fit_video_host(host: isize, parent: isize) {
    unsafe {
        let mut rc = RECT::default();
        let _ = GetClientRect(hwnd_from(parent), &mut rc);
        let _ = SetWindowPos(
            hwnd_from(host),
            Some(HWND_BOTTOM),
            0,
            0,
            rc.right - rc.left,
            rc.bottom - rc.top,
            SWP_NOACTIVATE,
        );
    }
}

/// Put the video host above the library web view (player mode).
pub fn raise_video_host(host: isize) {
    unsafe {
        let _ = SetWindowPos(
            hwnd_from(host),
            Some(windows::Win32::UI::WindowsAndMessaging::HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

pub fn show_video_host(host: isize, show: bool) {
    unsafe {
        let _ = ShowWindow(hwnd_from(host), if show { SW_SHOWNA } else { SW_HIDE });
    }
}

unsafe extern "system" fn no_alt_menu(h: HWND, msg: u32, w: WPARAM, l: LPARAM, _id: usize, _data: usize) -> LRESULT {
    const WM_SYSCOMMAND: u32 = 0x0112;
    const SC_KEYMENU: usize = 0xF100;
    // A lone Alt press would put the title bar into "menu mode", during which
    // the web views stop updating. Alt+Space (lParam = ' ') still opens the
    // window menu; Alt+F4 is not SC_KEYMENU.
    if msg == WM_SYSCOMMAND && (w.0 & 0xFFF0) == SC_KEYMENU && l.0 == 0 {
        return LRESULT(0);
    }
    windows::Win32::UI::Shell::DefSubclassProc(h, msg, w, l)
}

/// Stop a lone Alt key from entering the title bar's menu mode.
pub fn disable_alt_menu(hwnd: isize) {
    unsafe {
        let _ = windows::Win32::UI::Shell::SetWindowSubclass(hwnd_from(hwnd), Some(no_alt_menu), 0x4D45, 0);
    }
}

/// Colour the native title bar (Windows 11; ignored on Windows 10).
pub fn set_caption_colors(hwnd: isize, caption: u32, text: u32) {
    unsafe {
        let h = hwnd_from(hwnd);
        let dark: i32 = 1;
        let _ = DwmSetWindowAttribute(h, DWMWINDOWATTRIBUTE(20), &dark as *const _ as _, 4); // immersive dark mode
        let c = COLORREF(caption);
        let _ = DwmSetWindowAttribute(h, DWMWINDOWATTRIBUTE(35), &c as *const _ as _, 4); // caption colour
        let t = COLORREF(text);
        let _ = DwmSetWindowAttribute(h, DWMWINDOWATTRIBUTE(36), &t as *const _ as _, 4); // caption text colour
    }
}
