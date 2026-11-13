//! KH1's air steering on base Sora (`movement = kh1`).
//!
//! KH2's air control `0x1403B7AD0 (actor, &out, step, keep, drag)`, Checked (decomp): with the
//! stick held (`+0x1C`, the stick's speed, > 0) Sora's air velocity `+0x20` eases toward the stick's
//! direction `+0x10` × that speed by `keep`; let go, it is × `drag`; `out` = the velocity × the frame
//! step. Sora's plain air moves call it with keep 0.9 and drag 0.95 (`0x1403CB360`), once a 30 Hz
//! step (step 2, Checked (trace) t3).
//!
//! KH1 (`0x1402AF6A0`, Checked (decomp); the numbers Checked (trace)): held, `v += dir × 0.1 ×
//! speed` each 60 Hz frame, its length kept to the larger of its old length and the run (8); let
//! go, `v ×= 0.95` a 30 Hz frame. So the drag is already KH1's (0.95 a 2-frame step); only the
//! held branch is replaced, for base Sora, on the plain call (0.9 / 0.95) only: KH2's own moves
//! that pass other values keep theirs. Per call: KH1's frame rule once per 60 Hz frame of the step.

use super::hook;
use super::log;
use super::sora::is_base_sora;
use crate::kh1::air_steer;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const AIR: usize = 0x3B_7AD0;

static O_AIR: AtomicUsize = AtomicUsize::new(0);
static SAID: AtomicBool = AtomicBool::new(false);

type AirFn = unsafe extern "system" fn(usize, *mut f32, f32, f32, f32) -> *mut f32;
unsafe extern "system" fn air(actor: usize, out: *mut f32, step: f32, keep: f32, drag: f32) -> *mut f32 {
    let plain = (keep - 0.9).abs() < 1e-4 && (drag - 0.95).abs() < 1e-4;
    let v = (actor + 0x20) as *mut f32;
    let old = if actor != 0 { [*v, *v.add(2)] } else { [0.0, 0.0] };
    let o: AirFn = std::mem::transmute(O_AIR.load(Ordering::Acquire));
    let r = o(actor, out, step, keep, drag);
    if !plain || out.is_null() || !crate::settings::kh1(crate::settings::MOVEMENT) || !is_base_sora(actor) {
        return r;
    }
    let speed = *((actor + 0x1C) as *const f32);
    if speed <= 0.0 {
        return r;
    }
    let dir = [*((actor + 0x10) as *const f32), *((actor + 0x18) as *const f32)];
    let run = *((actor + 0x12C) as *const f32);
    let frames = step.round().max(1.0) as u32;
    let mut n = old;
    for _ in 0..frames {
        n = air_steer(n, dir, speed, run);
    }
    *v = n[0];
    *v.add(2) = n[1];
    *out = n[0] * step;
    *out.add(2) = n[1] * step;
    if !SAID.swap(true, Ordering::Relaxed) {
        log!("movement kh1: KH1's air steering on Sora (speed {speed}, run {run}, step {step})");
    }
    r
}

pub unsafe fn install(base: usize) {
    let ours = air as *const () as usize;
    if hook::install(base + AIR, &[0x48, 0x8B, 0xC4, 0x48, 0x89, 0x58, 0x08], ours, &O_AIR, "air control 0x1403B7AD0").is_none() {
        log!("movement kh1: KH1's air steering not used");
    }
}
