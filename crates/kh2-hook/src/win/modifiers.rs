//! KH1's combo modifiers in KH2's own picker (`crate::modifiers`, `docs/abilities.md` t93), at
//! `abilities = kh1`:
//! - Sora's `ptya` set (table `0x142AE57B8`, set 1, read by his `+0xD50`) rewritten in place, the
//!   same size, in KH1's row order with our rows (`modifiers::kh1_set`); KH2's bytes back at kh2.
//!   The combos patch (`combo_tables`) finds its rows by id, so ours is taken off around it.
//! - KH1's row tests for our rows only, in KH2's checks: the pick `0x140402D20` (KH1's view of the
//!   target and one draw for the finisher slices), the part test `0x1404029F0` (KH1's side enemy),
//!   the window test `0x1404027F0` (our kind-1 / kind-2 rows: KH1's decision instead of KH2's
//!   windows), the row check `0x140402750` (our kind-3 rows: KH1's slices, Blitz, Hurricane
//!   Blast). Every other row, and everything at kh2, runs KH2's own code untouched.
//! Runs on an Attack press only (KH2's picker), nothing per frame.

use super::combo_tables::Saved;
use super::{hook, log};
use crate::modifiers::{self, Pick, Target};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

const PTYA: usize = 0x2AE_57B8;
const SORA_SET: usize = 1;
const PICK_FN: usize = 0x40_2D20;
const PART_FN: usize = 0x40_29F0;
const WINDOW_FN: usize = 0x40_27F0;
const ROW_FN: usize = 0x40_2750;
/// KH2's jump height of an actor (`0x1403B5A40`, with High Jump; KH1's 190 / 290 at movement = kh1).
const JUMP_HEIGHT: usize = 0x3B_5A40;

static BASE: AtomicUsize = AtomicUsize::new(0);
static O_PICK: AtomicUsize = AtomicUsize::new(0);
static O_PART: AtomicUsize = AtomicUsize::new(0);
static O_WINDOW: AtomicUsize = AtomicUsize::new(0);
static O_ROW: AtomicUsize = AtomicUsize::new(0);
static SAVED_SET: Saved = Saved::new();
static SEED: AtomicU32 = AtomicU32::new(0x2545_F491);
static PICK: Mutex<Pick> = Mutex::new(Pick { target: None, reach: 190.0, r: 1.0, stun: false, gravity: false, zantetsuken: false, sliding_dash: false, side: false });

pub unsafe fn install(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    // 4C 8B DC mov r11, rsp; 53 push rbx; 41 56 push r14.
    hook::install(base + PICK_FN, &[0x4C, 0x8B, 0xDC, 0x53, 0x41, 0x56], pick as *const () as usize, &O_PICK, "combo pick 0x140402D20");
    // 48 8B C4 mov rax, rsp; 48 89 58 20 mov [rax+0x20], rbx.
    hook::install(base + PART_FN, &[0x48, 0x8B, 0xC4, 0x48, 0x89, 0x58, 0x20], part as *const () as usize, &O_PART, "combo part test 0x1404029F0");
    // 40 53 push rbx; 48 83 EC 60 sub rsp, 0x60.
    hook::install(base + WINDOW_FN, &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x60], window as *const () as usize, &O_WINDOW, "combo window test 0x1404027F0");
    // 48 89 74 24 10 mov [rsp+0x10], rsi.
    hook::install(base + ROW_FN, &[0x48, 0x89, 0x74, 0x24, 0x10], row_check as *const () as usize, &O_ROW, "combo row check 0x140402750");
}

fn on() -> bool {
    crate::settings::kh1(crate::settings::ABILITIES)
}

unsafe fn f(at: usize) -> f32 {
    *(at as *const f32)
}

/// KH2 loaded its tables again: its bytes, nothing of ours saved.
pub fn forget() {
    SAVED_SET.forget();
}

/// Ours off (KH2's set back), e.g. around the combos patch.
pub unsafe fn unapply() {
    let n = SAVED_SET.restore();
    if n > 0 {
        log!("modifiers: KH2's combo rows back");
    }
}

/// KH1's rows in Sora's set (kh1) or KH2's (kh2).
pub unsafe fn apply(kh1: bool) {
    unapply();
    let base = BASE.load(Ordering::Relaxed);
    if !kh1 || base == 0 {
        return;
    }
    let t = *((base + PTYA) as *const usize);
    if t == 0 || SORA_SET as i32 >= *((t + 4) as *const i32) {
        return;
    }
    let off = *((t + 8 + SORA_SET * 4) as *const i32);
    if off <= 0 {
        return;
    }
    let set = t + off as usize;
    let n = (*(set as *const i32)).clamp(0, 256) as usize;
    let own = std::slice::from_raw_parts(set as *const u8, 4 + n * modifiers::PTYA_ROW);
    match modifiers::kh1_set(own) {
        Some(new) => {
            SAVED_SET.write(set, &new);
            let ids: Vec<u8> = (0..n).map(|i| new[4 + i * modifiers::PTYA_ROW]).collect();
            log!("modifiers: Sora's combo rows in KH1's order (ids {ids:?})");
        }
        None => log!("modifiers: Sora's combo rows aren't KH2's as we know them; KH1's modifiers not in"),
    }
}

