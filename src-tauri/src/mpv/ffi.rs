//! Raw libmpv client API (client.h / render.h, API 2.x), loaded at run time
//! so a missing or broken libmpv-2.dll becomes a readable error, not a crash.
#![allow(non_camel_case_types, dead_code)]

use std::ffi::{c_char, c_double, c_int, c_void};
use std::path::Path;

use libloading::Library;

#[repr(C)]
pub struct mpv_handle {
    _p: [u8; 0],
}
#[repr(C)]
pub struct mpv_render_context {
    _p: [u8; 0],
}

pub type mpv_format = c_int;
pub const FORMAT_NONE: mpv_format = 0;
pub const FORMAT_STRING: mpv_format = 1;
pub const FORMAT_OSD_STRING: mpv_format = 2;
pub const FORMAT_FLAG: mpv_format = 3;
pub const FORMAT_INT64: mpv_format = 4;
pub const FORMAT_DOUBLE: mpv_format = 5;
pub const FORMAT_NODE: mpv_format = 6;
pub const FORMAT_NODE_ARRAY: mpv_format = 7;
pub const FORMAT_NODE_MAP: mpv_format = 8;
pub const FORMAT_BYTE_ARRAY: mpv_format = 9;

#[repr(C)]
#[derive(Clone, Copy)]
pub union mpv_node_u {
    pub string: *mut c_char,
    pub flag: c_int,
    pub int64: i64,
    pub double_: c_double,
    pub list: *mut mpv_node_list,
    pub ba: *mut mpv_byte_array,
}

#[repr(C)]
pub struct mpv_node {
    pub u: mpv_node_u,
    pub format: mpv_format,
}

#[repr(C)]
pub struct mpv_node_list {
    pub num: c_int,
    pub values: *mut mpv_node,
    pub keys: *mut *mut c_char,
}

#[repr(C)]
pub struct mpv_byte_array {
    pub data: *mut c_void,
    pub size: usize,
}

pub type mpv_event_id = c_int;
pub const EVENT_NONE: mpv_event_id = 0;
pub const EVENT_SHUTDOWN: mpv_event_id = 1;
pub const EVENT_LOG_MESSAGE: mpv_event_id = 2;
pub const EVENT_COMMAND_REPLY: mpv_event_id = 5;
pub const EVENT_START_FILE: mpv_event_id = 6;
pub const EVENT_END_FILE: mpv_event_id = 7;
pub const EVENT_FILE_LOADED: mpv_event_id = 8;
pub const EVENT_CLIENT_MESSAGE: mpv_event_id = 16;
pub const EVENT_VIDEO_RECONFIG: mpv_event_id = 17;
pub const EVENT_AUDIO_RECONFIG: mpv_event_id = 18;
pub const EVENT_SEEK: mpv_event_id = 20;
pub const EVENT_PLAYBACK_RESTART: mpv_event_id = 21;
pub const EVENT_PROPERTY_CHANGE: mpv_event_id = 22;
pub const EVENT_QUEUE_OVERFLOW: mpv_event_id = 24;
pub const EVENT_HOOK: mpv_event_id = 25;

#[repr(C)]
pub struct mpv_event {
    pub event_id: mpv_event_id,
    pub error: c_int,
    pub reply_userdata: u64,
    pub data: *mut c_void,
}

#[repr(C)]
pub struct mpv_event_property {
    pub name: *const c_char,
    pub format: mpv_format,
    pub data: *mut c_void,
}

#[repr(C)]
pub struct mpv_event_log_message {
    pub prefix: *const c_char,
    pub level: *const c_char,
    pub text: *const c_char,
    pub log_level: c_int,
}

pub const END_FILE_REASON_EOF: c_int = 0;
pub const END_FILE_REASON_STOP: c_int = 2;
pub const END_FILE_REASON_QUIT: c_int = 3;
pub const END_FILE_REASON_ERROR: c_int = 4;
pub const END_FILE_REASON_REDIRECT: c_int = 5;

