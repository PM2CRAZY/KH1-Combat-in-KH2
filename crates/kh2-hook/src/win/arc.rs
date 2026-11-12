//! KH1's jump and fall arc on Sora (`movement = kh1`): KH2's own arc functions run, and for Sora
//! only (base Sora: a Drive Form keeps KH2's arc), the parts that differ from KH1 are put right
//! after them. Everything else (every other
//! object, Sora at `movement = kh2`) runs KH2's functions untouched.
//!
//! What differs (`docs/movement.md`; KH1 Checked (decomp + exe) on Steam):
//! - Gravity: KH1 0.48469388 (KH1 `0x1402AFDA0` takeoff, `0x1402AF6A0` air step, constant at
//!   KH1 RVA `0x3E9D14`) against KH2's 20/49 (KH2's one constant at RVA `0x5CBD84`, read by every
//!   object, so it isn't changed). Used in the time to the apex at launch and in the fall.
//! - Fall cap: KH1 20 per frame (KH1 RVA `0x42F968`, × the frame step) against KH2's per-actor
//!   cap `+0x138` (from `sstm`, × the frame step): Sora's `+0x138` is set to 20 when he is built.
//! The rise curve (`h · sin(π/2 · sin(π/2 · t/T))`), the fall formula `y0 + h + g/2 (t − T)²`,
//! and the frame step (60 Hz frames a game frame) are the same in both games: Checked (decomp).
//!
//! KH2 functions wrapped (first bytes checked, Steam only), Checked (decomp):
//! - `0x140405030` Sora's constructor: `+0x138` = 20 after it.
//! - `0x1403CBC30 (actor, height)`, `0x1403CBB00 (actor, &vector)`: start an arc; the time to the
//!   apex (`+0xF4`, or `+0xF0` for a negative height) again with KH1's gravity.
//! - `0x1403CB100`, `0x1403CB360` (write the frame's move), `0x1403CB500` (returns its height):
//!   the arc's step; past the apex, the frame's drop again with KH1's gravity and Sora's cap.
//! Per frame: one pointer compare per arc call; the recompute only while Sora is falling.

use super::hook;
use super::log;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::kh1::{apex_time, KH1_FALL_CAP, KH1_GRAVITY};

const SORA_NEW: usize = 0x40_5030;
const LAUNCH_H: usize = 0x3C_BC30;
const LAUNCH_V: usize = 0x3C_BB00;
const STEP_A: usize = 0x3C_B100;
const STEP_B: usize = 0x3C_B360;
const STEP_C: usize = 0x3C_B500;
const POS_OF: usize = 0x3B_5B40;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ON: AtomicBool = AtomicBool::new(false);
static O_SORA_NEW: AtomicUsize = AtomicUsize::new(0);
static O_LAUNCH_H: AtomicUsize = AtomicUsize::new(0);
static O_LAUNCH_V: AtomicUsize = AtomicUsize::new(0);
static O_STEP_A: AtomicUsize = AtomicUsize::new(0);
static O_STEP_B: AtomicUsize = AtomicUsize::new(0);
static O_STEP_C: AtomicUsize = AtomicUsize::new(0);
static SAID_LAUNCH: AtomicBool = AtomicBool::new(false);
static SAID_FALL: AtomicBool = AtomicBool::new(false);

/// Base Sora only: Drive Forms keep KH2's arc (the owner, 2026-11-12, answer 2a).
fn is_sora(actor: usize) -> bool {
    super::sora::is_base_sora(actor)
}

unsafe fn f(actor: usize, off: usize) -> *mut f32 {
    (actor + off) as *mut f32
}

/// After a launch: KH2 set the arc fields from its gravity; the time to the apex again with KH1's.
unsafe fn relaunch(actor: usize, h: f32) {
    if h < 0.0 {
        *f(actor, 0xF0) = apex_time(h);
    } else {
        *f(actor, 0xF4) = apex_time(h);
    }
}

/// The frame's height change past the apex with KH1's gravity, as KH2's step computes it: `None`
/// while rising (the rise doesn't use gravity) or when KH2's step makes no arc move.
unsafe fn fall(actor: usize, dt: f32) -> Option<f32> {
    let flags = *((actor + 0x18C) as *const u32);
    if flags & 0x0100_0020 != 0 || flags >> 6 & 1 != 0 {
        return None;
    }
    let (t, apex, y0, h) = (*f(actor, 0xF0), *f(actor, 0xF4), *f(actor, 0xF8), *f(actor, 0xFC));
    if t < apex {
        return None;
    }
    let pos_of: unsafe extern "system" fn(usize) -> *const f32 = std::mem::transmute(BASE.load(Ordering::Relaxed) + POS_OF);
    let y = *pos_of(actor).add(1);
    let d = h + y0 + (t - apex) * KH1_GRAVITY * (t - apex) * 0.5 - y;
    Some(d.min(dt * *f(actor, 0x138)))
}

/// KH2's step keeps "moving down" in `+0x124` bit 27 from the move it wrote.
unsafe fn set_down(actor: usize, d: f32) {
    let p = (actor + 0x124) as *mut u32;
    *p = if d <= 0.0 { *p & 0xF7FF_FFFF } else { *p | 0x0800_0000 };
}

fn said(what: &str) {
    let flag = if what == "launch" { &SAID_LAUNCH } else { &SAID_FALL };
    if !flag.swap(true, Ordering::Relaxed) {
        log!("movement kh1: KH1's arc on Sora ({what})");
    }
}

