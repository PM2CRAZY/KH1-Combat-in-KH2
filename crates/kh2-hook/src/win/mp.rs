//! KH1's MP on base Sora, in KH2's own MP functions (`mp = kh1`, Phase 4 step 1). The logic is
//! `crate::kh1mp`; here it goes into KH2 (Checked (decomp) unless marked, Steam):
//! - the status build `0x1403C1D90 (status, record)`: the save record's `+6` / `+7` MP / max MP and
//!   `+0xF` level into status `+0x180` / `+0x184`. After it, Sora's max is KH1's bars × 10 and his
//!   MP within it. Sora's record: `0x1403A14E0 (save 0x1409ABDA0, 1)`.
//! - the MP change `0x1403D3CF0 (object, delta, flag)`: a change that would go below 0 starts the MP
//!   Charge and isn't paid. For base Sora: a cost (negative delta, the command's cost byte) is
//!   KH1's check and payment; a gain is KH1's gauge units; the result goes through KH2's own
//!   function as the change from his MP now, so it never starts the MP Charge. The max is set
//!   again each time (a level or an MP-up reward since the build).
//! - the command state `0x1403D88E0 (object, command, state)`: KH2 marks a spell he can't afford
//!   2 ("usable, starts the MP Charge"); for base Sora it is 5 (greyed) when KH1's check fails.
//! - a script's MP Charge `0x1403D4A20`: for base Sora, MP emptied instead (no charge).
//! - the costs: KH1's in KH2's `cmd` table (`kh1mp::KH1_SPELL_COSTS`), after the table loader
//!   (`combo_tables`).
//! - the hit record `0x1403CE740`: a hit he lands adds KH1's units (step 2, t61).
//! Runs on MP changes, hits and menu draws only; nothing per frame.

use super::hook;
use super::log;
use super::sora::is_base_sora;
use crate::kh1mp::{self, Mp};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const STATUS_BUILD: usize = 0x3C_1D90;
const MP_CHANGE: usize = 0x3D_3CF0;
const COMMAND_STATE: usize = 0x3D_88E0;
const RECORD_OF: usize = 0x3A_14E0;
const HANDLE: usize = 0x4A_D270;
const CHARGE_BY: usize = 0x3D_4A20;
const HIT_RECORD: usize = 0x3C_E740;
const SAVE: usize = 0x9A_BDA0;
const CMD_TABLE: usize = 0x2A1_61E8;
const CMD_ROW: usize = 0x30;

static BASE: AtomicUsize = AtomicUsize::new(0);
static O_BUILD: AtomicUsize = AtomicUsize::new(0);
static O_CHANGE: AtomicUsize = AtomicUsize::new(0);
static O_STATE: AtomicUsize = AtomicUsize::new(0);
static O_CHARGE: AtomicUsize = AtomicUsize::new(0);
static O_HIT: AtomicUsize = AtomicUsize::new(0);
static HIT_LOGGED: AtomicBool = AtomicBool::new(false);

/// KH2's swing ATKP ids that are finishers (KH1's finishers in Phase 2, `combo_tables::POWERS`).
const FINISHERS: [u16; 4] = [129, 130, 133, 134];

