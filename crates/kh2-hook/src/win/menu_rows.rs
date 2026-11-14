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
    log!("menu rows: KH2's {SUBMENUS} command submenus moved to {mem:#x} with room for {ROWS} rows each, the HUD draws {HUD_LEVEL} sprites a level, {ELEMENTS} magic elements in KH1's order ({done} of {} sites)", plan.patches.len());
}
