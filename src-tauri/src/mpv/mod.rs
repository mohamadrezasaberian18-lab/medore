//! Safe wrapper over the libmpv client API.
//!
//! libmpv handles are thread-safe: commands and property calls may come from
//! any thread. `wait_event` must only ever be called from one thread (the
//! player's event thread).

pub mod ffi;
pub mod render;

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::Path;
use std::ptr;
use std::sync::OnceLock;

use serde_json::{Map, Value};

static API: OnceLock<Result<ffi::Api, String>> = OnceLock::new();

/// Load libmpv once. Later calls return the first result.
pub fn load(path: &Path) -> Result<(), String> {
    match API.get_or_init(|| ffi::Api::load(path)) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.clone()),
    }
}

pub fn api() -> &'static ffi::Api {
    match API.get() {
        Some(Ok(a)) => a,
        _ => panic!("libmpv used before it was loaded"),
    }
}

pub fn loaded() -> bool {
    matches!(API.get(), Some(Ok(_)))
}

fn err_str(code: c_int) -> String {
    unsafe {
        let p = (api().error_string)(code);
        if p.is_null() {
            format!("mpv error {code}")
        } else {
            CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    }
}

fn check(code: c_int) -> Result<(), String> {
    if code >= 0 {
        Ok(())
    } else {
        Err(err_str(code))
    }
}

fn cstr(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_default()
}

unsafe fn opt_str(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

pub struct Mpv {
    h: *mut ffi::mpv_handle,
}

// libmpv's client API is documented as thread-safe.
unsafe impl Send for Mpv {}
unsafe impl Sync for Mpv {}

#[derive(Debug, Clone)]
pub enum Event {
    None,
    Shutdown,
    Log { prefix: String, level: String, text: String },
    StartFile { entry_id: i64 },
    EndFile { reason: i32, error: i32, entry_id: i64 },
    FileLoaded,
    VideoReconfig,
    AudioReconfig,
    Seek,
    PlaybackRestart,
    Property { id: u64, name: String, value: Value },
    Hook { user: u64, name: String, id: u64 },
    CommandReply { user: u64, error: i32 },
    QueueOverflow,
    Other(i32),
}

impl Mpv {
    pub fn create() -> Result<Mpv, String> {
        if !loaded() {
            return Err("libmpv is not loaded".into());
        }
        let h = unsafe { (api().create)() };
        if h.is_null() {
            return Err("mpv_create failed".into());
        }
        Ok(Mpv { h })
    }

    pub fn set_option(&self, name: &str, value: &str) -> Result<(), String> {
        let (n, v) = (cstr(name), cstr(value));
        check(unsafe { (api().set_option_string)(self.h, n.as_ptr(), v.as_ptr()) })
            .map_err(|e| format!("option {name}={value}: {e}"))
    }

    pub fn initialize(&self) -> Result<(), String> {
        check(unsafe { (api().initialize)(self.h) })
    }

    pub fn command(&self, args: &[&str]) -> Result<(), String> {
        let owned: Vec<CString> = args.iter().map(|a| cstr(a)).collect();
        let mut ptrs: Vec<*const c_char> = owned.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(ptr::null());
        check(unsafe { (api().command)(self.h, ptrs.as_mut_ptr()) }).map_err(|e| format!("{}: {e}", args.first().unwrap_or(&"")))
    }

    pub fn command_async(&self, user: u64, args: &[&str]) -> Result<(), String> {
        let owned: Vec<CString> = args.iter().map(|a| cstr(a)).collect();
        let mut ptrs: Vec<*const c_char> = owned.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(ptr::null());
        check(unsafe { (api().command_async)(self.h, user, ptrs.as_mut_ptr()) })
    }

    /// Run a command whose arguments are JSON values (named arguments when
    /// `args` is an object), returning its result.
    pub fn command_value(&self, args: &Value) -> Result<Value, String> {
        let mut keep = NodeBuilder::default();
        let mut node = keep.build(args);
        let mut out = ffi::mpv_node { u: ffi::mpv_node_u { int64: 0 }, format: ffi::FORMAT_NONE };
        let r = unsafe { (api().command_node)(self.h, &mut node, &mut out) };
        check(r)?;
        let v = unsafe { node_to_json(&out) };
        unsafe { (api().free_node_contents)(&mut out) };
        Ok(v)
    }

    pub fn set_property(&self, name: &str, value: &str) -> Result<(), String> {
        let (n, v) = (cstr(name), cstr(value));
        check(unsafe { (api().set_property_string)(self.h, n.as_ptr(), v.as_ptr()) })
            .map_err(|e| format!("{name}={value}: {e}"))
    }

    pub fn set_property_value(&self, name: &str, value: &Value) -> Result<(), String> {
        match value {
            Value::String(s) => self.set_property(name, s),
            Value::Bool(b) => self.set_property(name, if *b { "yes" } else { "no" }),
            Value::Number(n) => self.set_property(name, &n.to_string()),
            Value::Null => self.set_property(name, "no"),
            _ => {
                let n = cstr(name);
                let mut keep = NodeBuilder::default();
                let mut node = keep.build(value);
                check(unsafe { (api().set_property)(self.h, n.as_ptr(), ffi::FORMAT_NODE, &mut node as *mut _ as *mut c_void) })
            }
        }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        let n = cstr(name);
        let mut out = ffi::mpv_node { u: ffi::mpv_node_u { int64: 0 }, format: ffi::FORMAT_NONE };
        let r = unsafe { (api().get_property)(self.h, n.as_ptr(), ffi::FORMAT_NODE, &mut out as *mut _ as *mut c_void) };
        if r < 0 {
            return None;
        }
        let v = unsafe { node_to_json(&out) };
        unsafe { (api().free_node_contents)(&mut out) };
        Some(v)
    }

    pub fn get_string(&self, name: &str) -> Option<String> {
        let n = cstr(name);
        unsafe {
            let p = (api().get_property_string)(self.h, n.as_ptr());
            if p.is_null() {
                return None;
            }
            let s = CStr::from_ptr(p).to_string_lossy().into_owned();
            (api().free)(p as *mut c_void);
            Some(s)
        }
    }

    pub fn get_f64(&self, name: &str) -> Option<f64> {
        let n = cstr(name);
        let mut v: f64 = 0.0;
        let r = unsafe { (api().get_property)(self.h, n.as_ptr(), ffi::FORMAT_DOUBLE, &mut v as *mut _ as *mut c_void) };
        (r >= 0).then_some(v)
    }

    pub fn get_i64(&self, name: &str) -> Option<i64> {
        let n = cstr(name);
        let mut v: i64 = 0;
        let r = unsafe { (api().get_property)(self.h, n.as_ptr(), ffi::FORMAT_INT64, &mut v as *mut _ as *mut c_void) };
        (r >= 0).then_some(v)
    }

    pub fn get_bool(&self, name: &str) -> Option<bool> {
        let n = cstr(name);
        let mut v: c_int = 0;
        let r = unsafe { (api().get_property)(self.h, n.as_ptr(), ffi::FORMAT_FLAG, &mut v as *mut _ as *mut c_void) };
        (r >= 0).then_some(v != 0)
    }

    pub fn observe(&self, id: u64, name: &str) -> Result<(), String> {
        let n = cstr(name);
        check(unsafe { (api().observe_property)(self.h, id, n.as_ptr(), ffi::FORMAT_NODE) })
    }

    pub fn request_log(&self, level: &str) -> Result<(), String> {
        let l = cstr(level);
        check(unsafe { (api().request_log_messages)(self.h, l.as_ptr()) })
    }

    pub fn hook_add(&self, user: u64, name: &str, priority: i32) -> Result<(), String> {
        let n = cstr(name);
        check(unsafe { (api().hook_add)(self.h, user, n.as_ptr(), priority) })
    }

    pub fn hook_continue(&self, id: u64) {
        unsafe {
            (api().hook_continue)(self.h, id);
        }
    }

    pub fn wakeup(&self) {
        unsafe { (api().wakeup)(self.h) }
    }

    pub fn raw(&self) -> *mut ffi::mpv_handle {
        self.h
    }

    /// Block until the next event (or `timeout` seconds; negative = forever).
    pub fn wait_event(&self, timeout: f64) -> Event {
        unsafe {
            let e = (api().wait_event)(self.h, timeout);
            if e.is_null() {
                return Event::None;
            }
            let e = &*e;
            match e.event_id {
                ffi::EVENT_NONE => Event::None,
                ffi::EVENT_SHUTDOWN => Event::Shutdown,
                ffi::EVENT_LOG_MESSAGE => {
                    let m = &*(e.data as *const ffi::mpv_event_log_message);
                    Event::Log { prefix: opt_str(m.prefix), level: opt_str(m.level), text: opt_str(m.text) }
                }
                ffi::EVENT_START_FILE => {
                    let d = &*(e.data as *const ffi::mpv_event_start_file);
                    Event::StartFile { entry_id: d.playlist_entry_id }
                }
                ffi::EVENT_END_FILE => {
                    let d = &*(e.data as *const ffi::mpv_event_end_file);
                    Event::EndFile { reason: d.reason, error: d.error, entry_id: d.playlist_entry_id }
                }
                ffi::EVENT_FILE_LOADED => Event::FileLoaded,
                ffi::EVENT_VIDEO_RECONFIG => Event::VideoReconfig,
                ffi::EVENT_AUDIO_RECONFIG => Event::AudioReconfig,
                ffi::EVENT_SEEK => Event::Seek,
                ffi::EVENT_PLAYBACK_RESTART => Event::PlaybackRestart,
                ffi::EVENT_QUEUE_OVERFLOW => Event::QueueOverflow,
                ffi::EVENT_COMMAND_REPLY => Event::CommandReply { user: e.reply_userdata, error: e.error },
                ffi::EVENT_PROPERTY_CHANGE => {
                    let p = &*(e.data as *const ffi::mpv_event_property);
                    let value = if p.format == ffi::FORMAT_NODE && !p.data.is_null() {
                        node_to_json(&*(p.data as *const ffi::mpv_node))
                    } else {
                        Value::Null
                    };
                    Event::Property { id: e.reply_userdata, name: opt_str(p.name), value }
                }
                ffi::EVENT_HOOK => {
                    let h = &*(e.data as *const ffi::mpv_event_hook);
                    Event::Hook { user: e.reply_userdata, name: opt_str(h.name), id: h.id }
                }
                other => Event::Other(other),
            }
        }
    }
}

impl Drop for Mpv {
    fn drop(&mut self) {
        unsafe { (api().terminate_destroy)(self.h) }
    }
}

/// Convert an mpv node into JSON.
///
/// # Safety
/// `n` must point to a valid node owned by libmpv.
pub unsafe fn node_to_json(n: *const ffi::mpv_node) -> Value {
    let n = &*n;
    match n.format {
        ffi::FORMAT_STRING | ffi::FORMAT_OSD_STRING => Value::String(opt_str(n.u.string)),
        ffi::FORMAT_FLAG => Value::Bool(n.u.flag != 0),
        ffi::FORMAT_INT64 => Value::from(n.u.int64),
        ffi::FORMAT_DOUBLE => serde_json::Number::from_f64(n.u.double_).map(Value::Number).unwrap_or(Value::Null),
        ffi::FORMAT_NODE_ARRAY => {
            let l = &*n.u.list;
            let mut out = Vec::with_capacity(l.num.max(0) as usize);
            for i in 0..l.num.max(0) as usize {
                out.push(node_to_json(l.values.add(i)));
            }
            Value::Array(out)
        }
        ffi::FORMAT_NODE_MAP => {
            let l = &*n.u.list;
            let mut out = Map::new();
            for i in 0..l.num.max(0) as usize {
                let k = opt_str(*l.keys.add(i));
                out.insert(k, node_to_json(l.values.add(i)));
            }
            Value::Object(out)
        }
        _ => Value::Null,
    }
}

/// Builds mpv nodes from JSON and keeps their memory alive while in use.
#[derive(Default)]
pub struct NodeBuilder {
    strings: Vec<CString>,
    lists: Vec<Box<ffi::mpv_node_list>>,
    values: Vec<Vec<ffi::mpv_node>>,
    keys: Vec<Vec<*mut c_char>>,
}

impl NodeBuilder {
    pub fn build(&mut self, v: &Value) -> ffi::mpv_node {
        match v {
            Value::Null => ffi::mpv_node { u: ffi::mpv_node_u { int64: 0 }, format: ffi::FORMAT_NONE },
            Value::Bool(b) => ffi::mpv_node { u: ffi::mpv_node_u { flag: *b as c_int }, format: ffi::FORMAT_FLAG },
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    ffi::mpv_node { u: ffi::mpv_node_u { int64: i }, format: ffi::FORMAT_INT64 }
                } else {
                    ffi::mpv_node { u: ffi::mpv_node_u { double_: n.as_f64().unwrap_or(0.0) }, format: ffi::FORMAT_DOUBLE }
                }
            }
            Value::String(s) => {
                let c = cstr(s);
                let p = c.as_ptr() as *mut c_char;
                self.strings.push(c);
                ffi::mpv_node { u: ffi::mpv_node_u { string: p }, format: ffi::FORMAT_STRING }
            }
            Value::Array(a) => {
                let mut vals: Vec<ffi::mpv_node> = a.iter().map(|x| self.build(x)).collect();
                let list = Box::new(ffi::mpv_node_list { num: vals.len() as c_int, values: vals.as_mut_ptr(), keys: ptr::null_mut() });
                self.values.push(vals);
                self.push_list(list, ffi::FORMAT_NODE_ARRAY)
            }
            Value::Object(o) => {
                let mut vals = Vec::with_capacity(o.len());
                let mut keys = Vec::with_capacity(o.len());
                for (k, x) in o {
                    let c = cstr(k);
                    keys.push(c.as_ptr() as *mut c_char);
                    self.strings.push(c);
                    vals.push(self.build(x));
                }
                let list = Box::new(ffi::mpv_node_list { num: vals.len() as c_int, values: vals.as_mut_ptr(), keys: keys.as_mut_ptr() });
                self.values.push(vals);
                self.keys.push(keys);
                self.push_list(list, ffi::FORMAT_NODE_MAP)
            }
        }
    }

    fn push_list(&mut self, mut list: Box<ffi::mpv_node_list>, format: ffi::mpv_format) -> ffi::mpv_node {
        let p: *mut ffi::mpv_node_list = &mut *list;
        self.lists.push(list);
        ffi::mpv_node { u: ffi::mpv_node_u { list: p }, format }
    }
}
