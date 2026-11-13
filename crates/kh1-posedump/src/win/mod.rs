mod hook;
mod sys;

use std::collections::HashSet;
use std::ffi::c_void;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use sys::*;

const KH1_EXE: &str = "KINGDOM HEARTS FINAL MIX.exe";
const POSE_RVA: usize = 0x391C70;
const POSE_PROLOGUE: [u8; 7] = [0x4C, 0x8B, 0xDC, 0x53, 0x56, 0x41, 0x54];
/// The pass for characters with ground IK, `(world matrix, instance, flags)` (Steam
/// `0x140392560`; the motion and time are the instance's `+8` / `+0x10`). Checked (decomp, exe).
const POSE_IK_RVA: usize = 0x392560;
const POSE_IK_PROLOGUE: [u8; 9] = [0x4C, 0x8B, 0xDC, 0x55, 0x53, 0x41, 0x54, 0x41, 0x57];
static ORIG_IK: AtomicUsize = AtomicUsize::new(0);
const HANDLE_TABLE_RVA: usize = 0x2EE3980;
const SORA_BONES: u16 = 293;
const VK_F10: i32 = 0x79;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG: AtomicUsize = AtomicUsize::new(0);
static RECORDING: AtomicBool = AtomicBool::new(false);
static KEY_DOWN: AtomicBool = AtomicBool::new(false);
static CALLS: AtomicUsize = AtomicUsize::new(0);
static CALLS_IK: AtomicUsize = AtomicUsize::new(0);
static LOG: Mutex<Option<File>> = Mutex::new(None);
static OUT: Mutex<Option<(BufWriter<File>, HashSet<usize>, u64)>> = Mutex::new(None);

pub fn log_line(line: &str) {
    if let Some(f) = LOG.lock().unwrap().as_mut() {
        let _ = writeln!(f, "{line}");
        let _ = f.flush();
    }
}

macro_rules! log {
    ($($t:tt)*) => { $crate::win::log_line(&::std::format!($($t)*)) };
}
pub(crate) use log;

fn exe_path() -> String {
    let mut buf = [0u16; 1024];
    let n = unsafe { GetModuleFileNameW(std::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32) } as usize;
    String::from_utf16_lossy(&buf[..n])
}

fn exe_dir() -> String {
    let p = exe_path();
    p.rsplit_once('\\').map(|(d, _)| d.to_string()).unwrap_or(p)
}

/// Reads `len` bytes at `at` without faulting (None if any byte isn't readable).
fn read(at: usize, len: usize) -> Option<Vec<u8>> {
    if at < 0x10000 {
        return None;
    }
    let mut buf = vec![0u8; len];
    let mut got = 0usize;
    let ok = unsafe { ReadProcessMemory(GetCurrentProcess(), at as *const c_void, buf.as_mut_ptr() as *mut c_void, len, &mut got) };
    (ok != 0 && got == len).then_some(buf)
}

fn read_u64(at: usize) -> Option<usize> {
    read(at, 8).map(|b| u64::from_le_bytes(b.try_into().unwrap()) as usize)
}

fn read_u32(at: usize) -> Option<u32> {
    read(at, 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()))
}

/// A KH1 handle to a pointer.
fn ptr(h: u32) -> Option<usize> {
    if h == 0 {
        return None;
    }
    let table = BASE.load(Ordering::Relaxed) + HANDLE_TABLE_RVA;
    let hi = read_u64(table + (((h & 0x7FFF_FFFF) >> 25) as usize) * 8)?;
    Some(hi | (h & 0x1FF_FFFF) as usize)
}

fn bone_count(instance: usize) -> Option<u16> {
    let slot = read_u64(instance)?;
    let header = read_u64(slot)?;
    read(header, 2).map(|b| u16::from_le_bytes([b[0], b[1]]))
}

fn toggle_key() {
    let down = unsafe { GetAsyncKeyState(VK_F10) } as u16 & 0x8000 != 0;
    let was = KEY_DOWN.swap(down, Ordering::Relaxed);
    if !down || was {
        return;
    }
    let mut out = OUT.lock().unwrap();
    if RECORDING.load(Ordering::Relaxed) {
        RECORDING.store(false, Ordering::Relaxed);
        if let Some((mut w, motions, frames)) = out.take() {
            let _ = w.flush();
            log!("recording stopped: {frames} Sora frames, {} motions", motions.len());
        }
    } else {
        let path = format!("{}\\kh1-posedump.bin", exe_dir());
        match File::create(&path) {
            Ok(f) => {
                let mut w = BufWriter::with_capacity(1 << 20, f);
                let _ = w.write_all(b"KH1POSE1");
                *out = Some((w, HashSet::new(), 0));
                RECORDING.store(true, Ordering::Relaxed);
                log!("recording into {path}");
            }
            Err(e) => log!("can't create {path}: {e}"),
        }
    }
}

