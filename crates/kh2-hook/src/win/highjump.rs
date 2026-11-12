//! KH1's High Jump on base Sora (`movement = kh1`; the owner, 2026-11-12, answer 1a: any KH2 High
//! Jump level gives KH1's height). KH2 asks Sora's jump height of `0x1403B59B0 (actor, &motion)`
//! (the jump) and `0x1403B5A40 (actor)`: his own `+0x130` (the `prty` row's), or with a High Jump
//! level his growth row's (`fmab`, `0x1403E92D0`) height and the flip motion 201. Checked
//! (decomp). `fmab` is shared with the Drive Forms, so it isn't changed: for base Sora (not in a
//! form) a High Jump height becomes KH1's 290 (Checked (trace), `kh1.rs`); his plain jump is
//! already KH1's 190 through his `prty` row (`prty.rs`).

use super::hook;
use super::log;
use super::sora::is_base_sora;
use crate::kh1::KH1_HIGH_JUMP;
use std::sync::atomic::{AtomicUsize, Ordering};

const HEIGHT_A: usize = 0x3B_59B0;
const HEIGHT_B: usize = 0x3B_5A40;

static O_A: AtomicUsize = AtomicUsize::new(0);
static O_B: AtomicUsize = AtomicUsize::new(0);

/// A height other than his own `+0x130` is a High Jump's.
unsafe fn kh1(actor: usize, h: f32) -> f32 {
    if is_base_sora(actor) && h != *((actor + 0x130) as *const f32) {
        KH1_HIGH_JUMP
    } else {
        h
    }
}

type AFn = unsafe extern "system" fn(usize, *mut u32) -> f32;
unsafe extern "system" fn height_a(actor: usize, motion: *mut u32) -> f32 {
    let o: AFn = std::mem::transmute(O_A.load(Ordering::Acquire));
    kh1(actor, o(actor, motion))
}

type BFn = unsafe extern "system" fn(usize) -> f32;
unsafe extern "system" fn height_b(actor: usize) -> f32 {
    let o: BFn = std::mem::transmute(O_B.load(Ordering::Acquire));
    kh1(actor, o(actor))
}

pub unsafe fn install(base: usize) {
    let a = hook::install(base + HEIGHT_A, &[0x48, 0x89, 0x5C, 0x24, 0x08], height_a as *const () as usize, &O_A, "jump height 0x1403B59B0");
    let b = a.is_some() && hook::install(base + HEIGHT_B, &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x30], height_b as *const () as usize, &O_B, "jump height 0x1403B5A40").is_some();
    if b {
        log!("movement kh1: High Jump at any level is KH1's {KH1_HIGH_JUMP} on base Sora");
    } else {
        log!("movement kh1: KH1's High Jump not used");
    }
}
