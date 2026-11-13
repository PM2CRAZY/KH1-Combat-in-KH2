//! KH1's combo data in KH2's own combo tables, patched once after KH2 loads them (`combos =
//! kh1`, Phase 2). Two tables: the swing ATKP rows, and Sora's air rows in `ptya`.
//!
//! ## ATKP: KH1's hit data on KH2's swing rows (the owner's choice: KH2's
//! damage formula with each KH1 swing given the nearest KH2 power, KH1's reactions).
//!
//! KH2 loads `00battle.bin`'s `atkp` in its table loader `0x1403F4C30` and keeps the table at
//! `0x142AE57B0` (`0x1403EB010`; `+4` row count, rows of 0x30 from `+8`, `+2` id, `+6` power u16;
//! Checked (decomp)). After it runs, the rows of the Keyblade's swing hit windows (the ATKP each
//! swing slot's Keyblade motion names, `W_EX010.mset`, Checked (files)) get KH1's power:
//! - KH1's plain swings (`btltbl.bin` rows 13 / 11 / 14 / 18 / 17, data `0x4B78` / `0x4BE8`) all
//!   have power 0 and flinch (3): every hit the same. KH2's first swing's 100 for all: 126, 127,
//!   128 (KH2 65, 65), 131, 132.
//! - KH1's finishers (rows 8 / 16, data `0x4C58`) power 4 and launch (5): 4 points up KH1's
//!   damage curve (about ×1.19 a point) is ×2.0 at the curve's middle. So 200: 129, 130 (KH2 150),
//!   133, 134 (KH2 150). Checked (files): the curve in `btltbl.bin`, the rows above.
//! KH2's reactions already match (3 flinch on the swings, 7 on the finishers for KH1's launch),
//! and its knockback stays (KH1's data has none: its launch comes from the reaction).
//! Assumption: no other object's ATKP uses these ids (Sora's swing ids, KH2's own data).
//! KH1's critical hit (its finishers' 4 % for +4) has no place in KH2's damage: not ported.

use super::hook;
use super::log;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const LOADER: usize = 0x3F_4C30;
const ATKP: usize = 0x2AE_57B0;
const ROW: usize = 0x30;
const PTYA: usize = 0x2AE_57B8;
const SORA_SET: usize = 1;
const PTYA_ROW: usize = 0x44;

/// Sora's air swing rows in `ptya` (row id, KH1's hop height, KH1's speed). KH1's row runner
/// `0x1402A68F0` (Checked (decomp)) gives an air swing a hop of its row's height (`+8`: 20 on the
/// plain air swings, rows 17 / 18; 40 on the air finisher, row 16; `btltbl.bin`, Checked (files))
/// with KH1's gravity (`0x1402AF960`), and a constant speed toward its aim: 8 on the combo's first
/// hit, 12 on the air finishers (rows 15 / 16), else 4. KH2's air rows (Checked (files), Sora's
/// set): ids 9 / 10 the plain air swings (KH2: height 17..67 / 21..71 by the target's height,
/// speed −8..8 by its distance), 7 / 8 the air finishers (21..71, 12..18). So: height and speed
/// fixed at KH1's (`jump` = `jump_min` = `jump_max`, `speed_min` = `speed_max`). Assumption: KH2's
/// row heights are arc heights in its units (the OG rewrite's reading); with `movement = kh1` the
/// arc is KH1's own. Not covered: KH1's 8 on the first air hit (a row can't tell the first hit;
/// the plain rows take KH1's 4).
const AIR_ROWS: [(u8, f32, f32); 4] = [(9, 20.0, 4.0), (10, 20.0, 4.0), (7, 40.0, 12.0), (8, 40.0, 12.0)];

unsafe fn patch_ptya() {
    let t = *((BASE.load(Ordering::Relaxed) + PTYA) as *const usize);
    if t == 0 || SORA_SET as i32 >= *((t + 4) as *const i32) {
        return;
    }
    let off = *((t + 8 + SORA_SET * 4) as *const i32);
    if off == 0 {
        return;
    }
    let set = t + off as usize;
    let n = (*(set as *const i32)).clamp(0, 256) as usize;
    let mut done = Vec::new();
    for i in 0..n {
        let r = set + 4 + i * PTYA_ROW;
        let (id, ability) = (*(r as *const u8), *((r + 0x40) as *const u16));
        let Some(&(_, h, v)) = AIR_ROWS.iter().find(|x| x.0 == id).filter(|_| ability == 0) else { continue };
        let f = |o: usize| (r + o) as *mut f32;
        done.push((id, *f(0x0C), *f(0x18), *f(0x1C)));
        *f(0x0C) = h;
        *f(0x10) = h;
        *f(0x14) = h;
        *f(0x18) = v;
        *f(0x1C) = v;
    }
    log!("combos kh1: KH1's air swing hop and speed on ptya rows (id, KH2's height, speed min, max): {done:?}");
}

