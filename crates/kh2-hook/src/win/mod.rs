//! Windows (64-bit) side: the KH2 check, the log, and the way in (Panacea's `OnInit`).

mod sys;

use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use sys::*;

pub const KH2_EXE: &str = "KINGDOM HEARTS II FINAL MIX.exe";

static LOG: Mutex<Option<std::fs::File>> = Mutex::new(None);
/// The host is KH2 (anything else: we do nothing).
static IN_KH2: AtomicBool = AtomicBool::new(false);

/// Writes a line to `kh2-kh1.log` next to the game.
pub fn log_line(line: &str) {
    if let Ok(mut g) = LOG.lock() {
        if let Some(f) = g.as_mut() {
            let _ = writeln!(f, "{line}");
            let _ = f.flush();
        }
    }
}

macro_rules! log {
    ($($t:tt)*) => { $crate::win::log_line(&::std::format!($($t)*)) };
}
#[allow(unused_imports)]
pub(crate) use log;

fn exe_path() -> String {
    let mut buf = [0u16; 1024];
    let n = unsafe { GetModuleFileNameW(std::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32) } as usize;
    String::from_utf16_lossy(&buf[..n])
}

pub fn exe_dir() -> String {
    let p = exe_path();
    p.rsplit_once('\\').map_or(".".to_string(), |(d, _)| d.to_string())
}

/// The PE header's link stamp (`TimeDateStamp`) of the loaded exe: which build of KH2 this is.
/// Read for the log only; the Steam and Epic stamps get matched once the first hook needs them
/// (Assumption until then: the stamp tells the two builds apart, as it does for KH1).
unsafe fn link_stamp(base: usize) -> u32 {
    let pe = base + *((base + 0x3C) as *const u32) as usize;
    *((pe + 8) as *const u32)
}

/// Panacea calls this with the game's mod folder once our DLL is loaded.
#[no_mangle]
pub unsafe extern "C" fn OnInit(mod_path: *const u16) {
    if !IN_KH2.load(Ordering::Relaxed) {
        return;
    }
    let mut s = String::new();
    if !mod_path.is_null() {
        let n = (0..1024).take_while(|&i| *mod_path.add(i) != 0).count();
        s = String::from_utf16_lossy(std::slice::from_raw_parts(mod_path, n));
    }
    log!("loaded by Panacea (mod folder {s})");
}

#[no_mangle]
pub unsafe extern "system" fn DllMain(_inst: *mut c_void, reason: u32, _reserved: *mut c_void) -> i32 {
    if reason == 1 {
        let exe = exe_path();
        let kh2 = exe.rsplit('\\').next().is_some_and(|n| n.eq_ignore_ascii_case(KH2_EXE));
        IN_KH2.store(kh2, Ordering::Relaxed);
        if !kh2 {
            return 1;
        }
        let path = format!("{}\\kh2-kh1.log", exe_dir());
        let _ = std::fs::rename(&path, format!("{}\\kh2-kh1.prev.log", exe_dir()));
        if let Ok(f) = OpenOptions::new().create(true).write(true).truncate(true).open(&path) {
            *LOG.lock().unwrap() = Some(f);
        }
        log!("kh2-kh1 {} loaded (build {}) in {exe}", crate::VERSION, env!("KH2_KH1_BUILD"));
        std::panic::set_hook(Box::new(|info| log!("PANIC in kh2-kh1: {info}")));
        let base = GetModuleHandleA(std::ptr::null()) as usize;
        log!("KH2 at {base:#x}, link stamp {:#010x}; no hooks in this version, nothing was changed", link_stamp(base));
    }
    1
}
