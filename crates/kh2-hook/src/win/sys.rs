//! The few Win32 calls the loader needs.

use std::ffi::c_void;

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleFileNameW(module: *mut c_void, buf: *mut u16, size: u32) -> u32;
    pub fn GetModuleHandleA(name: *const u8) -> *mut c_void;
    pub fn VirtualAlloc(addr: *mut c_void, size: usize, kind: u32, prot: u32) -> *mut c_void;
    pub fn VirtualProtect(addr: *mut c_void, size: usize, prot: u32, old: *mut u32) -> i32;
    pub fn FlushInstructionCache(process: *mut c_void, addr: *const c_void, size: usize) -> i32;
    pub fn GetCurrentProcess() -> *mut c_void;
    pub fn ReadProcessMemory(process: *mut c_void, at: *const c_void, buf: *mut c_void, size: usize, read: *mut usize) -> i32;
    pub fn QueryPerformanceCounter(count: *mut i64) -> i32;
    pub fn QueryPerformanceFrequency(freq: *mut i64) -> i32;
}

#[link(name = "user32")]
extern "system" {
    pub fn GetAsyncKeyState(vk: i32) -> i16;
}
