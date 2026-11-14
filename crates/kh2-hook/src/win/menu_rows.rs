//! KH2's command submenus with room for 12 rows (`crate::menu_rows` computes the bytes; owner's
//! choice (a1)). Always on: with 8 rows or fewer KH2's menus behave exactly as before.

use super::{hook, log};
use crate::menu_rows::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

static BASE: AtomicUsize = AtomicUsize::new(0);
static MEMORY: AtomicUsize = AtomicUsize::new(0);
/// KH2's static init had already run when we loaded (we set up the magic elements ourselves).
static LATE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static PLAN: OnceLock<Plan> = OnceLock::new();
/// The move happened (KH2 has 9 magic elements): our `cmd` / `item` rows may go in.
static APPLIED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
const CMD_TABLE: usize = 0x2A1_61E8;
const ITEM_TABLE: usize = 0x2A2_5370;
const ATKP_TABLE: usize = 0x2AE_57B0;
const MAGC_TABLE: usize = 0x2A2_3C68;
const STOP_TABLE: usize = 0x2AE_59D8;

/// After KH2's table loader (`combo_tables`): its `cmd` and `item` tables copied with Gravity's,
/// Stop's and Aero's rows (`crate::magic_rows`), KH2's pointers set to the copies. The copies are
/// kept for the session (KH2 may hold a row of an older one).
pub unsafe fn extend_tables(base: usize) {
    if !APPLIED.load(Ordering::Relaxed) {
        return;
    }
    let mut done = Vec::new();
    for (at, row, f, what) in [
        (CMD_TABLE, crate::magic_rows::CMD_ROW, crate::magic_rows::extend_commands as fn(&[u8]) -> Option<Vec<u8>>, "cmd"),
        (ITEM_TABLE, crate::magic_rows::ITEM_ROW, crate::magic_rows::extend_items, "item"),
    ] {
        let p = (base + at) as *mut usize;
        if *p == 0 {
            continue;
        }
        let n = (*((*p + 4) as *const i32)).clamp(0, 0x4000) as usize;
        let old = std::slice::from_raw_parts(*p as *const u8, 8 + n * row);
        if let Some(new) = f(old) {
            *p = Box::leak(new.into_boxed_slice()).as_ptr() as usize;
            done.push(what);
        }
    }
    // Gravity's MAGC rows only when its files are in the mod folder (KH2 loads the file of the
    // level Sora has; a missing one would fail).
    let dir = super::mod_path();
    let built = |file: &str| (1..=3).all(|l| std::path::Path::new(&dir).join(format!("magic/{file}_{l}.mag")).is_file());
    let mut more: Vec<(usize, usize, Box<dyn Fn(&[u8]) -> Option<Vec<u8>>>, &str)> =
        vec![(ATKP_TABLE, crate::magic_rows::ATKP_ROW, Box::new(crate::magic_rows::extend_atkp), "atkp"), (STOP_TABLE, 4, Box::new(crate::magic_rows::extend_stop), "stop")];
    for (k, (element, file)) in [(GRAVITY, "GRAVITY"), (STOP, "STOP"), (AERO, "AERO")].into_iter().enumerate() {
        if built(file) {
            more.push((MAGC_TABLE, crate::magic_rows::MAGC_ROW, Box::new(move |t: &[u8]| crate::magic_rows::extend_magc(t, element, file, k)), file));
        }
    }
    for (at, row, f, what) in more {
        let p = (base + at) as *mut usize;
        if *p == 0 {
            continue;
        }
        let n = (*((*p + 4) as *const i32)).clamp(0, 0x4000) as usize;
        let old = std::slice::from_raw_parts(*p as *const u8, 8 + n * row);
        if let Some(new) = f(old) {
            *p = Box::leak(new.into_boxed_slice()).as_ptr() as usize;
            done.push(what);
        }
    }
    log!("magic: Gravity, Stop and Aero rows in KH2's tables: {done:?}");
}

pub unsafe fn install(base: usize) {
    let Some(mem) = hook::alloc_near(base + CTOR_RVA as usize, MEM) else {
        log!("menu rows: no memory near KH2's code; submenus keep 8 rows");
        return;
    };
    let plan = match plan(base as u64, mem as u64) {
        Ok(p) => p,
        Err(e) => {
            log!("menu rows: {e}; submenus keep 8 rows");
            return;
        }
    };
    for p in &plan.patches {
        let have = std::slice::from_raw_parts((base + p.rva as usize) as *const u8, p.expect.len());
        if have != p.expect {
            log!("menu rows: {:#x} is {have:02x?}, not KH2's {:02x?}; nothing changed, submenus keep 8 rows", base + p.rva as usize, p.expect);
            return;
        }
    }
    if *((base + HUD_POINTER_RVA as usize) as *const usize) != 0 {
        log!("menu rows: KH2's command HUD already exists (made at KH2's size); nothing changed, submenus keep 8 rows");
        return;
    }
    BASE.store(base, Ordering::Relaxed);
    MEMORY.store(mem, Ordering::Relaxed);
    let _ = PLAN.set(plan);
    let pc = base + PC_RVA as usize;
    if *(pc as *const usize) == base + BUILT_VTABLE_RVA as usize {
        LATE.store(true, Ordering::Relaxed);
        apply();
    } else if hook::redirect_call(base + CTOR_CALL_RVA as usize, base + CTOR_RVA as usize, ctor as *const () as usize, "menu rows: command menu ctor") {
        log!("menu rows: the command menu isn't built yet; moving its submenus once KH2 builds it");
    } else {
        log!("menu rows: could not wait for the command menu's ctor; submenus keep 8 rows");
    }
}

