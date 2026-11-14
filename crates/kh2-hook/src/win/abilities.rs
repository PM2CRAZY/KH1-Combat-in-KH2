//! KH1's action abilities in their KH2 pairs' slots (`crate::ability_rows`, `docs/abilities.md`):
//! after KH2's table loader (and the magic rows, `menu_rows::extend_tables`), a copy of KH2's
//! `item` table with the pairs carrying KH1's abilities is made once; KH2's pointer
//! (`0x142A25370`, read by `0x1403E39A0`, Checked (decomp)) is that copy at `abilities = kh1` and
//! KH2's own at `kh2`. A switch in the Config menu only sets the pointer. Both tables are kept for
//! the session (KH2 may hold a row of either). Nothing of ours runs per frame.

use super::log;
use std::sync::atomic::{AtomicUsize, Ordering};

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
    BASE.store(base, Ordering::Relaxed);
    KH2.store(*p, Ordering::Relaxed);
    KH1.store(Box::leak(copy.into_boxed_slice()).as_ptr() as usize, Ordering::Relaxed);
    log!("abilities: KH1's {} action abilities ready in their KH2 pairs' rows", kh1_assets::abilities::PAIRS.len());
    apply(crate::settings::kh1(crate::settings::ABILITIES));
}

/// The Config menu (or the loader): KH1's rows or KH2's. KH2's ability block keeps the old bits
/// until KH2 builds Sora's status again (a room change, the equip screen: `0x1403C1D90`).
pub unsafe fn apply(kh1: bool) {
    let (base, kh2, ours) = (BASE.load(Ordering::Relaxed), KH2.load(Ordering::Relaxed), KH1.load(Ordering::Relaxed));
    if base == 0 || kh2 == 0 || ours == 0 {
        return;
    }
    *((base + ITEM_TABLE) as *mut usize) = if kh1 { ours } else { kh2 };
    log!("abilities {}: {}", if kh1 { "kh1" } else { "kh2" }, if kh1 { "KH1's abilities in their pairs' slots" } else { "KH2's ability rows back" });
}
