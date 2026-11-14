//! KH1's held hits under Stop, in KH2's own hit delivery (`crate::held_hits` has KH1's rules).
//!
//! KH2 delivers a hit at once (Checked (decomp)): a record from its pool (`0x1403CE740`, pool
//! `0x142A24530`, 20 of 0x40 bytes, free list `0x142A24A30`, count `0x142A24A80`; the damage is
//! worked out there, `+0x28`), the struck object's hit slot (`vtable + 0xC0`: damage
//! `0x1403D3BA0`, then its reaction), then the record goes back (`0x1403CE950`). Six places do
//! this (`call [r9+0xC0]`): `0x1401CACAB`, `0x1403D0879`, `0x1403D1930` (an attack's contact),
//! `0x14043295D` (`trap_attack_strike`), `0x140538B47`, `0x14053AADF`. Each now calls ours: a hit on
//! an object KH1's Stop holds (our stop kind running in its block, `+0x64C`) is kept, as KH1 keeps
//! it, and the record goes back to KH2's pool; any other hit goes to the slot as before. Once that
//! Stop is over, the kept hits are delivered through the same slot with a record from KH2's pool, at
//! KH1's pace. Only while hits are kept does anything of ours run each frame.

use super::{hook, log};
use crate::held_hits::{Queue, Rand, RECORD};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ON: AtomicBool = AtomicBool::new(false);
static HELD: Mutex<Option<(Queue, Rand)>> = Mutex::new(None);
/// How many hits are kept (read each frame without the lock).
static COUNT: AtomicU32 = AtomicU32::new(0);
static LOGS: AtomicU32 = AtomicU32::new(0);
/// A stand-in attack for a landing whose attack is gone (`Fake`), near the exe.
static FAKE: AtomicUsize = AtomicUsize::new(0);

/// The six `call qword ptr [r9+0xC0]` (Checked (exe)), each after `mov r9,[rax]`.
const SITES: [usize; 6] = [0x1C_ACAB, 0x3D_0879, 0x3D_1930, 0x43_295D, 0x53_8B47, 0x53_AADF];
const CALL: [u8; 7] = [0x41, 0xFF, 0x91, 0xC0, 0x00, 0x00, 0x00];
const FROM_HANDLE: usize = 0x4A_D270;
const TO_HANDLE: usize = 0x4A_D240;
/// The next live object after one (0: the first), `0x1403BF4F0`.
const NEXT_OBJECT: usize = 0x3B_F4F0;
const RECORD_FREE: usize = 0x3C_E950;
const POOL: usize = 0x2A2_4530;
const POOL_FREE: usize = 0x2A2_4A30;
const POOL_COUNT: usize = 0x2A2_4A80;
/// KH2's live attacks (`0x1403CF830` adds each: head `0x142A24B40`, next by handle `+0xC4`).
const ATTACKS: usize = 0x2A2_4B40;
const STOP_BLOCK: usize = 0x64C;

unsafe fn from_handle(h: u32) -> usize {
    let f: extern "system" fn(u32) -> usize = std::mem::transmute(BASE.load(Ordering::Relaxed) + FROM_HANDLE);
    f(h)
}
unsafe fn to_handle(p: usize) -> u32 {
    let f: extern "system" fn(usize) -> u32 = std::mem::transmute(BASE.load(Ordering::Relaxed) + TO_HANDLE);
    f(p)
}

/// KH1's Stop holds `obj`: our stop kind is running in its block.
unsafe fn held_by_stop(obj: usize) -> bool {
    let block = obj + STOP_BLOCK;
    (0..4).any(|i| *((block + 4 + 8 * i) as *const i32) == crate::magic_rows::STOP_ID as i32 && *((block + 8 + 8 * i) as *const f32) > 0.0)
}

/// The record's ATKP row (`+0x20`, a handle) is a Stop strike (KH1 handles Stop records while
/// stopped).
unsafe fn is_stop_strike(rec: usize) -> bool {
    let row = from_handle(*((rec + 0x20) as *const u32));
    row != 0 && *((row + 4) as *const u8) == crate::magic_rows::STOP_TYPE
}

unsafe fn slot(cls: usize) -> extern "system" fn(usize, usize, usize) -> usize {
    std::mem::transmute(*((*(cls as *const usize) + 0xC0) as *const usize))
}