/// KH2 makes a hit record for every hit (`0x1403CE740 (attack, ATKP row, target, ..)`, called by
/// `0x1403D12C0`; the attack's owner is its handle `+0x10`; the record's `+0x18` bit 2 "immune",
/// `+0x28` the damage; Checked (decomp)). After it, a hit base Sora lands gives KH1's units
/// (step 2): 6 for a finisher, 0 for a magic-stat attack (ATKP `+5` = 1), else 3.
unsafe extern "system" fn hit_record(attack: usize, atkp: usize, target: usize, kind: u8) -> usize {
    let o: unsafe extern "system" fn(usize, usize, usize, u8) -> usize = std::mem::transmute(O_HIT.load(Ordering::Acquire));
    let rec = o(attack, atkp, target, kind);
    if rec == 0 || attack == 0 || atkp == 0 {
        return rec;
    }
    let base = BASE.load(Ordering::Relaxed);
    let h: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base + HANDLE);
    let owner = h(*((attack + 0x10) as *const u32));
    if owner == 0 || owner == target || !is_base_sora(owner) || *((rec + 0x18) as *const u32) & 4 != 0 || *((rec + 0x28) as *const i32) <= 0 {
        return rec;
    }
    let id = *((atkp + 2) as *const u16);
    let units = if *((atkp + 5) as *const u8) == 1 {
        0
    } else if FINISHERS.contains(&id) {
        kh1mp::KH1_FINISHER_UNITS
    } else {
        kh1mp::KH1_HIT_UNITS
    };
    if units > 0 {
        let f: unsafe extern "system" fn(usize, i32, u8) -> u64 = std::mem::transmute(base + MP_CHANGE);
        f(owner, units, 0);
        if !HIT_LOGGED.swap(true, Ordering::Relaxed) {
            log!("mp kh1: KH1's MP from Sora's hits (first: ATKP {id}, {units} units)");
        }
    }
    rec
}
static LOGGED: AtomicBool = AtomicBool::new(false);

unsafe fn sora_record() -> usize {
    let base = BASE.load(Ordering::Relaxed);
    let f: unsafe extern "system" fn(usize, u32) -> usize = std::mem::transmute(base + RECORD_OF);
    f(base + SAVE, 1)
}

/// KH1's max bars for Sora now, from his save record.
unsafe fn max_bars() -> i32 {
    let r = sora_record();
    if r == 0 {
        return kh1mp::KH1_START_BARS;
    }
    kh1mp::max_bars(*((r + 0xF) as *const u8), *((r + 7) as *const u8) as i32)
}

unsafe fn get(status: usize, max: i32) -> Mp {
    Mp::from_kh2(*((status + 0x180) as *const i32), *((status + 0x1BC) as *const f32) as i32, max)
}

unsafe fn put_max(status: usize, max: i32) {
    *((status + 0x184) as *mut i32) = max * kh1mp::MP_PER_BAR;
}

unsafe extern "system" fn build(status: usize, record: usize) {
    let o: unsafe extern "system" fn(usize, usize) = std::mem::transmute(O_BUILD.load(Ordering::Acquire));
    o(status, record);
    if status == 0 {
        return;
    }
    // The record it was built from: the one passed, or its own handle `+0x250` (as the build reads it).
    let rec = if record != 0 {
        record
    } else {
        let h: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(BASE.load(Ordering::Relaxed) + HANDLE);
        h(*((status + 0x250) as *const u32))
    };
    let sora = sora_record();
    if sora == 0 || rec != sora {
        return;
    }
    let max = max_bars();
    put_max(status, max);
    let m = get(status, max);
    *((status + 0x180) as *mut i32) = m.kh2(max);
    *((status + 0x1BC) as *mut f32) = m.units as f32;
    if !LOGGED.swap(true, Ordering::Relaxed) {
        log!("mp kh1: Sora's MP is KH1's bars: {max} bars (level {}, KH2 max MP {}), {} bars and {} units now", *((sora + 0xF) as *const u8), *((sora + 7) as *const u8), m.bars, m.units);
    }
}

unsafe extern "system" fn change(obj: usize, delta: i32, flag: u8) -> u64 {
    let o: unsafe extern "system" fn(usize, i32, u8) -> u64 = std::mem::transmute(O_CHANGE.load(Ordering::Acquire));
    if delta == 0 || !is_base_sora(obj) {
        return o(obj, delta, flag);
    }
    let status = *((obj + 0x5C0) as *const usize);
    if status == 0 || *((status + 0x1C0) as *const f32) > 0.0 {
        return o(obj, delta, flag);
    }
    let max = max_bars();
    put_max(status, max);
    let now = get(status, max);
    let next = if delta < 0 {
        match now.pay(-delta) {
            Some(m) => m,
            None => return 0,
        }
    } else {
        now.add_units(delta, max)
    };
    let (a, b) = (*((status + 0x180) as *const i32), next.kh2(max));
    *((status + 0x1BC) as *mut f32) = next.units as f32;
    if a == b {
        return 1;
    }
    o(obj, b - a, flag)
}

