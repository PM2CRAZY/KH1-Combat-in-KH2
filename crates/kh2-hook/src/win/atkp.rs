//! KH1's hit data on KH2's swing ATKP rows (`combos = kh1`; Phase 2, the owner's choice: KH2's
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
}

pub unsafe fn install(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    hook::install(base + LOADER, &[0x48, 0x89, 0x5C, 0x24, 0x08], loader as *const () as usize, &O_LOADER, "table loader 0x1403F4C30");
}
