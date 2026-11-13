//! KH1's Dodge Roll invulnerability on base Sora (`defence = kh1`).
//!
//! KH2 (Checked (decomp + exe), the action handler `0x1403A81F0`): with growth kind 4 (Dodge Roll)
//! it requests motion `0xD1` (209, the roll), then `movss xmm1, [rax + 0x40]` at `0x1403A848D`
//! loads the level's `fmab` record's `dodge_roll_star_time` (10 / 16 / 24 / 32 by level: the OG
//! rewrite's `pref.rs`) and `0x1403D7E40` stores it as his no-damage time `+0xD70`, counted down
//! by the frame step `0x140717480` (60 Hz frames, Checked (decomp): `0x1403D4CC0`, `0x1403D68B0`; hits check it
//! is ≤ 0). Quick Run reaches the same store from `0x1403A8492`, after that load, so only the
//! roll passes through it.
//!
//! KH1 (the sister project's `kh1-roll-guard-native.md` §5, Checked (decomp) there): the roll
//! start `0x1402C9E60` gives 12 frames of no damage (`0x140297F30(actor, 12.0)`), at every level
//! (KH1's Dodge Roll has none), in 60 Hz frames (KH1's frame step, Stage 4).
//!
//! So the load becomes a call into a stub of ours that loads the same value and, for base Sora,
//! hands back KH1's 12. Forms (Limit Form's roll) keep KH2's. Nothing runs per frame: only on a
//! roll.
//!
//! ## Square: guard or roll (t52)
//!
//! KH1 (Checked (decomp), the Square handler `0x1402A7B30`): on the ground Square guards with Guard
//! equipped when Dodge Roll isn't, or when his move speed is under his run speed (`move +0xC <
//! stats +8`); else it rolls. His move speed is his walk speed (stats `+4`, 2) at a tilt up to 0.8
//! and his run speed (`+8`, 8) past it (`0x1402A5F10`, the ground state's step, registered by
//! `0x1402B1B80(0, ...)`, Checked (decomp)). So KH1 guards standing or walking, rolls running.
//!
//! KH2 (Checked (decomp + exe), `0x1403A81F0`): guards only with the move speed `+0x1C` at 0
//! (`0x1403A83C8`: `movss xmm0, [rdi+0x1C]; ucomiss xmm0, xmm7; jp; jne`), else rolls; without
//! Dodge Roll it guards at any tilt (as KH1). Its move speed is the same pick: walk `+0x128` under
//! 0.8 tilt, run `+0x12C` from it (`0x1403A8FA0`, Checked (decomp)).
//!
//! So that test becomes a call into a stub of ours: base Sora guards when `+0x1C` is under his run
//! speed `+0x12C` (KH1's rule), anyone else when it is 0 (KH2's). KH1's tilt boundary is ≤ 0.8,
//! KH2's < 0.8: a tilt of exactly 0.8 walks in KH2 (one value, not ported).

use super::hook;
use super::log;
use super::sora::is_base_sora;
use std::sync::atomic::{AtomicBool, Ordering};

const LOAD: usize = 0x3A_848D;
const LOAD_BYTES: [u8; 5] = [0xF3, 0x0F, 0x10, 0x48, 0x40];
/// KH1's roll: 12 frames without damage.
pub const KH1_ROLL_STAR_TIME: f32 = 12.0;

static LOGGED: AtomicBool = AtomicBool::new(false);

extern "system" fn star_time(actor: usize, kh2: f32) -> f32 {
    if !is_base_sora(actor) {
        return kh2;
    }
    if !LOGGED.swap(true, Ordering::Relaxed) {
        log!("defence kh1: Dodge Roll no-damage time {KH1_ROLL_STAR_TIME} (KH2's at this level {kh2})");
    }
    KH1_ROLL_STAR_TIME
}