fn rand() -> f32 {
    let mut x = SEED.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    SEED.store(x, Ordering::Relaxed);
    (x >> 8) as f32 / (1u32 << 24) as f32
}

unsafe fn equipped(sora: usize, k: usize) -> bool {
    let status = *((sora + 0x5C0) as *const usize);
    if status == 0 {
        return false;
    }
    let n = kh1_assets::abilities::number(k) as usize;
    (*((status + 0x1D0 + (n >> 5) * 4) as *const u32) >> (n & 31)) & 1 != 0
}

unsafe extern "C" fn pick(ctx: usize) -> usize {
    let o: unsafe extern "C" fn(usize) -> usize = std::mem::transmute(O_PICK.load(Ordering::Acquire));
    if !on() {
        return o(ctx);
    }
    let sora = *((ctx + 0x40) as *const usize);
    let target = (*(ctx as *const usize) != 0).then(|| {
        let (dx, dy, dz) = (f(ctx + 0x20) - f(ctx + 0x50), f(ctx + 0x24) - f(ctx + 0x54), f(ctx + 0x28) - f(ctx + 0x58));
        Target { gap: (dx * dx + dz * dz).sqrt() - f(ctx + 0xC), h: -dy }
    });
    let jump: unsafe extern "C" fn(usize) -> f32 = std::mem::transmute(BASE.load(Ordering::Relaxed) + JUMP_HEIGHT);
    let p = Pick {
        target,
        reach: jump(sora),
        r: rand(),
        stun: equipped(sora, 10),
        gravity: equipped(sora, 11),
        zantetsuken: equipped(sora, 12),
        sliding_dash: equipped(sora, 4),
        side: false,
    };
    if let Ok(mut g) = PICK.lock() {
        *g = p;
    }
    o(ctx)
}

unsafe extern "C" fn part(ctx: usize, part: usize, kind: u32) {
    let o: unsafe extern "C" fn(usize, usize, u32) = std::mem::transmute(O_PART.load(Ordering::Acquire));
    if on() && kind == 2 && part != 0 {
        let (dx, dz) = (f(part + 0x18) - f(ctx + 0x50), f(part + 0x20) - f(ctx + 0x58));
        let len = (dx * dx + dz * dz).sqrt();
        let (ax, az) = (f(ctx + 0x30), f(ctx + 0x38));
        let al = (ax * ax + az * az).sqrt();
        if len > 0.0 && al > 0.0 && modifiers::side_part(len - f(part + 4), (dx * ax + dz * az) / (len * al)) {
            if let Ok(mut g) = PICK.lock() {
                g.side = true;
            }
        }
    }
    o(ctx, part, kind)
}

unsafe fn ours(row: usize) -> Option<modifiers::Row> {
    let n = *((row + 0x40) as *const u16);
    if n < kh1_assets::abilities::FIRST_NUMBER {
        return None;
    }
    modifiers::row_of(n, *(row as *const u8))
}

unsafe extern "C" fn window(row: usize, sora: usize, gap: f32, dy: f32, dot: f32, part: usize) -> u64 {
    let o: unsafe extern "C" fn(usize, usize, f32, f32, f32, usize) -> u64 = std::mem::transmute(O_WINDOW.load(Ordering::Acquire));
    if on() {
        if let Some(r) = ours(row) {
            let p = PICK.lock().map(|g| *g).unwrap_or(Pick { target: None, reach: 190.0, r: 1.0, stun: false, gravity: false, zantetsuken: false, sliding_dash: false, side: false });
            return modifiers::valid(r, &p, Some((gap, dot))) as u64;
        }
    }
    o(row, sora, gap, dy, dot, part)
}

unsafe extern "C" fn row_check(row: usize, sora: usize) -> u64 {
    let o: unsafe extern "C" fn(usize, usize) -> u64 = std::mem::transmute(O_ROW.load(Ordering::Acquire));
    let v = o(row, sora);
    if v & 0xFF == 0 || !on() || *((row + 1) as *const u8) != 3 {
        return v;
    }
    match ours(row) {
        Some(r) => {
            let p = PICK.lock().map(|g| *g).unwrap_or(Pick { target: None, reach: 190.0, r: 1.0, stun: false, gravity: false, zantetsuken: false, sliding_dash: false, side: false });
            if modifiers::valid(r, &p, None) { v } else { v & !0xFF }
        }
        None => v,
    }
}
