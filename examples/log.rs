extern crate env_logger;
extern crate rist;

use std::ffi::*;

fn rist_log(
    settings: *mut librist_sys::rist_logging_settings,
    level: librist_sys::rist_log_level,
    message: &str,
) {
    unsafe {
        ((*settings).log_cb.unwrap())(
            (*settings).log_cb_arg,
            level,
            CString::new(message).unwrap().as_c_str().as_ptr(),
        );
    }
}

fn log_messages(mut ctx: Box<dyn rist::log::LogContext>) {
    rist_log(
        ctx.logging_settings_ptr(),
        librist_sys::rist_log_level_RIST_LOG_DEBUG,
        "This is a debug message",
    );
    rist_log(
        ctx.logging_settings_ptr(),
        librist_sys::rist_log_level_RIST_LOG_INFO,
        "This is an info message",
    );
    rist_log(
        ctx.logging_settings_ptr(),
        librist_sys::rist_log_level_RIST_LOG_NOTICE,
        "This is a notice message",
    );
    rist_log(
        ctx.logging_settings_ptr(),
        librist_sys::rist_log_level_RIST_LOG_WARN,
        "This is a warning message",
    );
    rist_log(
        ctx.logging_settings_ptr(),
        librist_sys::rist_log_level_RIST_LOG_ERROR,
        "This is an error message",
    );
}

fn main() {
    env_logger::init();

    // create a default context
    let default_context = rist::log::create_default_logging_context();

    // create a default context using the 'log' crate api
    let named_context = rist::log::create_default_named_logging_context("rist-recv-1");

    // create a context that wraps a callback
    let custom_context =
        rist::log::LogContextUnsafe::create(|message, _level, _target| println!("{}", message));

    // create a named context that wraps a callback 
    let custom_named_context =
        rist::log::LogContextUnsafe::create_named("rist-recv", |message, _level, target| {
            println!("{} - {}", target.unwrap(), message)
        });

    log_messages(default_context);
    log_messages(named_context);
    log_messages(custom_context);
    log_messages(custom_named_context);
}
