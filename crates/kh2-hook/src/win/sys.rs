//! The few Win32 calls the loader needs.

use std::ffi::c_void;

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleFileNameW(module: *mut c_void, buf: *mut u16, size: u32) -> u32;
    pub fn GetModuleHandleA(name: *const u8) -> *mut c_void;
}
