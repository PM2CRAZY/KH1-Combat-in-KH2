//! KH1's action abilities in their KH2 pairs' slots (`crate::ability_rows`, `docs/abilities.md`):
//! after KH2's table loader (and the magic rows, `menu_rows::extend_tables`), a copy of KH2's
//! `item` table with the pairs carrying KH1's abilities is made once; KH2's pointer
//! (`0x142A25370`, read by `0x1403E39A0`, Checked (decomp)) is that copy at `abilities = kh1` and
//! KH2's own at `kh2`. A switch in the Config menu only sets the pointer. Both tables are kept for
//! the session (KH2 may hold a row of either). Nothing of ours runs per frame.
//!
//! KH2's own combo modifiers and finishers (`kh1_assets::abilities::HIDDEN`) at `abilities = kh1`:
//! two wrappers hide them from KH2's own reads of Sora's list, putting the entries back right
//! after (the save keeps them; KH2's scripts reading the list by id still see them):
//! - the ability block build `0x1404014A0` (status build, room change, equip screen): their
//!   equipped bit is cleared for the call, so neither their number bits (their `ptya` rows) nor
//!   their item-id effects in `0x140401730` (Finishing Plus, Negative Combo, Berserk Charge, Combo
//!   Boost, Air Combo Boost) apply;
//! - the ability screen's list build `0x140360920`: their ids are 0 for the call, so KH2 skips
//!   them. Their AP is 0 in the KH1 table (KH2's AP totals sum every equipped entry).
//! Checked (decomp): both walk Sora's record `+0x54` (80 entries); the screen keeps the entry's
//! index for equipping, which the masking doesn't move.

use super::{hook, log};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Sora's save record (`save 0x1409A98B0 + 0x24F0`, char 1 → slot 1: `0x1403A14E0`), his ability
/// list at `+0x54`, 80 entries. Checked (decomp).
const SORA_RECORD: usize = 0x9A_BDA0;
const BLOCK_BUILD: usize = 0x40_14A0;
const SCREEN_LIST: usize = 0x36_0920;
static O_BLOCK: AtomicUsize = AtomicUsize::new(0);
static O_SCREEN: AtomicUsize = AtomicUsize::new(0);

pub unsafe fn install(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    // 40 53 push rbx; 48 83 EC 20 sub rsp, 0x20.
    hook::install(base + BLOCK_BUILD, &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20], block_build as *const () as usize, &O_BLOCK, "ability block 0x1404014A0");
    // 40 57 push rdi; 48 83 EC 40 sub rsp, 0x40.
    hook::install(base + SCREEN_LIST, &[0x40, 0x57, 0x48, 0x83, 0xEC, 0x40], screen_list as *const () as usize, &O_SCREEN, "ability screen list 0x140360920");
}

/// Runs `f` with Sora's hidden entries changed by `mask` (each put back after).
unsafe fn masked(mask: fn(u16) -> u16, f: impl FnOnce()) {
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 || !crate::settings::kh1(crate::settings::ABILITIES) {
        return f();
    }
    let list = (base + SORA_RECORD + 0x54) as *mut u16;
    let mut kept = [(0usize, 0u16); 80];
    let mut n = 0;
    for i in 0..80 {
        let e = *list.add(i);
        if e & 0x7FFF != 0 && kh1_assets::abilities::hidden(e & 0x7FFF) {
            kept[n] = (i, e);
            n += 1;
            *list.add(i) = mask(e);
        }
    }
    f();
    for &(i, e) in &kept[..n] {
        *list.add(i) = e;
    }
}

unsafe extern "C" fn block_build(block: usize) {
    let o: unsafe extern "C" fn(usize) = std::mem::transmute(O_BLOCK.load(Ordering::Acquire));
    masked(|e| e & 0x7FFF, || o(block));
}

unsafe extern "C" fn screen_list() {
    let o: unsafe extern "C" fn() = std::mem::transmute(O_SCREEN.load(Ordering::Acquire));
    masked(|_| 0, || o());
}

const ITEM_TABLE: usize = 0x2A2_5370;
static BASE: AtomicUsize = AtomicUsize::new(0);
static KH2: AtomicUsize = AtomicUsize::new(0);
static KH1: AtomicUsize = AtomicUsize::new(0);

/// After KH2's table loader: the KH1 copy of the table KH2 now points at.
pub unsafe fn extend(base: usize) {
    let p = (base + ITEM_TABLE) as *mut usize;
    if *p == 0 || *p == KH1.load(Ordering::Relaxed) {
        return;
    }
    let n = (*((*p + 4) as *const i32)).clamp(0, 0x4000) as usize;
    let own = std::slice::from_raw_parts(*p as *const u8, 8 + n * crate::ability_rows::ITEM_ROW);
    let Some(copy) = crate::ability_rows::kh1_items(own) else {
        log!("abilities: KH2's item table isn't one we know; KH1's abilities not in it");
        return;
    };
    KH2.store(*p, Ordering::Relaxed);
    KH1.store(Box::leak(copy.into_boxed_slice()).as_ptr() as usize, Ordering::Relaxed);
    log!("abilities: KH1's {} action abilities ready in their KH2 pairs' rows", kh1_assets::abilities::PAIRS.len());
    apply(crate::settings::kh1(crate::settings::ABILITIES));
}

/// The Config menu (or the loader): KH1's rows or KH2's. KH2's ability block keeps the old bits
/// until KH2 builds Sora's status again (a room change, the equip screen: `0x1403C1D90`).
pub unsafe fn apply(kh1: bool) {
    let (base, kh2, ours) = (BASE.load(Ordering::Relaxed), KH2.load(Ordering::Relaxed), KH1.load(Ordering::Relaxed));
    // (BASE is set by `install`, at launch.)
    if base == 0 || kh2 == 0 || ours == 0 {
        return;
    }
    *((base + ITEM_TABLE) as *mut usize) = if kh1 { ours } else { kh2 };
    log!("abilities {}: {}", if kh1 { "kh1" } else { "kh2" }, if kh1 { "KH1's abilities in their pairs' slots" } else { "KH2's ability rows back" });
}
