//! The MEDORE icon follows the Windows light/dark setting: the dark icon
//! (silver on obsidian) when Windows is in dark mode, the light icon (graphite
//! on white) in light mode. While MEDORE runs, the window and taskbar icon
//! change live; its Start menu, desktop and pinned taskbar shortcuts are
//! pointed at the matching .ico so they match even when MEDORE is closed.

use std::path::{Path, PathBuf};

use tauri::image::Image;
use tauri::{AppHandle, Manager};

const DARK_PNG: &[u8] = include_bytes!("../icons/window-dark.png");
const LIGHT_PNG: &[u8] = include_bytes!("../icons/window-light.png");

/// True when Windows' own surfaces (taskbar, Start) use the light theme.
#[cfg(windows)]
pub fn system_light() -> bool {
    use windows::core::w;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
    let mut v: u32 = 0;
    let mut size = 4u32;
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut v as *mut u32 as *mut _),
            Some(&mut size),
        )
    };
    r.is_ok() && v != 0
}
#[cfg(not(windows))]
pub fn system_light() -> bool {
    false
}

pub fn apply(app: &AppHandle) {
    let light = system_light();
    if let Some(w) = app.get_webview_window("main") {
        if let Ok(img) = Image::from_bytes(if light { LIGHT_PNG } else { DARK_PNG }) {
            let _ = w.set_icon(img);
        }
    }
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().unwrap_or_default();
        let ico = exe.parent().map(|d| d.join(if light { "medore-light.ico" } else { "medore-dark.ico" }));
        if let Some(ico) = ico.filter(|p| p.exists()) {
            std::thread::spawn(move || shortcuts::point_at(&exe, &ico));
        }
    }
}

/// Re-apply whenever the Windows theme changes.
#[cfg(windows)]
pub fn watch(app: AppHandle) {
    use windows::core::w;
    use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegNotifyChangeKeyValue, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_NOTIFY, REG_NOTIFY_CHANGE_LAST_SET,
    };
    use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject, INFINITE};
    std::thread::Builder::new()
        .name("theme-watch".into())
        .spawn(move || unsafe {
            let mut key = HKEY::default();
            if RegOpenKeyExW(HKEY_CURRENT_USER, w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"), Some(0), KEY_NOTIFY, &mut key)
                .is_err()
            {
                return;
            }
            let Ok(ev) = CreateEventW(None, false, false, None) else { return };
            let mut last = system_light();
            loop {
                if RegNotifyChangeKeyValue(key, false, REG_NOTIFY_CHANGE_LAST_SET, Some(ev), true).is_err() {
                    break;
                }
                if WaitForSingleObject(ev, INFINITE) != WAIT_OBJECT_0 {
                    break;
                }
                let now = system_light();
                if now != last {
                    last = now;
                    let a = app.clone();
                    let _ = app.run_on_main_thread(move || apply(&a));
                }
            }
            let _ = CloseHandle(ev);
            let _ = RegCloseKey(key);
        })
        .ok();
}
#[cfg(not(windows))]
pub fn watch(_app: AppHandle) {}

#[cfg(windows)]
mod shortcuts {
    use super::*;
    use windows::core::{Interface, HSTRING, PCWSTR};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, STGM_READWRITE};
    use windows::Win32::UI::Shell::{IShellLinkW, SHChangeNotify, ShellLink, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};

    fn candidates() -> Vec<PathBuf> {
        let mut v = Vec::new();
        if let Some(app) = std::env::var_os("APPDATA").map(PathBuf::from) {
            v.push(app.join(r"Microsoft\Windows\Start Menu\Programs\MEDORE.lnk"));
            v.push(app.join(r"Microsoft\Windows\Start Menu\Programs\MEDORE\MEDORE.lnk"));
            v.push(app.join(r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar\MEDORE.lnk"));
        }
        if let Some(home) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
            v.push(home.join(r"Desktop\MEDORE.lnk"));
            v.push(home.join(r"OneDrive\Desktop\MEDORE.lnk"));
        }
        v.into_iter().filter(|p| p.exists()).collect()
    }

    pub fn point_at(exe: &Path, ico: &Path) {
        let links = candidates();
        if links.is_empty() {
            return;
        }
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let mut changed = false;
            for lnk in links {
                let Ok(link) = CoCreateInstance::<_, IShellLinkW>(&ShellLink, None, CLSCTX_INPROC_SERVER) else { continue };
                let Ok(file) = link.cast::<IPersistFile>() else { continue };
                let path = HSTRING::from(lnk.as_os_str());
                if file.Load(PCWSTR(path.as_ptr()), STGM_READWRITE).is_err() {
                    continue;
                }
                let mut target = [0u16; 1024];
                if link.GetPath(&mut target, std::ptr::null_mut(), 0).is_err() {
                    continue;
                }
                let t = String::from_utf16_lossy(&target[..target.iter().position(|&c| c == 0).unwrap_or(0)]);
                if !Path::new(&t).eq(exe) && !t.eq_ignore_ascii_case(&exe.to_string_lossy()) {
                    continue; // someone else's shortcut with the same name
                }
                let mut cur = [0u16; 1024];
                let mut idx = 0i32;
                let _ = link.GetIconLocation(&mut cur, &mut idx);
                let cur = String::from_utf16_lossy(&cur[..cur.iter().position(|&c| c == 0).unwrap_or(0)]);
                if cur.eq_ignore_ascii_case(&ico.to_string_lossy()) {
                    continue;
                }
                let icon = HSTRING::from(ico.as_os_str());
                if link.SetIconLocation(PCWSTR(icon.as_ptr()), 0).is_ok() && file.Save(PCWSTR(path.as_ptr()), true).is_ok() {
                    changed = true;
                }
            }
            if changed {
                SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
            }
            CoUninitialize();
        }
    }
}