const SQUARE_TEST: usize = 0x3A_83C8;
const SQUARE_TEST_BYTES: [u8; 12] = [0xF3, 0x0F, 0x10, 0x47, 0x1C, 0x0F, 0x2E, 0xC7, 0x7A, 0x12, 0x75, 0x10];

static GUARD_LOGGED: AtomicBool = AtomicBool::new(false);

/// Whether Square guards now (KH2 then tries its guard; otherwise the roll and the rest).
extern "system" fn square_guards(actor: usize) -> u8 {
    let speed = unsafe { *((actor + 0x1C) as *const f32) };
    if !is_base_sora(actor) {
        return (speed == 0.0) as u8;
    }
    let run = unsafe { *((actor + 0x12C) as *const f32) };
    if speed != 0.0 && speed < run && !GUARD_LOGGED.swap(true, Ordering::Relaxed) {
        log!("defence kh1: Square guards while walking (speed {speed} under his run {run}), as KH1");
    }
    (speed < run) as u8
}

unsafe fn install_square(base: usize) {
    let site = base + SQUARE_TEST;
    let Some(stub) = hook::alloc_near(site, 64) else {
        log!("defence kh1: no memory near the Square test; KH2's guard rule kept");
        return;
    };
    // mov rcx, rdi; sub rsp, 0x28; call [rip+5]; add rsp, 0x28; ret; dq square_guards
    let mut code: Vec<u8> = vec![0x48, 0x89, 0xF9, 0x48, 0x83, 0xEC, 0x28, 0xFF, 0x15, 0x05, 0x00, 0x00, 0x00, 0x48, 0x83, 0xC4, 0x28, 0xC3];
    code.extend_from_slice(&(square_guards as *const () as usize as u64).to_le_bytes());
    std::ptr::copy_nonoverlapping(code.as_ptr(), stub as *mut u8, code.len());
    // call stub; test al, al; je 0x1403A83E4 (KH2's "not guarding" path); nop x3
    let mut new = [0x90u8; 12];
    new[0] = 0xE8;
    new[1..5].copy_from_slice(&((stub as i64 - (site as i64 + 5)) as i32).to_le_bytes());
    new[5..9].copy_from_slice(&[0x84, 0xC0, 0x74, 0x13]);
    if hook::patch(site, &SQUARE_TEST_BYTES, &new, "the Square guard test 0x1403A83C8") {
        log!("defence kh1: KH1's Square rule on base Sora (guard below his run speed; test 0x1403A83C8 -> {stub:#x})");
    }
}

pub unsafe fn install(base: usize) {
    install_square(base);
    let site = base + LOAD;
    let Some(stub) = hook::alloc_near(site, 64) else {
        log!("defence kh1: no memory near the roll's star time load; KH2's roll time kept");
        return;
    };
    // movss xmm1, [rax+0x40]; mov rcx, rdi; sub rsp, 0x28; call [rip+8];
    // add rsp, 0x28; movaps xmm1, xmm0; ret; dq star_time
    let mut code: Vec<u8> = vec![0xF3, 0x0F, 0x10, 0x48, 0x40, 0x48, 0x89, 0xF9, 0x48, 0x83, 0xEC, 0x28, 0xFF, 0x15, 0x08, 0x00, 0x00, 0x00, 0x48, 0x83, 0xC4, 0x28, 0x0F, 0x28, 0xC8, 0xC3];
    code.extend_from_slice(&(star_time as *const () as usize as u64).to_le_bytes());
    std::ptr::copy_nonoverlapping(code.as_ptr(), stub as *mut u8, code.len());
    let mut call = [0u8; 5];
    call[0] = 0xE8;
    call[1..].copy_from_slice(&((stub as i64 - (site as i64 + 5)) as i32).to_le_bytes());
    if hook::patch(site, &LOAD_BYTES, &call, "the roll's star time load 0x1403A848D") {
        log!("defence kh1: KH1's roll no-damage time on base Sora (load 0x1403A848D -> {stub:#x})");
    }
}
