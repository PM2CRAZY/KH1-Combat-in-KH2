//! Diagnostic for the Keyblade-switch crash (owner, t27–t30). `0x140407DE0 (attack, object)`
//! walks the object's collision entries (`object +0xAB8`) of types 5 and 14 while an attack is
//! on, and looks up each one's item (entry +2 for type 5, `0x83` through `object +0x9F0 +0x2C`,
//! else `object +0xCF0`) with `0x1403B68B0`, which reads the item table's row without checking
//! it was found: an id not in the table crashes at `0x1403B68E0`. Checked (decomp, exe bytes).
//! This wrapper logs, once per distinct case, every such id that isn't in the table, with the
//! entry, then lets the original run unchanged. Temporary: it runs on each call (attack frames).

use super::hook;
use super::log;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

const WALK: usize = 0x40_7DE0;
/// The item table pointer (`DAT_142a161e8`): +4 count, +8 rows of 0x30, each starting u16 id.
const ITEMS: usize = 0x2A1_61E8;

static O: AtomicUsize = AtomicUsize::new(0);
static BASE: AtomicUsize = AtomicUsize::new(0);
static SEEN: Mutex<Vec<(usize, u32)>> = Mutex::new(Vec::new());

unsafe fn in_table(id: u32) -> bool {
    let t = *((BASE.load(Ordering::Relaxed) + ITEMS) as *const usize);
    if t == 0 {
        return true;
    }
    let n = *((t + 4) as *const i32).max(&0) as usize;
    (0..n.min(0x4000)).any(|i| *((t + 8 + i * 0x30) as *const u16) as u32 == id)
}

unsafe fn check(object: usize) {
    let col = *((object + 0xAB8) as *const usize);
    if col == 0 {
        return;
    }
    let n = (*(col as *const i32)).clamp(0, 256) as usize;
    for k in 0..n {
        let e = col + 0x40 + k * 0x14;
        let ty = *((e + 4) as *const u8);
        if ty != 5 && ty != 14 {
            continue;
        }
        let arg = *((e + 2) as *const u16) as u32;
        let via = *((object + 0x9F0) as *const usize);
        let id = if ty == 5 { if arg == 0x83 && via != 0 { *((via + 0x2C) as *const u16) as u32 } else { arg } } else { *((object + 0xCF0) as *const u32) };
        if id == 0 || in_table(id) {
            continue;
        }
        let mut s = SEEN.lock().unwrap();
        if s.contains(&(col, id)) || s.len() > 32 {
            continue;
        }
        s.push((col, id));
        let bytes = std::slice::from_raw_parts(e as *const u8, 0x14);
        let base = BASE.load(Ordering::Relaxed);
        log!(
            "item check: object {object:#x} (Sora? {}) collision {col:#x} entry {k} of {n} {bytes:02x?}: item {id:#x} is not in the item table (+0xCF0 {:#x}, +0x9F0 {via:#x}, object vtable exe+{:#x})",
            super::sora::is_base_sora(object),
            *((object + 0xCF0) as *const u32),
            (*(object as *const usize)).wrapping_sub(base)
        );
    }
}

type WalkFn = unsafe extern "system" fn(usize, usize);
unsafe extern "system" fn walk(attack: usize, object: usize) {
    check(object);
    let o: WalkFn = std::mem::transmute(O.load(Ordering::Acquire));
    o(attack, object)
}

pub unsafe fn install(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    // push rbp; push r14; sub rsp, 0xC8 (whole, position-independent).
    hook::install(base + WALK, &[0x40, 0x55, 0x41, 0x56, 0x48, 0x81, 0xEC, 0xC8, 0x00, 0x00, 0x00], walk as *const () as usize, &O, "collision item walk 0x140407DE0");
}