unsafe extern "system" fn command_state(obj: usize, cmd: usize, state: i32) -> i32 {
    let o: unsafe extern "system" fn(usize, usize, i32) -> i32 = std::mem::transmute(O_STATE.load(Ordering::Acquire));
    let r = o(obj, cmd, state);
    if r >= 4 || cmd == 0 || !is_base_sora(obj) {
        return r;
    }
    let status = *((obj + 0x5C0) as *const usize);
    let cost = *((cmd + 0x1C) as *const u8) as i32;
    if status == 0 || cost == 0 {
        return r;
    }
    let max = max_bars();
    if get(status, max).can_pay(cost) {
        // KH2's 2 means "under the cost, starts the MP Charge"; KH1's check passed.
        if r == 2 { state } else { r }
    } else {
        5
    }
}

/// KH2's MP Charge started by a script (`0x1403D4A20 (object, share)`, the one other caller of the
/// charge start `0x1403C20A0`; Checked (decomp)): for base Sora, KH1's "all MP" instead (MP and the
/// gauge emptied, no charge), as the owner chose for the Limits.
unsafe extern "system" fn charge_by(obj: usize, share: f32) {
    let o: unsafe extern "system" fn(usize, f32) = std::mem::transmute(O_CHARGE.load(Ordering::Acquire));
    if !is_base_sora(obj) {
        return o(obj, share);
    }
    let status = *((obj + 0x5C0) as *const usize);
    if status == 0 {
        return o(obj, share);
    }
    *((status + 0x180) as *mut i32) = 0;
    *((status + 0x1BC) as *mut f32) = 0.0;
}

/// KH1's spell costs on KH2's magic commands (after KH2's table loader).
pub unsafe fn patch_costs(base: usize) {
    let t = *((base + CMD_TABLE) as *const usize);
    if t == 0 {
        return;
    }
    let n = (*((t + 4) as *const i32)).clamp(0, 0x1000) as usize;
    let mut done = Vec::new();
    for i in 0..n {
        let r = t + 8 + i * CMD_ROW;
        if *((r + 2) as *const u16) != 2 {
            continue;
        }
        let arg = *((r + 4) as *const i16);
        if let Some(&(_, c)) = kh1mp::KH1_SPELL_COSTS.iter().find(|x| x.0 == arg) {
            let at = (r + 0x1C) as *mut u8;
            // A magic row with no cost in KH2 (cmd 2) is not a cast; it keeps its 0.
            if *at == 0 {
                continue;
            }
            done.push((*((r) as *const u16), *at, c));
            *at = c;
        }
    }
    log!("mp kh1: KH1's spell costs on KH2's magic commands (id, KH2's, KH1's): {done:?}");
}

pub unsafe fn install(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    let a = hook::install(base + STATUS_BUILD, &[0x40, 0x53, 0x57, 0x48, 0x83, 0xEC, 0x28], build as *const () as usize, &O_BUILD, "status build 0x1403C1D90");
    let b = a.is_some() && hook::install(base + MP_CHANGE, &[0x48, 0x89, 0x5C, 0x24, 0x08], change as *const () as usize, &O_CHANGE, "MP change 0x1403D3CF0").is_some();
    let c = b && hook::install(base + COMMAND_STATE, &[0x48, 0x89, 0x5C, 0x24, 0x08], command_state as *const () as usize, &O_STATE, "command state 0x1403D88E0").is_some();
    let d = c && hook::install(base + CHARGE_BY, &[0x48, 0x8B, 0x81, 0x48, 0x01, 0x00, 0x00], charge_by as *const () as usize, &O_CHARGE, "MP Charge start 0x1403D4A20").is_some();
    let e = d && hook::install(base + HIT_RECORD, &[0x48, 0x89, 0x5C, 0x24, 0x08], hit_record as *const () as usize, &O_HIT, "hit record 0x1403CE740").is_some();
    if e {
        log!("mp kh1: KH1's MP on base Sora ({} MP a bar, no MP Charge)", kh1mp::MP_PER_BAR);
    } else {
        log!("mp kh1: KH1's MP not used");
    }
}
