#![allow(non_camel_case_types)]

use std::path::Path;

use serde_json::Value as JsonValue;

pub trait JanusLogger {
    fn new(server_name: &str, config_path: &Path) -> Self;
    fn incoming_logline(&self, timestamp: i64, line: &str);
    fn handle_request(&self, request: &JsonValue) -> JsonValue;
}

////////////////////////////////////////////////////////////////////////////////

use jansson_sys::json_t;
use std::os::raw::{c_char, c_int, c_long};

// https://github.com/meetecho/janus-gateway/blob/master/loggers/logger.h
#[repr(C)]
#[derive(Debug)]
pub struct janus_logger {
    pub init: extern "C" fn(server_name: *const c_char, config_path: *const c_char) -> c_int,
    pub destroy: extern "C" fn(),
    pub get_api_compatibility: extern "C" fn() -> c_int,
    pub get_version: extern "C" fn() -> c_int,
    pub get_version_string: extern "C" fn() -> *const c_char,
    pub get_description: extern "C" fn() -> *const c_char,
    pub get_name: extern "C" fn() -> *const c_char,
    pub get_author: extern "C" fn() -> *const c_char,
    pub get_package: extern "C" fn() -> *const c_char,
    pub incoming_logline: extern "C" fn(timestamp: c_long, line: *const c_char),
    pub handle_request: extern "C" fn(request: *const json_t) -> *mut json_t,
}

////////////////////////////////////////////////////////////////////////////////

#[macro_export]
macro_rules! define_logger {
    ($l:tt) => {
        const JANUS_LOGGER_API_VERSION: std::os::raw::c_int = 3;

        lazy_static::lazy_static! {
            static ref LOGGER: atom::AtomSetOnce<Box<$l>> = atom::AtomSetOnce::empty();
        }

        extern "C" fn init(
            server_name: *const std::os::raw::c_char,
            config_path: *const std::os::raw::c_char,
        ) -> std::os::raw::c_int {
            use std::ffi::CStr;

            use crate::janus_logger::JanusLogger;

            let server_name = unsafe { CStr::from_ptr(server_name) }
                .to_str()
                .expect("Failed to cast server name");

            let config_path = unsafe { CStr::from_ptr(config_path) }
                .to_str()
                .expect("Failed to cast config path");

            let logger = $l::new(server_name, &std::path::Path::new(config_path));
            LOGGER.set_if_none(Box::new(logger), std::sync::atomic::Ordering::SeqCst);
            0
        }

        extern "C" fn destroy() {}

        extern "C" fn get_api_compatibility() -> std::os::raw::c_int {
            JANUS_LOGGER_API_VERSION
        }

        extern "C" fn get_version() -> std::os::raw::c_int {
            let major = env!("CARGO_PKG_VERSION_MAJOR")
                .parse::<i32>()
                .expect("Failed to parse major crate version");

            let minor = env!("CARGO_PKG_VERSION_MINOR")
                .parse::<i32>()
                .expect("Failed to parse minor crate version");

            let patch = env!("CARGO_PKG_VERSION_MAJOR")
                .parse::<i32>()
                .expect("Failed to parse patch crate version");

            major * 10000 + minor * 100 + patch
        }

        extern "C" fn get_version_string() -> *const std::os::raw::c_char {
            std::ffi::CString::new(env!("CARGO_PKG_VERSION"))
                .expect("Failed to convert package version to CString")
                .as_ptr()
        }

        extern "C" fn get_description() -> *const std::os::raw::c_char {
            std::ffi::CString::new(env!("CARGO_PKG_DESCRIPTION"))
                .expect("Failed to convert package description to CString")
                .as_ptr()
        }

        extern "C" fn get_name() -> *const std::os::raw::c_char {
            std::ffi::CString::new(env!("CARGO_PKG_NAME"))
                .expect("Failed to convert package name to CString")
                .as_ptr()
        }

        extern "C" fn get_author() -> *const std::os::raw::c_char {
            std::ffi::CString::new(env!("CARGO_PKG_AUTHORS"))
                .expect("Failed to convert package authors to CString")
                .as_ptr()
        }

        extern "C" fn get_package() -> *const std::os::raw::c_char {
            std::ffi::CString::new(format!("janus.logger.{}", env!("CARGO_PKG_NAME")))
                .expect("Failed to convert package name to CString")
                .as_ptr()
        }

        extern "C" fn incoming_logline(
            timestamp: std::os::raw::c_long,
            line: *const std::os::raw::c_char,
        ) {
            use std::ffi::CStr;
            use std::sync::atomic::Ordering;

            use crate::janus_logger::JanusLogger;

            if let Some(ref logger) = LOGGER.get(Ordering::SeqCst) {
                if let Ok(line) = unsafe { CStr::from_ptr(line) }.to_str() {
                    logger.incoming_logline(timestamp, line);
                }
            }
        }

        extern "C" fn handle_request(
            request: *const jansson_sys::json_t,
        ) -> *mut jansson_sys::json_t {
            use std::ffi::{CStr, CString};
            use std::sync::atomic::Ordering;

            use crate::janus_logger::JanusLogger;

            let maybe_logger = LOGGER.get(Ordering::SeqCst);

            let logger = match maybe_logger {
                Some(ref logger) => logger,
                None => return std::ptr::null_mut(),
            };

            let dumped_request = unsafe { jansson_sys::json_dumps(request, 0) };

            let request_str = match unsafe { CStr::from_ptr(dumped_request) }.to_str() {
                Ok(request_str) => request_str,
                Err(_err) => return std::ptr::null_mut(),
            };

            let parsed_json = match serde_json::from_str(request_str) {
                Ok(parsed_json) => parsed_json,
                Err(_err) => return std::ptr::null_mut(),
            };

            let response = logger.handle_request(&parsed_json);

            let dumped_response = match CString::new(response.to_string()) {
                Ok(dumped_response) => dumped_response,
                Err(_err) => return std::ptr::null_mut(),
            };

            unsafe { jansson_sys::json_loads(dumped_response.as_ptr(), 0, std::ptr::null_mut()) }
        }

        const C_LOGGER: crate::janus_logger::janus_logger = crate::janus_logger::janus_logger {
            init,
            destroy,
            get_api_compatibility,
            get_version,
            get_version_string,
            get_description,
            get_name,
            get_author,
            get_package,
            incoming_logline,
            handle_request,
        };

        #[no_mangle]
        pub extern "C" fn create() -> *const $crate::janus_logger::janus_logger {
            &C_LOGGER
        }
    };
}
