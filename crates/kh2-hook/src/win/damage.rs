//! KH1's damage on base Sora's hits (`damage = kh1`; `crate::kh1dmg` has KH1's numbers), inside
//! KH2's own damage function (`0x1403C12F0`, wrapped in `gravity.rs`).
//!
//! KH2's numbers it reads (Checked (decomp)): the damage function's `(target status, attack, ATKP
//! row, which)`; the attack's owner handle `+0x10`; an object's status `+0x5C0`; status `+0` HP,
//! `+0x188` Strength (what KH2's own formula reads for a Strength attack), `+0x1AE` the general
//! resistance (0: KH2 makes it take nothing), `+0x270` the handle of its `enmp` row (`0x1403C06B0`
//! compares it); `enmp` (pointer `0x142AE57C0`, rows of 0x5C from `+8`, `0x1403EBF30`): `+0` the
//! enemy id, `+2` its level (0: the world's, `0x1403C2240`); `lvpm` (pointer `0x142AE5978`, rows of
//! 12, `0x1403EE8A0`) and the world's row (`0x142AE5980`, `0x1403EE8C0`).

use super::log;
use kh1_assets::kh1_records::Record;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

static BASE: AtomicUsize = AtomicUsize::new(0);
static KH1: OnceLock<(Vec<Record>, Vec<u16>)> = OnceLock::new();
static CACHE: Mutex<Vec<((u16, i32), Record)>> = Mutex::new(Vec::new());
static RAND: AtomicU32 = AtomicU32::new(0x2545_F491);
static LOGS: AtomicU32 = AtomicU32::new(0);

const FROM_HANDLE: usize = 0x4A_D270;
const ENMP: usize = 0x2AE_57C0;
const ENMP_ROW: usize = 0x5C;
const LVPM: usize = 0x2AE_5978;
const LVPM_WORLD: usize = 0x2AE_5980;
const SORA: usize = 0x2AE_9A28;

/// KH1's records and curve, read once from the player's KH1 files (on a thread at start).
pub fn load(base: usize, root: String) {
    BASE.store(base, Ordering::Relaxed);
    std::thread::spawn(move || {
        let (records, curve, notes) = kh1_assets::kh1_records::load(&root);
        for n in notes.iter().take(10) {
            log!("damage kh1: {n}");
        }
        let Some(curve) = curve else {
            log!("damage kh1: no KH1 curve: KH2's damage stays");
            return;
        };
        log!("damage kh1: {} KH1 enemy records, curve of {}", records.len(), curve.len());
        let _ = KH1.set((records, curve));
    });
}

unsafe fn from_handle(h: u32) -> usize {
    let f: extern "system" fn(u32) -> usize = std::mem::transmute(BASE.load(Ordering::Relaxed) + FROM_HANDLE);
    f(h)
}

fn roll() -> f32 {
    let mut x = RAND.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    RAND.store(x, Ordering::Relaxed);
    (x & 0xFF_FFFF) as f32 / 16_777_216.0
}

/// The KH2 enemy (id, level) a status belongs to, from its `enmp` row.
unsafe fn enemy(status: usize) -> Option<(u16, i32)> {
    let base = BASE.load(Ordering::Relaxed);
    let t = *((base + ENMP) as *const usize);
    if t == 0 {
        return None;
    }
    let n = (*((t + 4) as *const i32)).clamp(0, 0x10000) as usize;
    let row = from_handle(*((status + 0x270) as *const u32));
    if row < t + 8 || row >= t + 8 + n * ENMP_ROW || (row - t - 8) % ENMP_ROW != 0 {
        return None;
    }
    let id = *(row as *const u16);
    let mut level = *((row + 2) as *const i16) as i32;
    if level == 0 {
        let (lv, world) = (*((base + LVPM) as *const usize), *((base + LVPM_WORLD) as *const usize));
        if lv != 0 && world >= lv {
            level = ((world - lv) / 12) as i32 + 1;
        }
    }
    Some((id, level))
}

/// KH1's number for this hit, or None (KH2's own damage).
pub unsafe fn kh1(status: *const i32, attack: usize, atkp: *const u8, kh2: impl Fn() -> i32) -> Option<i32> {
    if !crate::settings::kh1(crate::settings::DAMAGE) || atkp.is_null() || attack == 0 {
        return None;
    }
    let (records, curve) = KH1.get()?;
    let base = BASE.load(Ordering::Relaxed);
    let owner = from_handle(*((attack + 0x10) as *const u32));
    if owner == 0 || owner != *((base + SORA) as *const usize) || !super::sora::is_base_sora(owner) {
        return None;
    }
    let (id, sub) = (u16::from_le_bytes([*atkp.add(2), *atkp.add(3)]), *atkp.add(1));
    let atk = crate::kh1dmg::sora_attack(id, sub)?;
    let bars = super::mp::max_bars();
    // Cure (KH1 element 11: the power as it is) heals Sora; no enemy record.
    if atk.element == 11 {
        return Some(atk.power + bars);
    }
    let st = *((owner + 0x5C0) as *const usize);
    if st == 0 {
        return None;
    }
    let strength = *((st + 0x188) as *const u16) as i32;
    if *(status as *const u8).add(0x1AE) == 0 {
        return Some(0);
    }
    let (enemy, level) = enemy(status as usize)?;
    let rec = {
        let mut c = CACHE.lock().ok()?;
        match c.iter().find(|e| e.0 == (enemy, level)) {
            Some(e) => e.1.clone(),
            None => {
                let r = kh1_assets::kh1_records::pick(records, enemy, level)?;
                log!("damage kh1: KH2 enemy {enemy} level {level} takes KH1's {} {:#x} (level {}, Defence {}, floor {} / cap {})", r.model, r.offset, r.level, r.defence, r.floor, r.cap);
                c.push(((enemy, level), r.clone()));
                r
            }
        }
    };
    let t = crate::kh1dmg::Target { hp: *status, defence: rec.defence, floor: rec.floor, cap: rec.cap, resist: rec.resist };
    let d = crate::kh1dmg::damage(&atk, strength, bars, &t, curve, roll());
    if LOGS.fetch_add(1, Ordering::Relaxed) < 30 {
        log!("damage kh1: ATKP {id}/{sub} on enemy {enemy} (HP {}): Strength {strength}, max MP {bars} -> {d} (KH2's: {})", *status, kh2());
    }
    Some(d)
}
