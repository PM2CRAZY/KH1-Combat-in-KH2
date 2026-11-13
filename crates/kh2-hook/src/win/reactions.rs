//! KH1's hit reactions on base Sora, in KH2's own reaction routine (`defence = kh1`; the owner,
//! 2026-11-13: KH1's rule, KH1's knock-down, KH1's 30 frames after).
//!
//! KH2 (Checked (decomp + exe)): every hit on an object goes through `0x1403D2FE0 (object, hit)`.
//! The hit record (a pool of 0x40-byte records at `0x142A24530`, filled by `0x1403CE740`): `+0x0`
//! the push direction, `+0x18` bit 0 "no effect", `+0x20` the ATKP handle, `+0x26` the reaction
//! (ATKP `+0xA`), `+0x28` the damage (`0x1403C12F0`'s). The reaction is folded by `0x1403CE980`
//! (1, 0xC → 1 push; 3, 6, 8, 0xA, 0xD, 0x10, 0x12 → 3 flinch and push; 4, 7, 9, 0xB, 0xF, 0x11 →
//! 4 launch; 2 flinch; the rest special). Case 3 on the ground is the flinch `0x1403D43B0` (motion
//! 10 / 12 by `dot(facing, push) < 0`) and the push `0x1403D4960` by the hit-back `0x1403CEAB0`; in
//! the air (`0x1403BA9A0`) KH2's air flinch. The target's status is at `+0x5C0` (`[0]` HP, `[1]`
//! max HP, `0x1403C12F0`'s low-HP test).
//!
//! KH1 (Checked (decomp)): a normal hit's reaction is picked by its damage's share of max HP
//! (`0x1402BF290`: `(int)(damage / max × 100)` under btltbl's 20 a flinch, else a knock-back);
//! a flinch pushes him `750 × share`, at most 150 (`0x14029D210`, the sister project's
//! `kh1-hit-reactions-native.md` §4, Checked (files) there); a ground knock-back is a knock-down
//! in place (`0x14029CEC0`: motion `0x4C` / `0x4D`, slots 65 / 66); in the air a knock-back
//! launches him (`0x14029CC40`). Every reaction gives 30 frames without damage, counted once he
//! can act again (`0x140297F30(actor, 30.0)`, §3 there).
//!
//! So, for base Sora's hits of reaction groups 1..4 (specials, Magnet and the rest, stay KH2's):
//! - under 20 %: KH2's flinch and push (group 3), the push KH1's distance (through a hook on the
//!   hit-back, for this hit only);
//! - 20 % and over, on the ground: KH2's flinch with no push (group 2), then KH1's knock-down
//!   motion, ids 20 / 21 by the same front / behind test (`kh1_assets::KNOCKDOWN`, built into
//!   Sora's motion set);
//! - 20 % and over, in the air: KH2's launch (group 4), with KH2's own launch numbers and motions
//!   for now (KH1's air knock-back: a hop of 20 and 16 sideways, slots 69 / 70: not yet);
//! - after it, KH2's no-damage time `+0xD70` (60 Hz frames, see `roll.rs`) at least KH1's 30
//!   frames past when he can act again: the flinch's "may be left" 46, the air flinch's end 30, the
//!   knock-down's 58 (Checked (files): KH1's slots). Assumption: KH1's count starts there.
//! Not ported: KH1's push curve (0.22314 of the distance a step, × 0.8) for KH2's (0.1, × 0.9):
//! the same distance, a different shape. Runs once per hit on Sora; nothing per frame.

use super::hook;
use super::log;
use super::sora::is_base_sora;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const REACTION: usize = 0x3D_2FE0;
const HITBACK: usize = 0x3C_EAB0;
const IN_AIR: usize = 0x3B_A9A0;
const FACING: usize = 0x3B_56E0;
const MOTION_HAS: usize = 0x3C_7D30;
const MOTION_REQUEST: usize = 0x3C_86A0;
const MOTION_START: usize = 0x3C_8310;

/// KH1's reaction block (btltbl `+0x58`, Checked (files)): flinch under 20 % of max HP; push 750 ×
/// share, at most 150.
const KH1_FLINCH_SHARE: i32 = 20;
const KH1_PUSH_PER_MAX: f32 = 750.0;
const KH1_PUSH_CAP: f32 = 150.0;
/// KH1's no-damage frames after a reaction, and when he may act in each (Checked (files)).
const KH1_MERCY: f32 = 30.0;
const KH1_FLINCH_FREE: f32 = 46.0;
const KH1_AIR_FLINCH_FREE: f32 = 30.0;
const KH1_KNOCKDOWN_FREE: f32 = 58.0;

static BASE: AtomicUsize = AtomicUsize::new(0);
static O_REACTION: AtomicUsize = AtomicUsize::new(0);
static O_HITBACK: AtomicUsize = AtomicUsize::new(0);
/// The hit whose push is KH1's, and that push's bits (only while KH2's routine runs it).
static PUSH_HIT: AtomicUsize = AtomicUsize::new(0);
static PUSH: AtomicUsize = AtomicUsize::new(0);
static LOGGED: AtomicBool = AtomicBool::new(false);