#[repr(C)]
pub struct mpv_event_end_file {
    pub reason: c_int,
    pub error: c_int,
    pub playlist_entry_id: i64,
    pub playlist_insert_id: i64,
    pub playlist_insert_num_entries: c_int,
}

#[repr(C)]
pub struct mpv_event_start_file {
    pub playlist_entry_id: i64,
}

#[repr(C)]
pub struct mpv_event_hook {
    pub name: *const c_char,
    pub id: u64,
}

// render.h
pub const RENDER_PARAM_INVALID: c_int = 0;
pub const RENDER_PARAM_API_TYPE: c_int = 1;
pub const RENDER_PARAM_ADVANCED_CONTROL: c_int = 10;
pub const RENDER_PARAM_SW_SIZE: c_int = 17;
pub const RENDER_PARAM_SW_FORMAT: c_int = 18;
pub const RENDER_PARAM_SW_STRIDE: c_int = 19;
pub const RENDER_PARAM_SW_POINTER: c_int = 20;
pub const RENDER_UPDATE_FRAME: u64 = 1;

#[repr(C)]
pub struct mpv_render_param {
    pub type_: c_int,
    pub data: *mut c_void,
}

macro_rules! api {
    ($($name:ident : fn($($arg:ty),*) $(-> $ret:ty)?;)*) => {
        pub struct Api {
            _lib: Library,
            $(pub $name: unsafe extern "C" fn($($arg),*) $(-> $ret)?,)*
        }
        impl Api {
            pub fn load(path: &Path) -> Result<Api, String> {
                unsafe {
                    let lib = Library::new(path).map_err(|e| format!("cannot load {}: {e}", path.display()))?;
                    $(
                        let $name = *lib
                            .get::<unsafe extern "C" fn($($arg),*) $(-> $ret)?>(concat!("mpv_", stringify!($name), "\0").as_bytes())
                            .map_err(|e| format!("libmpv is missing mpv_{}: {e}", stringify!($name)))?;
                    )*
                    Ok(Api { _lib: lib, $($name,)* })
                }
            }
        }
    };
}

api! {
    client_api_version: fn() -> u64;
    error_string: fn(c_int) -> *const c_char;
    free: fn(*mut c_void);
    create: fn() -> *mut mpv_handle;
    initialize: fn(*mut mpv_handle) -> c_int;
    terminate_destroy: fn(*mut mpv_handle);
    set_option_string: fn(*mut mpv_handle, *const c_char, *const c_char) -> c_int;
    command: fn(*mut mpv_handle, *mut *const c_char) -> c_int;
    command_async: fn(*mut mpv_handle, u64, *mut *const c_char) -> c_int;
    command_node: fn(*mut mpv_handle, *mut mpv_node, *mut mpv_node) -> c_int;
    set_property: fn(*mut mpv_handle, *const c_char, mpv_format, *mut c_void) -> c_int;
    set_property_string: fn(*mut mpv_handle, *const c_char, *const c_char) -> c_int;
    get_property: fn(*mut mpv_handle, *const c_char, mpv_format, *mut c_void) -> c_int;
    get_property_string: fn(*mut mpv_handle, *const c_char) -> *mut c_char;
    observe_property: fn(*mut mpv_handle, u64, *const c_char, mpv_format) -> c_int;
    request_log_messages: fn(*mut mpv_handle, *const c_char) -> c_int;
    wait_event: fn(*mut mpv_handle, c_double) -> *mut mpv_event;
    wakeup: fn(*mut mpv_handle);
    hook_add: fn(*mut mpv_handle, u64, *const c_char, c_int) -> c_int;
    hook_continue: fn(*mut mpv_handle, u64) -> c_int;
    free_node_contents: fn(*mut mpv_node);
    render_context_create: fn(*mut *mut mpv_render_context, *mut mpv_handle, *mut mpv_render_param) -> c_int;
    render_context_render: fn(*mut mpv_render_context, *mut mpv_render_param) -> c_int;
    render_context_update: fn(*mut mpv_render_context) -> u64;
    render_context_free: fn(*mut mpv_render_context);
}
