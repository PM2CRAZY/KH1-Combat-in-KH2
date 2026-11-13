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
use std::sync::atomic::{AtomicUsize, Ordering};

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

unsafe extern "system" fn loader() {
    let o: unsafe extern "system" fn() = std::mem::transmute(O_LOADER.load(Ordering::Acquire));
    o();
    patch();
    patch_ptya();
}

pub unsafe fn install(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    hook::install(base + LOADER, &[0x48, 0x89, 0x5C, 0x24, 0x08], loader as *const () as usize, &O_LOADER, "table loader 0x1403F4C30");
}