fn record(tag: u32, world: usize, instance: usize, t: f32, motion: usize) -> Option<()> {
    if bone_count(instance)? != SORA_BONES || motion == 0 {
        return None;
    }
    let n = SORA_BONES as usize;
    let wm = read(ptr(read_u32(instance + 0x1B4)?)?, n * 64)?;
    let lm = read(ptr(read_u32(instance + 0x1B8)?)?, n * 64)?;
    let arg = read(world, 64)?;
    let mut g = OUT.lock().unwrap();
    let (w, motions, frames) = g.as_mut()?;
    if motions.insert(motion) {
        let header = read(motion, 0x40)?;
        let words = ptr(read_u32(motion + 0x14)?).and_then(|p| read(p, n * 4)).unwrap_or_else(|| vec![0; n * 4]);
        let _ = w.write_all(&1u32.to_le_bytes());
        let _ = w.write_all(&(motion as u64).to_le_bytes());
        let _ = w.write_all(&header);
        let _ = w.write_all(&(n as u32).to_le_bytes());
        let _ = w.write_all(&words);
    }
    let _ = w.write_all(&tag.to_le_bytes());
    let _ = w.write_all(&(instance as u64).to_le_bytes());
    let _ = w.write_all(&(motion as u64).to_le_bytes());
    let _ = w.write_all(&t.to_le_bytes());
    let _ = w.write_all(&(n as u32).to_le_bytes());
    let _ = w.write_all(&arg);
    let _ = w.write_all(&wm);
    let _ = w.write_all(&lm);
    *frames += 1;
    Some(())
}

/// Logs a hook's first call, and every 3600th (about a minute), so the log shows it runs.
fn note_call(n: &AtomicUsize, what: &str) {
    let c = n.fetch_add(1, Ordering::Relaxed);
    if c == 0 || (c + 1) % 3600 == 0 {
        log!("{what}: {} calls", c + 1);
    }
}

/// Watches F10 (every 30 ms) on its own thread, so a press counts whatever the game is doing.
unsafe extern "system" fn keys(_: *mut c_void) -> u32 {
    loop {
        toggle_key();
        Sleep(30);
    }
}

type Pose = unsafe extern "system" fn(usize, usize, f32, usize);

unsafe extern "system" fn pose(world: usize, instance: usize, t: f32, motion: usize) {
    let f: Pose = std::mem::transmute(ORIG.load(Ordering::Acquire));
    f(world, instance, t, motion);
    note_call(&CALLS, "skeleton pass");
    if RECORDING.load(Ordering::Relaxed) && instance != 0 {
        let _ = record(2, world, instance, t, motion);
    }
}

type PoseIk = unsafe extern "system" fn(usize, usize, f32);

unsafe extern "system" fn pose_ik(world: usize, instance: usize, flags: f32) {
    let f: PoseIk = std::mem::transmute(ORIG_IK.load(Ordering::Acquire));
    f(world, instance, flags);
    note_call(&CALLS_IK, "ground-IK pass");
    if RECORDING.load(Ordering::Relaxed) && instance != 0 {
        let motion = read_u64(instance + 8).unwrap_or(0);
        let t = read(instance + 0x10, 4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).unwrap_or(-1.0);
        let _ = record(3, world, instance, t, motion);
    }
}

#[no_mangle]
pub extern "C" fn OnFrame() {}

#[no_mangle]
pub extern "C" fn OnInit(_mod_path: *const u16) {}

#[no_mangle]
pub unsafe extern "system" fn DllMain(_inst: *mut c_void, reason: u32, _reserved: *mut c_void) -> i32 {
    if reason != 1 {
        return 1;
    }
    let exe = exe_path();
    if !exe.rsplit('\\').next().is_some_and(|n| n.eq_ignore_ascii_case(KH1_EXE)) {
        return 1;
    }
    if let Ok(f) = OpenOptions::new().create(true).write(true).truncate(true).open(format!("{}\\kh1-posedump.log", exe_dir())) {
        *LOG.lock().unwrap() = Some(f);
    }
    let base = GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    log!("kh1-posedump loaded in {exe} (KH1 at {base:#x}); F10 starts and stops a recording");
    if hook::install(base + POSE_RVA, &POSE_PROLOGUE, pose as *const () as usize, &ORIG, "KH1 skeleton pass 0x140391C70").is_some() {
        log!("hooked KH1's skeleton pass");
    }
    CreateThread(std::ptr::null_mut(), 0, keys, std::ptr::null_mut(), 0, std::ptr::null_mut());
    if hook::install(base + POSE_IK_RVA, &POSE_IK_PROLOGUE, pose_ik as *const () as usize, &ORIG_IK, "KH1 skeleton pass with ground IK 0x140392560").is_some() {
        log!("hooked KH1's skeleton pass with ground IK");
    }
    1
}