/// (ATKP id, KH1 power as a KH2 power).
pub const POWERS: [(u16, u16); 9] = [(126, 100), (127, 100), (128, 100), (131, 100), (132, 100), (129, 200), (130, 200), (133, 200), (134, 200)];

static BASE: AtomicUsize = AtomicUsize::new(0);
static O_LOADER: AtomicUsize = AtomicUsize::new(0);

unsafe fn patch() {
    let t = *((BASE.load(Ordering::Relaxed) + ATKP) as *const usize);
    if t == 0 {
        return;
    }
    let n = (*((t + 4) as *const i32)).clamp(0, 0x10000) as usize;
    let mut changed = Vec::new();
    for i in 0..n {
        let r = t + 8 + i * ROW;
        let id = *((r + 2) as *const u16);
        if let Some(&(_, p)) = POWERS.iter().find(|x| x.0 == id) {
            let at = (r + 6) as *mut u16;
            if *at != p {
                changed.push((id, *at, p));
                *at = p;
            }
        }
    }
    log!("combos kh1: KH1's swing powers on ATKP (id, KH2's, KH1's): {changed:?}");
}

static COMBOS: AtomicBool = AtomicBool::new(false);
static DEFENCE: AtomicBool = AtomicBool::new(false);

/// KH1's guard arc on KH2's guard (`defence = kh1`). KH1's guard (attack data `0x4DA8`, `+0x64`)
/// meets attacks within an arc cosine of -0.174, 100 degrees either side of his facing (the
/// sister project's `kh1-roll-guard-native.md` §3.3, Checked (files) there). KH2's guard window
/// (ATKP 161, the one t41 times to KH1's guard) meets attacks by its reflect range `+0x22` in
/// degrees either side of the owner's facing: `0x1403D1F40` clashes when `dot(the other's
/// push, facing) < -cos(range)` (Checked (decomp)); 161's is 120 (Checked (files)). So 161's
/// range becomes 100. Assumption: nothing but Sora's guard uses ATKP 161.
const GUARD_ATKP: u16 = 161;
const KH1_GUARD_ARC: u8 = 100;

unsafe fn patch_guard_arc() {
    let t = *((BASE.load(Ordering::Relaxed) + ATKP) as *const usize);
    if t == 0 {
        return;
    }
    let n = (*((t + 4) as *const i32)).clamp(0, 0x10000) as usize;
    let mut changed = Vec::new();
    for i in 0..n {
        let r = t + 8 + i * ROW;
        if *((r + 2) as *const u16) == GUARD_ATKP {
            let at = (r + 0x22) as *mut u8;
            changed.push(*at);
            *at = KH1_GUARD_ARC;
        }
    }
    log!("defence kh1: KH1's guard arc {KH1_GUARD_ARC} degrees on ATKP {GUARD_ATKP} (KH2's {changed:?})");
}
static MODEL: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn loader() {
    let o: unsafe extern "system" fn() = std::mem::transmute(O_LOADER.load(Ordering::Acquire));
    o();
    if COMBOS.load(Ordering::Relaxed) {
        patch();
        patch_ptya();
    }
    if MODEL.load(Ordering::Relaxed) {
        super::weapon_hold::patch(BASE.load(Ordering::Relaxed));
    }
    if DEFENCE.load(Ordering::Relaxed) {
        patch_guard_arc();
    }
}

/// Hooks KH2's table loader for what patches its tables: `combos = kh1` (above), `model = kh1`
/// (`weapon_hold`) and `defence = kh1` (the guard arc).
pub unsafe fn install(base: usize, combos: bool, model: bool, defence: bool) {
    BASE.store(base, Ordering::Relaxed);
    COMBOS.store(combos, Ordering::Relaxed);
    DEFENCE.store(defence, Ordering::Relaxed);
    MODEL.store(model, Ordering::Relaxed);
    hook::install(base + LOADER, &[0x48, 0x89, 0x5C, 0x24, 0x08], loader as *const () as usize, &O_LOADER, "table loader 0x1403F4C30");
}