type NewFn = unsafe extern "system" fn(usize, usize, u64, u64, u64, u64) -> usize;
unsafe extern "system" fn sora_new(a: usize, b: usize, c: u64, d: u64, e: u64, g: u64) -> usize {
    let o: NewFn = std::mem::transmute(O_SORA_NEW.load(Ordering::Acquire));
    let r = o(a, b, c, d, e, g);
    if ON.load(Ordering::Relaxed) && a != 0 {
        if super::sora::is_base_sora(a) || *((a + 0x148) as *const usize) == 0 {
            *f(a, 0x138) = KH1_FALL_CAP;
            log!("movement kh1: Sora built at {a:#x}, fall cap 20");
        } else {
            log!("movement kh1: Sora built at {a:#x} on another prty row (a form?): KH2's fall cap kept");
        }
    }
    r
}

type LaunchH = unsafe extern "system" fn(usize, f32);
unsafe extern "system" fn launch_h(actor: usize, h: f32) {
    let o: LaunchH = std::mem::transmute(O_LAUNCH_H.load(Ordering::Acquire));
    o(actor, h);
    if ON.load(Ordering::Relaxed) && is_sora(actor) {
        relaunch(actor, h);
        said("launch");
    }
}

type LaunchV = unsafe extern "system" fn(usize, *const f32);
unsafe extern "system" fn launch_v(actor: usize, v: *const f32) {
    let h = if v.is_null() { 0.0 } else { *v.add(1) };
    let o: LaunchV = std::mem::transmute(O_LAUNCH_V.load(Ordering::Acquire));
    o(actor, v);
    if ON.load(Ordering::Relaxed) && is_sora(actor) {
        relaunch(actor, h);
        said("launch");
    }
}

type StepA = unsafe extern "system" fn(u64, *mut f32, usize, f32, u64, u64) -> *mut f32;
unsafe extern "system" fn step_a(a: u64, out: *mut f32, actor: usize, dt: f32, e: u64, g: u64) -> *mut f32 {
    let o: StepA = std::mem::transmute(O_STEP_A.load(Ordering::Acquire));
    let r = o(a, out, actor, dt, e, g);
    if ON.load(Ordering::Relaxed) && is_sora(actor) && !out.is_null() {
        if let Some(d) = fall(actor, dt) {
            *out.add(1) = d;
            set_down(actor, d);
            said("fall");
        }
    }
    r
}

type StepB = unsafe extern "system" fn(u64, *mut f32, usize, f32) -> *mut f32;
unsafe extern "system" fn step_b(a: u64, out: *mut f32, actor: usize, dt: f32) -> *mut f32 {
    let o: StepB = std::mem::transmute(O_STEP_B.load(Ordering::Acquire));
    let r = o(a, out, actor, dt);
    if ON.load(Ordering::Relaxed) && is_sora(actor) && !out.is_null() {
        if let Some(d) = fall(actor, dt) {
            *out.add(1) = d;
            set_down(actor, d);
            said("fall");
        }
    }
    r
}

type StepC = unsafe extern "system" fn(u64, usize, f32) -> f32;
unsafe extern "system" fn step_c(a: u64, actor: usize, dt: f32) -> f32 {
    let o: StepC = std::mem::transmute(O_STEP_C.load(Ordering::Acquire));
    let r = o(a, actor, dt);
    if ON.load(Ordering::Relaxed) && is_sora(actor) {
        if let Some(d) = fall(actor, dt) {
            said("fall");
            return d;
        }
    }
    r
}

/// Installs the wrappers (Steam, `movement = kh1` only). All or nothing: if any function's first
/// bytes differ, none of KH1's arc is used.
pub unsafe fn install(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    let list: [(usize, &[u8], usize, &AtomicUsize, &str); 6] = [
        (SORA_NEW, &[0x48, 0x89, 0x5C, 0x24, 0x08], sora_new as *const () as usize, &O_SORA_NEW, "Sora's constructor 0x140405030"),
        (LAUNCH_H, &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x40], launch_h as *const () as usize, &O_LAUNCH_H, "arc launch 0x1403CBC30"),
        (LAUNCH_V, &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x70], launch_v as *const () as usize, &O_LAUNCH_V, "arc launch 0x1403CBB00"),
        (STEP_A, &[0x48, 0x89, 0x5C, 0x24, 0x08], step_a as *const () as usize, &O_STEP_A, "arc step 0x1403CB100"),
        (STEP_B, &[0x48, 0x89, 0x5C, 0x24, 0x08], step_b as *const () as usize, &O_STEP_B, "arc step 0x1403CB360"),
        (STEP_C, &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x40], step_c as *const () as usize, &O_STEP_C, "arc step 0x1403CB500"),
    ];
    // (Check every function first, so a mismatch leaves all of KH2's arc as it is.)
    for (rva, expect, _, _, what) in list.iter() {
        let have = std::slice::from_raw_parts((base + rva) as *const u8, expect.len());
        if have != *expect {
            log!("movement kh1: {what} starts {have:02x?}, not {expect:02x?}: KH1's arc not installed");
            return;
        }
    }
    for (rva, expect, ours, orig, what) in list.iter() {
        if hook::install(base + rva, expect, *ours, orig, what).is_none() {
            log!("movement kh1: {what} could not be hooked");
            return;
        }
    }
    ON.store(true, Ordering::Relaxed);
    log!("movement kh1: KH1's gravity {KH1_GRAVITY} and fall cap {KH1_FALL_CAP} on Sora's arc");
}
