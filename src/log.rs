extern crate librist_sys;
extern crate log;

use std::ffi::{c_void, CStr};
use std::os::raw::c_char;
use std::sync::Mutex;

fn get_log_level(rist_log_level: librist_sys::rist_log_level) -> Option<log::Level> {
    match rist_log_level {
        librist_sys::rist_log_level_RIST_LOG_DEBUG => Some(log::Level::Trace),
        librist_sys::rist_log_level_RIST_LOG_INFO => Some(log::Level::Debug),
        librist_sys::rist_log_level_RIST_LOG_NOTICE => Some(log::Level::Info),
        librist_sys::rist_log_level_RIST_LOG_WARN => Some(log::Level::Warn),
        librist_sys::rist_log_level_RIST_LOG_ERROR => Some(log::Level::Error),
        _ => None,
    }
}

pub struct LogContextSafe {
    librist_logging_settings: librist_sys::rist_logging_settings,
    user_log_fn: Mutex<Box<dyn FnMut(&str, log::Level, Option<&str>)>>,
    target: Option<String>,
}

unsafe extern "C" fn logcb_safe(
    ctx: *mut c_void,
    rist_log_level: librist_sys::rist_log_level,
    msg: *const c_char,
) -> i32 {
    get_log_level(rist_log_level).map(|l| {
        let ctx = ctx as *mut LogContextSafe;
        CStr::from_ptr(msg)
            .to_str()
            .map(|str| ((*ctx).user_log_fn.lock().unwrap())(str, l, (*ctx).target.as_ref().map(|s|s.as_str())))
            .ok()
    });
    0
}

unsafe extern "C" fn logcb_unsafe(
    ctx: *mut c_void,
    rist_log_level: librist_sys::rist_log_level,
    msg: *const c_char,
) -> i32 {
    get_log_level(rist_log_level).map(|l| {
        let ctx = ctx as *mut LogContextUnsafe;
        CStr::from_ptr(msg)
            .to_str()
            .map(|str| ((*ctx).user_log_fn)(str, l, (*ctx).target.as_ref().map(|s| s.as_str())))
            .ok()
    });
    0
}

impl LogContextSafe {
    pub fn create(c: impl FnMut(&str, log::Level, Option<&str>) + 'static) -> Box<LogContextSafe> {
        let raw = Box::into_raw(Box::new(LogContextSafe {
            librist_logging_settings: librist_sys::rist_logging_settings {
                log_level: librist_sys::rist_log_level_RIST_LOG_DEBUG,
                log_cb: Some(logcb_safe),
                log_socket: 0,
                log_cb_arg: std::ptr::null_mut(),
                log_stream: std::ptr::null_mut(),
            },
            user_log_fn: Mutex::new(Box::new(c)),
            target: None,
        }));
        unsafe {
            (*raw).librist_logging_settings.log_cb_arg = raw as *mut c_void;
            Box::from_raw(raw)
        }
    }

    pub fn create_named(name: &str, c: impl FnMut(&str, log::Level, Option<&str>) + 'static) -> Box<LogContextSafe> {
        let mut context = LogContextSafe::create(c);
        context.target = Some(String::from(name));
        return context;
    }
}

pub struct LogContextUnsafe {
    rustapi_logging_settings: librist_sys::rist_logging_settings,
    user_log_fn: Box<dyn FnMut(&str, log::Level, Option<&str>)>,
    target: Option<String>,
}

impl LogContextUnsafe {
    pub fn create(
        c: impl FnMut(&str, log::Level, Option<&str>) + 'static,
    ) -> Box<LogContextUnsafe> {
        let raw = Box::into_raw(Box::new(LogContextUnsafe {
            rustapi_logging_settings: librist_sys::rist_logging_settings {
                log_level: librist_sys::rist_log_level_RIST_LOG_DEBUG,
                log_cb: Some(logcb_unsafe),
                log_socket: 0,
                log_cb_arg: std::ptr::null_mut(),
                log_stream: std::ptr::null_mut(),
            },
            user_log_fn: Box::new(c),
            target: None,
        }));
        unsafe {
            (*raw).rustapi_logging_settings.log_cb_arg = raw as *mut c_void;
            Box::from_raw(raw)
        }
    }

    pub fn create_named(name: &str, c: impl FnMut(&str, log::Level, Option<&str>) + 'static) -> Box<LogContextUnsafe> {
        let mut context = LogContextUnsafe::create(c);
        context.target = Some(String::from(name));
        return context;
    }
}

pub trait LogContext {
    fn logging_settings_ptr(&mut self) -> *mut librist_sys::rist_logging_settings;
}

impl LogContext for LogContextSafe {
    fn logging_settings_ptr(&mut self) -> *mut librist_sys::rist_logging_settings {
        std::ptr::addr_of_mut!(self.librist_logging_settings)
    }
}

impl LogContext for LogContextUnsafe {
    fn logging_settings_ptr(&mut self) -> *mut librist_sys::rist_logging_settings {
        std::ptr::addr_of_mut!(self.rustapi_logging_settings)
    }
}

pub fn create_default_logging_context() -> Box<dyn LogContext> {
    LogContextUnsafe::create(|message, level, target: Option<&str>| {
        log::log!(target: target.unwrap_or("rist"), level, "{}", message);
    })
}

pub fn create_default_named_logging_context(name: &str) -> Box<dyn LogContext> {
    LogContextUnsafe::create_named(name, |message, level, target: Option<&str>| {
        log::log!(target: target.unwrap_or("rist"), level, "{}", message);
    })
}