/// KH2's reaction group of a record's reaction (`0x1403CE980`, `+0x26`): 0 none.
const REACTION_GROUP: usize = 0x3C_E980;

/// The hit through the object's own slot, then KH1's Gravity on it: its hit flattens (reaction
/// kind 8); any other hit with a reaction knocks a flattened enemy out of it (KH1 `0x1402B8B90`).
unsafe fn apply(cls: usize, obj: usize, rec: usize) -> usize {
    let gravity = {
        let row = from_handle(*((rec + 0x20) as *const u32));
        row != 0 && *((row + 4) as *const u8) == crate::magic_rows::GRAVITY_TYPE
    };
    let group: extern "system" fn(u8) -> u32 = std::mem::transmute(BASE.load(Ordering::Relaxed) + REACTION_GROUP);
    let reacts = *((rec + 0x18) as *const u32) & 1 == 0 && group(*((rec + 0x26) as *const u8)) != 0;
    let r = slot(cls)(cls, obj, rec);
    let status = *((obj + 0x5C0) as *const usize);
    if status != 0 && *(status as *const i32) > 0 {
        if gravity && *((rec + 0x28) as *const i32) > 0 {
            super::squash::flatten(obj);
        } else if reacts && super::squash::flattened(obj) {
            super::squash::knock_out(obj);
        }
    }
    r
}

/// Each delivery site's call: `(class object, struck object, record)`.
unsafe extern "system" fn deliver(cls: usize, obj: usize, rec: usize) -> usize {
    if obj != 0 && rec != 0 && ON.load(Ordering::Relaxed) && held_by_stop(obj) && !is_stop_strike(rec) {
        let mut record = [0u8; RECORD];
        std::ptr::copy_nonoverlapping(rec as *const u8, record.as_mut_ptr(), RECORD);
        // A push away from the attack (kind 5, `0x1403CEAB0` asks the attack at `+0x1C`): its centre now.
        let mut centre = None;
        if record[0x27] == 5 {
            let atk = from_handle(*((rec + 0x1C) as *const u32));
            if atk != 0 {
                let mut out = [0f32; 4];
                let f: extern "system" fn(usize, *mut [f32; 4]) -> *const [f32; 4] = std::mem::transmute(*((*(atk as *const usize) + 8) as *const usize));
                centre = Some(*f(atk, &mut out));
            }
        }
        let target = to_handle(obj);
        if let Ok(mut g) = HELD.lock() {
            let (q, r) = g.get_or_insert_with(|| (Queue::default(), Rand::new(0x4B48_3121)));
            if q.hold(target, &record, centre, r) {
                COUNT.store(q.len() as u32, Ordering::Relaxed);
                if LOGS.fetch_add(1, Ordering::Relaxed) < 20 {
                    let dmg = i32::from_le_bytes(record[0x28..0x2C].try_into().unwrap());
                    log!("magic: KH1's Stop holds a hit on {obj:#x} (damage {dmg}); {} held", q.len());
                }
                return 0;
            }
        }
    }
    apply(cls, obj, rec)
}

/// Whether `obj` is still one of KH2's live objects.
unsafe fn alive(obj: usize) -> bool {
    let next: extern "system" fn(usize) -> usize = std::mem::transmute(BASE.load(Ordering::Relaxed) + NEXT_OBJECT);
    let mut o = next(0);
    let mut n = 0;
    while o != 0 && n < 4096 {
        if o == obj {
            return true;
        }
        o = next(o);
        n += 1;
    }
    false
}

/// Whether `atk` is still one of KH2's live attacks.
unsafe fn attack_alive(atk: usize) -> bool {
    let base = BASE.load(Ordering::Relaxed);
    let mut a = *((base + ATTACKS) as *const usize);
    let mut n = 0;
    while a != 0 && n < 4096 {
        if a == atk {
            return true;
        }
        a = from_handle(*((a + 0xC4) as *const u32));
        n += 1;
    }
    false
}

/// A stand-in attack whose only use is its centre (`vtable + 8`).
#[repr(C)]
struct Fake {
    vtable: *const usize,
    pad: usize,
    centre: [f32; 4],
    table: [usize; 2],
}

unsafe extern "system" fn fake_centre(this: *const Fake, out: *mut [f32; 4]) -> *mut [f32; 4] {
    *out = (*this).centre;
    out
}