/// KH2's `0x1403CE980`, Checked (decomp).
fn fold(r: u8) -> u8 {
    match r {
        1 | 0xC => 1,
        3 | 6 | 8 | 0xA | 0xD | 0x10 | 0x12 => 3,
        4 | 7 | 9 | 0xB | 0xF | 0x11 => 4,
        x => x,
    }
}

unsafe fn f<T>(off: usize) -> T
where
    T: Copy,
{
    std::mem::transmute_copy(&(BASE.load(Ordering::Relaxed) + off))
}

unsafe extern "system" fn hitback(hit: usize) -> f32 {
    let o: unsafe extern "system" fn(usize) -> f32 = std::mem::transmute(O_HITBACK.load(Ordering::Acquire));
    if hit != 0 && PUSH_HIT.load(Ordering::Relaxed) == hit {
        return f32::from_bits(PUSH.load(Ordering::Relaxed) as u32);
    }
    o(hit)
}

unsafe fn mercy(obj: usize, free: f32) {
    let at = (obj + 0xD70) as *mut f32;
    *at = (*at).max(free + KH1_MERCY);
}

unsafe extern "system" fn reaction(obj: usize, hit: usize) {
    let o: unsafe extern "system" fn(usize, usize) = std::mem::transmute(O_REACTION.load(Ordering::Acquire));
    if hit == 0 || !is_base_sora(obj) || *((hit + 0x18) as *const u8) & 1 != 0 {
        return o(obj, hit);
    }
    let r = (hit + 0x26) as *mut u8;
    let group = fold(*r);
    let status = *((obj + 0x5C0) as *const usize);
    if !(1..=4).contains(&group) || status == 0 {
        return o(obj, hit);
    }
    let max = *((status + 4) as *const i32);
    let damage = *((hit + 0x28) as *const i32);
    let share = if max < 1 { 1.0 } else { damage as f32 / max as f32 };
    let in_air: unsafe extern "system" fn(usize) -> u8 = f(IN_AIR);
    let air = in_air(obj) != 0;
    let kept = *r;
    if !LOGGED.swap(true, Ordering::Relaxed) {
        log!("defence kh1: KH1's reaction rule on Sora (first hit: damage {damage} of {max}, KH2 reaction {kept})");
    }
    if ((share * 100.0) as i32) < KH1_FLINCH_SHARE {
        *r = 3;
        PUSH.store((KH1_PUSH_PER_MAX * share).min(KH1_PUSH_CAP).to_bits() as usize, Ordering::Relaxed);
        PUSH_HIT.store(hit, Ordering::Relaxed);
        o(obj, hit);
        PUSH_HIT.store(0, Ordering::Relaxed);
        *r = kept;
        mercy(obj, if air { KH1_AIR_FLINCH_FREE } else { KH1_FLINCH_FREE });
        return;
    }
    if air {
        *r = 4;
        o(obj, hit);
        *r = kept;
        mercy(obj, KH1_AIR_FLINCH_FREE);
        return;
    }
    // On the ground: KH2's flinch without a push, then KH1's knock-down on the same front /
    // behind test as KH2's flinch (`0x1403D43B0`: `dot(facing, push) < 0` is the front).
    *r = 2;
    o(obj, hit);
    *r = kept;
    let facing: unsafe extern "system" fn(usize, *mut [f32; 4]) -> *const [f32; 4] = f(FACING);
    let mut v = [0.0f32; 4];
    let p = facing(obj, &mut v);
    let push = *(hit as *const [f32; 3]);
    let d = if p.is_null() { -1.0 } else { (*p)[0] * push[0] + (*p)[1] * push[1] + (*p)[2] * push[2] };
    let id = kh1_assets::KNOCKDOWN_IDS[if d < 0.0 { 0 } else { 1 }];
    let ctrl = obj + 0x158;
    let has: unsafe extern "system" fn(usize, u16) -> u8 = f(MOTION_HAS);
    if has(ctrl, id) == 0 {
        mercy(obj, KH1_FLINCH_FREE);
        return;
    }
    let request: unsafe extern "system" fn(usize, u32, f32, f32) = f(MOTION_REQUEST);
    let start: unsafe extern "system" fn(usize, u32, f32, f32, u64) = f(MOTION_START);
    request(ctrl, id as u32, 0.0, 0.0);
    start(ctrl, 0, 0.0, 0.0, 0);
    mercy(obj, KH1_KNOCKDOWN_FREE);
}

pub unsafe fn install(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    let a = hook::install(base + HITBACK, &[0x48, 0x8B, 0xC4, 0x48, 0x81, 0xEC, 0x98, 0x00, 0x00, 0x00], hitback as *const () as usize, &O_HITBACK, "hit-back 0x1403CEAB0");
    let ok = a.is_some() && hook::install(base + REACTION, &[0x40, 0x55, 0x56, 0x41, 0x56], reaction as *const () as usize, &O_REACTION, "reaction 0x1403D2FE0").is_some();
    if ok {
        log!("defence kh1: KH1's reactions on base Sora (share of max HP, KH1's push, knock-down {:?}, 30 frames after)", kh1_assets::KNOCKDOWN_IDS);
    } else {
        log!("defence kh1: KH1's reactions not used");
    }
}
