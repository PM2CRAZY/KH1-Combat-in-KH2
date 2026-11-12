//! KH1's ground numbers in Sora's own `prty` row (`movement = kh1`): KH2 loads `03system.bin`'s
//! `pref` and its setter `0x1403E9360` keeps `prty` at `0x142AE5780` (Checked (decomp),
//! `docs/kh2-tables.md`); right after it, Sora's row (row 1: `0x1403E9590`'s fallback, and his
//! walk 2 / run 8 / jump 185 / turn 0.262 in the t3 trace, Checked (trace)) gets KH1's jump
//! height and turn rate. KH2's own code then copies them into Sora (`0x14040A480`) as it always
//! does. Walk and run are the same in both games (2 and 8), so they stay.
//!
//! The row is changed only if it holds KH2's values (2, 8, 185, 0.262): a different `03system.bin`
//! is left alone and logged.

use super::hook;
use super::log;
use crate::kh1::{KH1_JUMP, KH1_TURN};
use std::sync::atomic::{AtomicUsize, Ordering};

const PREF_SET: usize = 0x3E_9360;
const PRTY: usize = 0x2AE_5780;
const SORA_ROW: usize = 1;

static BASE: AtomicUsize = AtomicUsize::new(0);
static O_PREF_SET: AtomicUsize = AtomicUsize::new(0);

unsafe fn patch_row() {
    if *((BASE.load(Ordering::Relaxed) + PRTY) as *const usize) == 0 {
        return;
    }
    let at = super::sora::prty_row(SORA_ROW);
    if at == 0 {
        log!("movement kh1: prty has no row {SORA_ROW}");
        return;
    }
    let row = at as *mut f32;
    let have = [*row, *row.add(1), *row.add(2), *row.add(3)];
    let kh2 = [2.0f32, 8.0, 185.0, 0.262];
    if have.iter().zip(kh2).any(|(a, b)| (a - b).abs() > 0.001) {
        log!("movement kh1: Sora's prty row holds {have:?}, not KH2's {kh2:?}: left alone");
        return;
    }
    *row.add(2) = KH1_JUMP;
    *row.add(3) = KH1_TURN;
    log!("movement kh1: Sora's prty row: jump {KH1_JUMP}, turn {KH1_TURN} (KH2's 185, 0.262)");
}

type SetFn = unsafe extern "system" fn(usize);
unsafe extern "system" fn pref_set(file: usize) {
    let o: SetFn = std::mem::transmute(O_PREF_SET.load(Ordering::Acquire));
    o(file);
    patch_row();
}

pub unsafe fn install(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    // (Installed at the DLL's load, before KH2 loads its system files; if `pref` is already in,
    // its row is changed now too.)
    let ours = pref_set as *const () as usize;
    if hook::install(base + PREF_SET, &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20], ours, &O_PREF_SET, "pref setter 0x1403E9360").is_none() {
        log!("movement kh1: KH1's jump height and turn rate not used");
        return;
    }
    patch_row();
}