/// KH2's static init calls this in place of the object's ctor: the ctor, then the move.
unsafe extern "C" fn ctor(this: *mut u8) -> *mut u8 {
    let kh2: extern "C" fn(*mut u8) -> *mut u8 = std::mem::transmute(BASE.load(Ordering::Relaxed) + CTOR_RVA as usize);
    let r = kh2(this);
    apply();
    r
}

unsafe fn apply() {
    let (base, mem) = (BASE.load(Ordering::Relaxed), MEMORY.load(Ordering::Relaxed));
    let Some(plan) = PLAN.get() else { return };
    let pc = base + PC_RVA as usize;
    std::ptr::copy_nonoverlapping(plan.stubs.as_ptr(), (mem + CAVES) as *mut u8, plan.stubs.len());
    magic_data(std::slice::from_raw_parts_mut(mem as *mut u8, MEM));
    if LATE.load(Ordering::Relaxed) {
        // KH2's static init `0x140025690` ran on its own 6: build our 9 the way it does.
        let set: extern "C" fn(usize) = std::mem::transmute(base + ELEMENT_SET_CTOR_RVA as usize);
        for i in 0..ELEMENTS {
            set(mem + MAGIC_ELEMS + i * ELEMENT + 0x18);
        }
    }
    let object = std::slice::from_raw_parts_mut(pc as *mut u8, 0xC00);
    let block = std::slice::from_raw_parts_mut(mem as *mut u8, BLOCK);
    migrate(object, pc as u64, block, mem as u64);
    let mut done = 0;
    for p in &plan.patches {
        if hook::patch(base + p.rva as usize, p.expect, &p.new, "menu rows") {
            done += 1;
        }
    }
    APPLIED.store(done == plan.patches.len(), Ordering::Relaxed);
    log!("menu rows: KH2's {SUBMENUS} command submenus moved to {mem:#x} with room for {ROWS} rows each, the HUD draws {HUD_LEVEL} sprites a level, {ELEMENTS} magic elements in KH1's order ({done} of {} sites)", plan.patches.len());
}

/// Debug `new_spells` (0 off): the level the new spells are given at.
static DEBUG_SPELLS: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
/// KH2's item counts (`0x1409ACE30`, by an item's slot; `0x1403C4720`, Checked (decomp)).
const ITEM_COUNTS: usize = 0x9A_CE30;

pub fn set_debug_spells(level: u8) {
    DEBUG_SPELLS.store(level, Ordering::Relaxed);
}

/// Each frame (debug only): Gravity's item count up to the debug level, once our rows are in,
/// with `magic = kh1` (a save loaded later still gets it).
pub fn debug_spells() {
    if !APPLIED.load(Ordering::Relaxed) || !crate::settings::kh1(crate::settings::MAGIC) {
        return;
    }
    earned_spells();
    let level = DEBUG_SPELLS.load(Ordering::Relaxed);
    if level == 0 {
        return;
    }
    for (slot, name) in crate::magic_rows::SLOTS.iter().zip(["Gravity", "Stop", "Aero"]) {
        let at = (BASE.load(Ordering::Relaxed) + ITEM_COUNTS + *slot as usize) as *mut u8;
        unsafe {
            if *at < level {
                *at = level;
                log!("debug: {name} given at level {level} (new_spells)");
            }
        }
    }
}

/// KH2's earned victory bonuses (`0x1409ACFB4`, a bit a reward id, set by `0x1403EDCA0`).
const BONUS_BITS: usize = 0x9A_CFB4;

/// A save that passed a spell's story points before KH1's spells were on (or before this
/// version) still has the levels it earned: each new spell's count is at least the number of
/// its bonuses already earned. Each frame: nine bit tests (the count only ever goes up).
fn earned_spells() {
    let base = BASE.load(Ordering::Relaxed);
    let dir = super::mod_path();
    for (k, element) in [GRAVITY, STOP, AERO].into_iter().enumerate() {
        let earned = crate::magic_rows::UNLOCKS
            .iter()
            .filter(|u| u.1 == element)
            .filter(|u| unsafe { *((base + BONUS_BITS + 4 * (u.0 as usize >> 5)) as *const u32) } >> (u.0 & 31) & 1 != 0)
            .count() as u8;
        if earned == 0 {
            continue;
        }
        let at = (base + ITEM_COUNTS + crate::magic_rows::SLOTS[k] as usize) as *mut u8;
        unsafe {
            if *at < earned {
                let file = ["GRAVITY", "STOP", "AERO"][k];
                if !std::path::Path::new(&dir).join(format!("magic/{file}_1.mag")).is_file() {
                    continue;
                }
                *at = earned;
                log!("magic kh1: {file} at level {earned} from bonuses already earned");
            }
        }
    }
}
