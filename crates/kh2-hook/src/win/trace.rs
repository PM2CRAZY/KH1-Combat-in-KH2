//! The debug trace (`trace = on` in `kh2-kh1.ini`): F10 starts and stops a recording of Sora,
//! one row per game frame, written to `kh2-kh1-trace-N.csv` next to the game when it stops.
//! It runs from Panacea's `OnFrame` and reads memory only (no hook, nothing changed); off, it
//! costs one flag check a frame.
//!
//! Sora's object: KH2's SORA constructor `0x140405030` stores it at `0x142AE9A28` (read back by
//! `0x140405180`, cleared by `0x140404BB0`). Checked (decomp).
//! The fields read are leads to confirm with this trace (Assumption each):
//! - `+0x70` and `+0x670`: a position (KH2's `0x1403B5C50` picks one of the two by whether the
//!   object is attached to another, `+0x6A0`);
//! - `+0x128..+0x134`: the movement block KH2's `0x14040A480` fills from the `prty` row (walk,
//!   run, jump height, turn rate), Checked (decomp) for the layout;
//! - `+0x180`: the motion playing (the combo picker `0x140402C50` compares rows with it);
//! - `+0xDE0`: a state number (the update `0x140404A40` tests it against 0xC).
//!
//! The field camera `0x140718C60` (Checked (decomp): `0x140164F00` runs its update and copies its
//! eye `+0x18` and look `+0x08` out): its mode `+0x48`, drawn eye, drawn look and FOV `+0x28`
//! (radians across KH2's 512-wide view).

use super::sys::*;
use super::{exe_dir, log};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

const SORA_PTR: usize = 0x2AE_9A28; // RVA of 0x142AE9A28
const CAMERA: usize = 0x71_8C60; // RVA of 0x140718C60
const VK_F10: i32 = 0x79;

static ON: AtomicBool = AtomicBool::new(false);
static RECORDING: AtomicBool = AtomicBool::new(false);
static WAS_DOWN: AtomicBool = AtomicBool::new(false);
static FILE_NO: AtomicU32 = AtomicU32::new(0);
static FRAME: AtomicU32 = AtomicU32::new(0);
static ROWS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static BASE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub fn enable(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    ON.store(true, Ordering::Relaxed);
    log!("trace on: F10 starts and stops a recording");
}

/// Reads `N` bytes at `at` without faulting (a bad pointer gives `None`).
fn read<const N: usize>(at: usize) -> Option<[u8; N]> {
    let mut buf = [0u8; N];
    let mut got = 0usize;
    let ok = unsafe { ReadProcessMemory(GetCurrentProcess(), at as *const c_void, buf.as_mut_ptr() as *mut c_void, N, &mut got) };
    (ok != 0 && got == N).then_some(buf)
}
fn f32_at(at: usize) -> Option<f32> {
    read::<4>(at).map(f32::from_le_bytes)
}
fn u32_at(at: usize) -> Option<u32> {
    read::<4>(at).map(u32::from_le_bytes)
}

fn now_us() -> f64 {
    let (mut c, mut f) = (0i64, 0i64);
    unsafe {
        QueryPerformanceCounter(&mut c);
        QueryPerformanceFrequency(&mut f);
    }
    if f == 0 { 0.0 } else { c as f64 * 1e6 / f as f64 }
}

const HEADER: &str = "frame,time_us,sora,x70,y70,z70,x670,y670,z670,walk,run,jump,turn,motion180,state_de0,cam_mode,eye_x,eye_y,eye_z,look_x,look_y,look_z,fov";

/// Once a game frame (Panacea's `OnFrame`).
pub fn frame() {
    if !ON.load(Ordering::Relaxed) {
        return;
    }
    let down = unsafe { GetAsyncKeyState(VK_F10) } as u16 & 0x8000 != 0;
    if down && !WAS_DOWN.load(Ordering::Relaxed) {
        if RECORDING.swap(false, Ordering::Relaxed) {
            stop();
        } else {
            FRAME.store(0, Ordering::Relaxed);
            ROWS.lock().unwrap().clear();
            RECORDING.store(true, Ordering::Relaxed);
            log!("trace: recording");
        }
    }
    WAS_DOWN.store(down, Ordering::Relaxed);
    if !RECORDING.load(Ordering::Relaxed) {
        return;
    }
    let n = FRAME.fetch_add(1, Ordering::Relaxed);
    let base = BASE.load(Ordering::Relaxed);
    let sora = read::<8>(base + SORA_PTR).map(u64::from_le_bytes).unwrap_or(0) as usize;
    let f = |o: usize| if sora == 0 { String::new() } else { f32_at(sora + o).map_or(String::new(), |v| format!("{v:.3}")) };
    let u = |o: usize| if sora == 0 { String::new() } else { u32_at(sora + o).map_or(String::new(), |v| format!("{v:#x}")) };
    let cam = base + CAMERA;
    let c = |o: usize| f32_at(cam + o).map_or(String::new(), |v| format!("{v:.3}"));
    let mode = u32_at(cam + 0x48).map_or(String::new(), |v| v.to_string());
    let row = format!(
        "{n},{:.0},{sora:#x},{},{},{},{},{},{},{},{},{},{},{},{},{mode},{},{},{},{},{},{},{}",
        now_us(),
        f(0x70), f(0x74), f(0x78), f(0x670), f(0x674), f(0x678),
        f(0x128), f(0x12C), f(0x130), f(0x134), u(0x180), u(0xDE0),
        c(0x18), c(0x1C), c(0x20), c(0x08), c(0x0C), c(0x10), c(0x28)
    );
    let mut rows = ROWS.lock().unwrap();
    rows.push(row);
    // (A safety cap: about 5 minutes at 60 fps.)
    if rows.len() >= 18_000 {
        drop(rows);
        RECORDING.store(false, Ordering::Relaxed);
        stop();
    }
}

fn stop() {
    let rows = std::mem::take(&mut *ROWS.lock().unwrap());
    let no = FILE_NO.fetch_add(1, Ordering::Relaxed) + 1;
    let path = format!("{}\\kh2-kh1-trace-{no}.csv", exe_dir());
    let mut text = String::with_capacity(rows.len() * 120 + 100);
    text.push_str(HEADER);
    text.push('\n');
    for r in &rows {
        text.push_str(r);
        text.push('\n');
    }
    match std::fs::write(&path, text) {
        Ok(()) => log!("trace: {} frames written to {path}", rows.len()),
        Err(e) => log!("trace: could not write {path}: {e}"),
    }
}
