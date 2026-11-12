//! KH1's ground and air combo through KH2's own combo picker (`combos = kh1`; Phase 2 step 2,
//! `docs/combos.md`).
//!
//! KH2 picks Sora's attacks from his `ptya` set (Checked (decomp)): the selector `0x1404028D0`
//! takes it from his `+0xD50` (set by the PARTY constructor `0x1403D5840` from `ptya` by his
//! moveset), `0x140402D20` marks the valid rows (`0x1404029F0` by target, `0x140402750` by ability,
//! ground / air and combo stage: count `+0xD74` against the length `+0x200` / `+0x201`) and the
//! walk `0x140402C50` takes the first valid row whose motion isn't playing, falling back to row
//! id 5 (ground) or 9 (air). KH1's pickers (`0x1402A6EE0` ground, `0x1402A6C70` air, Checked
//! (decomp)) work the same way: the first marked row in table order whose request isn't playing,
//! the finisher at the count reaching the length, a fallback swing.
//!
//! So Sora's own set keeps its rows, order, windows and ability rows (KH2's ability moves join
//! KH1's combo: the owner, 2026-11-12), and its plain rows (no ability) play KH1's motions, built
//! into his motion set as KH2 motions 210..216 (`kh1_assets`, t12):
//! - ground: id 4 (kind 2) → KH1 row 11 (`0xC9`, motion 99), id 5 (the fallback) → row 13 (`0xC8`,
//!   98: KH1's own fallback is 13), id 6 (kind 1) → row 14 (`0xCA`, 100); finishers ids 2 / 3 →
//!   row 8 (`0xCB`, 101);
//! - air: id 9 (the fallback) → row 18 (`0xCC`, 102: KH1's plain air swing), id 10 → row 17
//!   (`0xCD`, 103); finishers ids 7 / 8 → row 16 (`0xCE`, 104).
//! KH1's rows and requests: `btltbl.bin` `+0xA9F0`, Checked (files). Assumptions: KH1 request
//! `0xC8 + n` plays KH1 motion `98 + n`; KH1's target tests for rows 11 / 13 / 14 / 17 (`0x1402A7320`)
//! are near enough KH2's windows on ids 4 / 5 / 6 / 10 (to be replaced with KH1's own tests).
//! Not yet: the jump approach rows 0 / 1 (KH2's, kept), KH1's combo length, ability finishers,
//! hit data, windows and lunges (Phase 2 steps 3..5).
//!
//! The change is to KH2's loaded table, once, when its rows still hold KH2's motions, and only when
//! this launch's build made motions 210..216 (the patch is skipped without them).

use super::hook;
use super::log;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const LOADER: usize = 0x3F_4C30;
const PTYA: usize = 0x2AE_57B8;
const SORA_SET: usize = 1;
const ROW: usize = 0x44;

/// (row id, KH2's motion, KH1's motion).
const ROWS: [(u8, u16, usize); 9] = [
    (4, 152, 99),
    (5, 151, 98),
    (6, 153, 100),
    (2, 154, 101),
    (3, 155, 101),
    (9, 181, 102),
    (10, 182, 103),
    (7, 184, 104),
    (8, 183, 104),
];

static BASE: AtomicUsize = AtomicUsize::new(0);
static O_LOADER: AtomicUsize = AtomicUsize::new(0);
static READY: AtomicBool = AtomicBool::new(false);
static DONE: AtomicBool = AtomicBool::new(false);

/// Sora's `ptya` set (count, then rows), as KH2's `0x1403EB760` finds it.
unsafe fn sora_set() -> usize {
    let t = *((BASE.load(Ordering::Relaxed) + PTYA) as *const usize);
    if t == 0 || SORA_SET as i32 >= *((t + 4) as *const i32) {
        return 0;
    }
    let off = *((t + 8 + SORA_SET * 4) as *const i32);
    if off == 0 { 0 } else { t + off as usize }
}

unsafe fn try_patch() {
    if !READY.load(Ordering::Relaxed) || DONE.load(Ordering::Relaxed) {
        return;
    }
    let set = sora_set();
    if set == 0 {
        return;
    }
    let n = *(set as *const i32) as usize;
    let row_of = |id: u8| (0..n).map(|i| set + 4 + i * ROW).find(|&r| *(r as *const u8) == id && *((r + 0x40) as *const u16) == 0);
    let mut plan = Vec::new();
    for (id, kh2, kh1) in ROWS {
        let Some(r) = row_of(id) else {
            log!("combos kh1: Sora's ptya has no plain row {id}: left alone");
            return;
        };
        let have = *((r + 8) as *const u16);
        if have != kh2 {
            log!("combos kh1: ptya row {id} plays motion {have}, not KH2's {kh2}: left alone");
            return;
        }
        plan.push((r, id, kh1_assets::kh2_motion_of(kh1).unwrap() as u16));
    }
    for (r, _, m) in &plan {
        *((r + 8) as *mut u16) = *m;
    }
    DONE.store(true, Ordering::Relaxed);
    log!("combos kh1: Sora's plain combo rows play KH1's swings: {:?}", plan.iter().map(|(_, id, m)| (*id, *m)).collect::<Vec<_>>());
}

unsafe extern "system" fn loader() {
    let o: unsafe extern "system" fn() = std::mem::transmute(O_LOADER.load(Ordering::Acquire));
    o();
    // (A reload brings KH2's rows back: patch them again.)
    DONE.store(false, Ordering::Relaxed);
    try_patch();
}

pub unsafe fn install(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    if hook::install(base + LOADER, &[0x48, 0x89, 0x5C, 0x24, 0x08], loader as *const () as usize, &O_LOADER, "table loader 0x1403F4C30").is_none() {
        log!("combos kh1: KH1's combo not used");
    }
}

/// The build made (or already had) KH1's motions: the rows may point at them now.
pub fn motions_ready() {
    READY.store(true, Ordering::Relaxed);
    unsafe { try_patch() };
}