/// Delivers one kept hit through KH2's own slot, with a record from KH2's pool.
unsafe fn land(obj: usize, held: &crate::held_hits::Held) {
    let base = BASE.load(Ordering::Relaxed);
    let count = (base + POOL_COUNT) as *mut i32;
    if *count <= 0 {
        return;
    }
    let status = *((obj + 0x5C0) as *const usize);
    if status == 0 || *(status as *const i32) <= 0 {
        return; // KH1 drops a record whose actor is down (`0x1402B7B90`).
    }
    *count -= 1;
    let idx = *((base + POOL_FREE + 4 * *count as usize) as *const i32) as usize;
    let rec = base + POOL + idx * RECORD;
    std::ptr::copy_nonoverlapping(held.record.as_ptr(), rec as *mut u8, RECORD);
    if let Some(c) = held.centre {
        let atk = from_handle(*((rec + 0x1C) as *const u32));
        if !attack_alive(atk) {
            let fake = FAKE.load(Ordering::Relaxed) as *mut Fake;
            if !fake.is_null() {
                (*fake).centre = c;
                *((rec + 0x1C) as *mut u32) = to_handle(fake as usize);
            }
        }
    }
    let cls = from_handle(*(obj as *const u32));
    if cls != 0 {
        apply(cls, obj, rec);
    }
    let free: extern "system" fn(usize) = std::mem::transmute(base + RECORD_FREE);
    free(rec);
}

/// Each frame, only while hits are kept: KH1's hit pass for each object KH1's Stop no longer holds.
pub fn frame() {
    if COUNT.load(Ordering::Relaxed) == 0 {
        return;
    }
    let Ok(mut g) = HELD.lock() else { return };
    let Some((q, _)) = g.as_mut() else { return };
    for target in q.targets() {
        unsafe {
            let obj = from_handle(target);
            if obj == 0 || !alive(obj) {
                let n = q.drop_target(target);
                log!("magic: KH1's Stop: {n} held hits dropped (their target is gone)");
                continue;
            }
            if held_by_stop(obj) {
                continue;
            }
            let landing = q.pop(target);
            for h in &landing {
                land(obj, h);
            }
            if !landing.is_empty() && LOGS.fetch_add(1, Ordering::Relaxed) < 40 {
                let left = q.targets().contains(&target);
                log!("magic: KH1's Stop is over on {obj:#x}: {} held hit(s) landed{}", landing.len(), if left { ", more to come at KH1's pace" } else { ", none left" });
            }
        }
    }
    COUNT.store(q.len() as u32, Ordering::Relaxed);
}

pub unsafe fn install(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    for &s in &SITES {
        let at = base + s;
        if std::slice::from_raw_parts(at as *const u8, 7) != CALL || *((at - 3) as *const [u8; 3]) != [0x4C, 0x8B, 0x08] {
            log!("magic: held hits: the hit delivery at {at:#x} is not as expected: nothing patched");
            return;
        }
    }
    let Some(mem) = hook::alloc_near(base + SITES[0], 0x100) else {
        log!("magic: held hits: no memory near the exe");
        return;
    };
    // The stub every site calls: `jmp [rip+0]; dq deliver`. Then the stand-in attack.
    let stub = mem;
    *(stub as *mut [u8; 6]) = [0xFF, 0x25, 0, 0, 0, 0];
    *((stub + 6) as *mut usize) = deliver as *const () as usize;
    let fake = (mem + 0x40) as *mut Fake;
    *fake = Fake { vtable: std::ptr::null(), pad: 0, centre: [0.0; 4], table: [0, fake_centre as *const () as usize] };
    (*fake).vtable = (*fake).table.as_ptr();
    FAKE.store(fake as usize, Ordering::Relaxed);
    for &s in &SITES {
        let at = base + s;
        let mut new = [0xE8, 0, 0, 0, 0, 0x66, 0x90];
        new[1..5].copy_from_slice(&((stub as i64 - (at as i64 + 5)) as i32).to_le_bytes());
        hook::patch(at, &CALL, &new, "held hits: hit delivery");
    }
    ON.store(true, Ordering::Relaxed);
    log!("magic: held hits: KH2's six hit deliveries keep the hits on objects KH1's Stop holds");
}
