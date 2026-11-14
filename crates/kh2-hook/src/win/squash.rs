//! KH1's Gravity on an enemy's height, in KH2's own per-object stop tick (`crate::squash` has
//! KH1's rules).
//!
//! KH2's object update ticks each object's stop block once a frame (`0x14040A830(object +
//! 0x64C)`, its only caller `0x1403BFDCD` in `0x1403BFD30`; Checked (decomp)). Its world matrix
//! takes the object's scale (`+0x30`, a vector; `+0x34` its height) when KH2 builds it
//! (`0x1403CCB90`: `scale × (+0xAAC × +0x3C)`, then `0x1401AA7D0`; Checked (decomp)). Our wrapper,
//! before KH2's tick: an object with KH1's squash kind running (the Gravity script puts it on each
//! enemy under the falling sphere) or KH1's flatten (the Gravity hit) takes KH1's height step that
//! frame on `+0x34`; one coming back to its height keeps stepping until it is there. Everything else
//! runs KH2's tick only.

use super::{hook, log};
use crate::magic_rows::{FLATTEN_ID, SQUASH_ID, STOP_ID};
use crate::squash::{done, step, State, FLAT_AIR, FLAT_GROUND};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

static BASE: AtomicUsize = AtomicUsize::new(0);
static O_TICK: AtomicUsize = AtomicUsize::new(0);
const TICK: usize = 0x40_A830;
const BLOCK: usize = 0x64C;
/// KH2's "in the air" (`0x1403BA9A0`: flier entry, or air bits).
const IN_AIR: usize = 0x3B_A9A0;
/// One enemy's height: (its class handle, KH1's state, KH1's height, KH2's own height).
static OBJECTS: Mutex<Option<HashMap<usize, (u32, State, f32, f32)>>> = Mutex::new(None);
static COUNT: AtomicU32 = AtomicU32::new(0);
static LOGS: AtomicU32 = AtomicU32::new(0);

unsafe fn kind(block: usize, id: u16) -> Option<f32> {
    (0..4).find_map(|i| {
        let (k, f) = (*((block + 4 + 8 * i) as *const i32), *((block + 8 + 8 * i) as *const f32));
        (k == id as i32 && f > 0.0).then_some(f)
    })
}

unsafe extern "system" fn tick(block: usize) {
    let obj = block - BLOCK;
    let squash = kind(block, SQUASH_ID).is_some();
    let flat = kind(block, FLATTEN_ID);
    if squash || flat.is_some() || COUNT.load(Ordering::Relaxed) > 0 {
        height(obj, block, squash, flat);
    }
    let o: unsafe extern "system" fn(usize) = std::mem::transmute(O_TICK.load(Ordering::Acquire));
    o(block)
}

unsafe fn height(obj: usize, block: usize, under: bool, flat: Option<f32>) {
    let Ok(mut g) = OBJECTS.lock() else { return };
    let map = g.get_or_insert_with(HashMap::new);
    let class = *(obj as *const u32);
    let y = (obj + 0x34) as *mut f32;
    let entry = match map.get(&obj) {
        Some(e) if e.0 == class => Some(*e),
        Some(_) => {
            map.remove(&obj); // another object in that memory now
            None
        }
        None => None,
    };
    let (state, h, own) = match entry {
        Some((_, s, h, own)) => (s, h, own),
        None if under || flat.is_some() => (State::Normal { phase: 0.0 }, 1.0, *y),
        None => {
            COUNT.store(map.len() as u32, Ordering::Relaxed);
            return;
        }
    };
    // KH1 runs none of this while the actor is stopped (`0x140297FE0`).
    if kind(block, STOP_ID).is_some() {
        return;
    }
    let state = match (state, flat) {
        // The hit's flatten starts (KH1 `0x1402B7DE0`): on the ground 68, in the air 75.
        // (the kind's time says which: 68 on the ground, 75 in the air, `flatten`).
        (State::Normal { .. } | State::GetUp, Some(f)) => {
            let air = f > FLAT_GROUND + 0.5;
            if LOGS.fetch_add(1, Ordering::Relaxed) < 20 {
                log!("magic: KH1's Gravity flattens {obj:#x} ({})", if air { "air, 75 frames" } else { "ground, 68 frames" });
            }
            State::Flat { timer: f, air }
        }
        // Another Gravity hit while flattened: KH1 starts the timer again.
        (State::Flat { timer, .. }, Some(f)) if f > timer + 0.5 => State::Flat { timer: f, air: f > FLAT_GROUND + 0.5 },
        // Knocked out of it (the kind taken off by a flinch): KH1's spring back.
        (State::Flat { .. }, None) => State::Normal { phase: 0.0 },
        (s, _) => s,
    };
    let (state, h) = step(state, h, under);
    *y = own * h;
    if done(state, h) {
        *y = own;
        map.remove(&obj);
    } else {
        if matches!(entry, None) && LOGS.fetch_add(1, Ordering::Relaxed) < 20 && under {
            log!("magic: KH1's Gravity squashes {obj:#x} under the falling sphere (height {own})");
        }
        map.insert(obj, (class, state, h, own));
    }
    COUNT.store(map.len() as u32, Ordering::Relaxed);
}

/// KH1's Gravity hit landed on `obj` (KH1's reaction kind 8): its flatten, through KH2's own stop
/// (`0x1403B6730`, which leaves alone what KH2 never stops: Assumption, KH1's per-actor reaction
/// rules); 68 frames on the ground, 75 in the air (KH2's `0x1403BA9A0`).
pub unsafe fn flatten(obj: usize) {
    let base = BASE.load(Ordering::Relaxed);
    let in_air: extern "system" fn(usize) -> u32 = std::mem::transmute(base + IN_AIR);
    let frames = if in_air(obj) & 0xFF != 0 { FLAT_AIR } else { FLAT_GROUND };
    let stop: extern "system" fn(usize, i32, f32) = std::mem::transmute(base + STOP_OBJECT);
    stop(obj, FLATTEN_ID as i32, frames);
}
const STOP_OBJECT: usize = 0x3B_6730;

/// A flinch on a flattened enemy knocks it out of the flatten (KH1 `0x1402B8B90`): its flatten
/// kind ends now (KH2's tick clears it this frame).
pub unsafe fn knock_out(obj: usize) {
    let block = obj + BLOCK;
    for i in 0..4 {
        if *((block + 4 + 8 * i) as *const i32) == FLATTEN_ID as i32 {
            *((block + 8 + 8 * i) as *mut f32) = 0.001;
        }
    }
}

pub fn flattened(obj: usize) -> bool {
    unsafe { kind(obj + BLOCK, FLATTEN_ID).is_some() }
}

pub unsafe fn install(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    hook::install(base + TICK, &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10], tick as *const () as usize, &O_TICK, "stop tick 0x14040A830");
}
