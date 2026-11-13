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
//! - 20 % and over, in the air: KH2's launch (group 4) on KH1's air knock-back motions (t58), and
//!   (t59, the owner: "KH1's exact path") KH1's path: KH2's blow step `0x140422780 (action, out,
//!   object, dt)` (per step: sideways × sstm `+0x18` 0.8 floored at `+0x10C`, rising × `+0x20`
//!   0.8, then + 20/49 capped at `+0x138`, out = velocity × dt; velocity `+0x20..+0x28`, y down;
//!   Checked (decomp)) gives this launch KH1's arc instead (`kh1::arc_height`, a hop of 20 under
//!   KH1's gravity, the fall capped at 20 a frame) and KH1's sideways 16 × 0.95 a 30 Hz frame.
//!   Only for the launch this hook started, on base Sora; any other blow is KH2's. Assumption:
//!   `dt` counts 60 Hz frames (logged the first time);
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
const BLOW_STEP: usize = 0x42_2780;

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
static O_BLOW: AtomicUsize = AtomicUsize::new(0);
/// The KH1 launch in flight: Sora's object (0: none), its time (60 Hz frames) and sideways
/// velocity (x, z).
static KNOCK: std::sync::Mutex<(usize, f32, [f32; 2])> = std::sync::Mutex::new((0, 0.0, [0.0; 2]));
static BLOW_LOGGED: AtomicBool = AtomicBool::new(false);

type BlowFn = unsafe extern "system" fn(usize, *mut [f32; 4], usize, f32) -> *mut [f32; 4];

unsafe extern "system" fn blow_step(action: usize, out: *mut [f32; 4], obj: usize, dt: f32) -> *mut [f32; 4] {
    let o: BlowFn = std::mem::transmute(O_BLOW.load(Ordering::Acquire));
    // Ours only while his sideways velocity is still the one we last gave him: a blow KH2 starts
    // itself (an event's) sets its own, and ends ours.
    let ours = KNOCK.lock().map(|k| k.0 == obj && obj != 0 && *((obj + 0x20) as *const f32) == k.2[0] && *((obj + 0x28) as *const f32) == k.2[1]).unwrap_or(false);
    let r = o(action, out, obj, dt);
    let Ok(mut k) = KNOCK.lock() else { return r };
    if !ours || out.is_null() {
        if k.0 == obj {
            k.0 = 0;
        }
        return r;
    }
    if !BLOW_LOGGED.swap(true, Ordering::Relaxed) {
        log!("defence kh1: KH1's air knock-back path on KH2's blow (step dt {dt})");
    }
    let (t0, t1) = (k.1, k.1 + dt);
    k.1 = t1;
    // y is down in KH2: the drop this step, KH1's fall capped at 20 a frame.
    let dy = (crate::kh1::arc_height(t0, crate::kh1::KH1_KNOCKBACK_HOP) - crate::kh1::arc_height(t1, crate::kh1::KH1_KNOCKBACK_HOP)).min(crate::kh1::KH1_FALL_CAP * dt);
    let drag = crate::kh1::KH1_AIR_DRAG.powf(dt / 2.0);
    k.2 = [k.2[0] * drag, k.2[1] * drag];
    *((obj + 0x20) as *mut f32) = k.2[0];
    *((obj + 0x24) as *mut f32) = if dt > 0.0 { dy / dt } else { 0.0 };
    *((obj + 0x28) as *mut f32) = k.2[1];
    (*out)[0] = k.2[0] * dt;
    (*out)[1] = dy;
    (*out)[2] = k.2[1] * dt;
    r
}

/// Starts KH1's path for the launch KH2's routine just gave `obj` (its sideways direction kept).
unsafe fn start_knock(obj: usize) {
    let (vx, vz) = (*((obj + 0x20) as *const f32), *((obj + 0x28) as *const f32));
    let len = (vx * vx + vz * vz).sqrt();
    let dir = if len > 1e-4 { [vx / len, vz / len] } else { [0.0, 0.0] };
    let v = [dir[0] * crate::kh1::KH1_KNOCKBACK_SPEED, dir[1] * crate::kh1::KH1_KNOCKBACK_SPEED];
    *((obj + 0x20) as *mut f32) = v[0];
    *((obj + 0x28) as *mut f32) = v[1];
    if let Ok(mut k) = KNOCK.lock() {
        *k = (obj, 0.0, v);
    }
}

fn stop_knock() {
    if let Ok(mut k) = KNOCK.lock() {
        k.0 = 0;
    }
}

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
    stop_knock();
    if air {
        *r = 4;
        o(obj, hit);
        *r = kept;
        start_knock(obj);
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
    let b = a.is_some() && hook::install(base + BLOW_STEP, &[0x48, 0x89, 0x5C, 0x24, 0x08], blow_step as *const () as usize, &O_BLOW, "blow step 0x140422780").is_some();
    let ok = b && hook::install(base + REACTION, &[0x40, 0x55, 0x56, 0x41, 0x56], reaction as *const () as usize, &O_REACTION, "reaction 0x1403D2FE0").is_some();
    if ok {
        log!("defence kh1: KH1's reactions on base Sora (share of max HP, KH1's push, knock-down {:?}, 30 frames after)", kh1_assets::KNOCKDOWN_IDS);
    } else {
        log!("defence kh1: KH1's reactions not used");
    }
}
