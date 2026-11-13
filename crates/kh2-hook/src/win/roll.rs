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

pub unsafe fn install(base: usize) {
    let site = base + LOAD;
    let Some(stub) = hook::alloc_near(site, 64) else {
        log!("defence kh1: no memory near the roll's star time load; KH2's roll time kept");
        return;
    };
    // movss xmm1, [rax+0x40]; mov rcx, rdi; sub rsp, 0x28; call [rip+8];
    // add rsp, 0x28; movaps xmm1, xmm0; ret; dq star_time
    let mut code: Vec<u8> = vec![0xF3, 0x0F, 0x10, 0x48, 0x40, 0x48, 0x89, 0xF9, 0x48, 0x83, 0xEC, 0x28, 0xFF, 0x15, 0x08, 0x00, 0x00, 0x00, 0x48, 0x83, 0xC4, 0x28, 0x0F, 0x28, 0xC8, 0xC3];
    code.extend_from_slice(&(star_time as usize as u64).to_le_bytes());
    std::ptr::copy_nonoverlapping(code.as_ptr(), stub as *mut u8, code.len());
    let mut call = [0u8; 5];
    call[0] = 0xE8;
    call[1..].copy_from_slice(&((stub as i64 - (site as i64 + 5)) as i32).to_le_bytes());
    if hook::patch(site, &LOAD_BYTES, &call, "the roll's star time load 0x1403A848D") {
        log!("defence kh1: KH1's roll no-damage time on base Sora (load 0x1403A848D -> {stub:#x})");
    }
}
