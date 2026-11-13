//! Windows (64-bit) side: the KH2 check, the log, and the way in (Panacea's `OnInit`).

mod airsteer;
mod combo_tables;
mod config_menu;
mod arc;
mod build;
mod crash;
mod highjump;
mod hook;
mod prty;
mod sora;
mod sys;
mod trace;

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
/// As `log_line`, but gives up if the log is busy (for the crash handler, which may run while
/// this thread holds it).
pub fn log_line_try(line: &str) {
    if let Ok(mut g) = LOG.try_lock() {
        if let Some(f) = g.as_mut() {
            let _ = writeln!(f, "{line}");
            let _ = f.flush();
        }
    }
}

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

static SETTINGS: std::sync::OnceLock<crate::settings::Settings> = std::sync::OnceLock::new();

/// The settings as the game started (`kh2-kh1.ini`; all KH2 until read).
pub fn settings() -> &'static crate::settings::Settings {
    SETTINGS.get_or_init(crate::settings::Settings::default)
}

/// Reads `kh2-kh1.ini` next to the game, writing the default one first if there is none.
fn load_settings() -> crate::settings::Settings {
    let path = format!("{}\\kh2-kh1.ini", exe_dir());
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let s = crate::settings::Settings::parse(&text);
            if crate::settings::Settings::missing_rows(&text) {
                let _ = std::fs::write(&path, s.render());
                log!("added this version's new rows to {path}");
            }
            s
        }
        Err(_) => {
            let s = crate::settings::Settings::default();
            let _ = std::fs::write(&path, s.render());
            log!("wrote the default {path}");
            s
        }
    }
}

/// Panacea calls this once a game frame (Assumption for KH2: as it does for KH1; the trace's
/// rows show it). Only the trace runs here, and only with `trace = on`.
#[no_mangle]
pub extern "C" fn OnFrame() {
    if IN_KH2.load(Ordering::Relaxed) {
        trace::frame();
    }
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
    assets(&s);
}

/// The build step (`kh1_assets`, `docs/asset-build.md`): with `combos = kh1`, KH2 Sora's motion
/// set with KH1's combo motions, built once into the mod folder (`obj/P_EX100.mset`, which
/// Panacea serves in place of the game's: Assumption, the owner's OpenKH maps the mod folder's
/// files into KH2's packages), again only when the mod's version or KH1's files change. With
/// `combos = kh2`, a file of ours there is removed, so KH2 loads its own.
fn assets(mod_path: &str) {
    if unsafe { build::detect(GetModuleHandleA(std::ptr::null()) as usize) } != build::Build::Steam || mod_path.is_empty() {
        return;
    }
    let dir = std::path::PathBuf::from(mod_path);
    let combos = settings().style("combos") == crate::settings::Style::Kh1;
    let model = settings().style("model") == crate::settings::Style::Kh1;
    let defence = settings().style("defence") == crate::settings::Style::Kh1;
    let movement = settings().style("movement") == crate::settings::Style::Kh1;
    if combos || model || defence || movement {
        let t = std::time::Instant::now();
        match kh1_assets::build_into(exe_dir(), &dir, crate::VERSION, model, combos, defence, movement) {
            Ok(Some(b)) => {
                log!("assets: built {} bytes into {} in {} ms: KH1 swings (KH1 motion, KH2 slot, frames) {:?}; model = {}", b.bytes, dir.join("obj").display(), t.elapsed().as_millis(), b.motions, if model { "kh1" } else { "kh2" });
            }
            Ok(None) => {
                log!("assets: already built ({}); model = {}", dir.join("obj").join("P_EX100.mset").display(), if model { "kh1" } else { "kh2" });
            }
            Err(e) => log!("assets: could not build: {e}"),
        }
    } else {
        let stamp = dir.join("kh2-kh1-build.txt");
        if stamp.is_file() {
            let _ = std::fs::remove_file(dir.join("obj").join("P_EX100.mset"));
            let _ = std::fs::remove_file(dir.join("obj").join("W_EX010.mset"));
            let _ = std::fs::remove_file(dir.join("obj").join("P_EX100.mdlx"));
            let _ = std::fs::remove_dir_all(dir.join("remastered").join("obj").join("P_EX100.mdlx"));
            let _ = std::fs::remove_file(&stamp);
            log!("assets: combos, defence, movement and model = kh2: our built files removed (KH2 loads its own)");
        }
    }
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
        crash::install(base);
        sora::set_base(base);
        let b = build::detect(base);
        log!("KH2 at {base:#x}, link stamp {:#010x}: {b:?} build", build::stamp(base));
        let s = SETTINGS.get_or_init(load_settings);
        for ((k, _), st) in crate::settings::SYSTEMS.iter().zip(&s.styles) {
            log!("setting {k} = {}", st.name());
        }
        if b != build::Build::Steam {
            log!("not the Steam build: nothing will be changed (Steam only for now)");
            return 1;
        }
        if s.trace {
            trace::enable(base);
        }
        config_menu::install(base);
        if s.style("movement") == crate::settings::Style::Kh1 {
            arc::install(base);
            prty::install(base);
            highjump::install(base);
            airsteer::install(base);
        }
        if s.style("combos") == crate::settings::Style::Kh1 {
            combo_tables::install(base);
        }
    }
    1
}
