//! The "KH1 in KH2" row in KH2's own Config screen (the owner: a 9th row that opens our page;
//! the settings stay in `kh2-kh1.ini`). Step 1 (t39): the row, drawn and scrolled to by KH2's own
//! list; it opens nothing yet.
//!
//! KH2's Config screen (camp mode `0x24`, Checked (decomp / exe), `docs/kh2-config-menu.md`):
//! - its rows come from the static table `0x1405BC340` (8 rows of 0x14: option count, label text,
//!   up to four value texts, up to four help texts), which its functions reach by
//!   `lea reg, [rip + disp32]` at 15 places (`0x140364974` … `0x140365C67`, the bases `+0`, `+0xC`
//!   and `+0x90`). Those all point at a 9-row copy of ours instead (KH2's 8 rows, then ours).
//! - its list widget (`0x140BEECD8`) gets its row count from `mov edx, 8` before
//!   `0x1402F8F10` at `0x1403656EF` (the list's `+0x16`, the count `0x1402F7FB0` wraps at); the 8
//!   rows on screen (`0x1402F9140`, `+0x1C`) stay. With 9 rows the list scrolls, which KH2's draw
//!   (`0x140365A20`) already handles (it reads row scroll + n of the table).
//! - the row values are a byte each in an 8-byte array (`0x140365390`: `mov ecx, 8` before `new`):
//!   16 now, so our row's byte is the array's own. Its value is kept 0 (our row has one value).
//! - text ids with bit 0x8000 are read by `0x1403AD9A0` (id → text, KH2's message tables); ours
//!   (`0x7F00..`) are answered by us with KH2-encoded text (OpenKH's international table,
//!   Apache-2.0: space 0x01, 0–9 0x90.., A–Z 0x2E.., a–z 0x9A..).

use super::hook;
use super::log;
use std::sync::atomic::{AtomicUsize, Ordering};

const TABLE: usize = 0x5B_C340;
const ROWS: usize = 8;
const ROW: usize = 0x14;
/// The `disp32` fields of the `lea`s that reach the table (address of the field, its target).
const SITES: [(usize, usize); 15] = [
    (0x36_4974, 0x5B_C340),
    (0x36_498A, 0x5B_C340),
    (0x36_4FEE, 0x5B_C340),
    (0x36_5004, 0x5B_C340),
    (0x36_5096, 0x5B_C34C),
    (0x36_52FA, 0x5B_C340),
    (0x36_5846, 0x5B_C340),
    (0x36_5A0A, 0x5B_C34C),
    (0x36_5A9B, 0x5B_C340),
    (0x36_5ADF, 0x5B_C340),
    (0x36_5B18, 0x5B_C3D0),
    (0x36_5BDE, 0x5B_C3D0),
    (0x36_5BFD, 0x5B_C340),
    (0x36_5C1F, 0x5B_C3D0),
    (0x36_5C67, 0x5B_C3D0),
];
const COUNT_IMM: usize = 0x36_56F0;
const ALLOC_IMM: usize = 0x36_539A;
const OPEN: usize = 0x36_5580;
const TEXT: usize = 0x3A_D9A0;
const VALUES: usize = 0xBF_0150;

/// Our text ids (with 0x8000 in the table, as KH2's system texts).
const T_LABEL: u16 = 0x7F00;
const T_VALUE: u16 = 0x7F01;
const T_HELP: u16 = 0x7F02;

static BASE: AtomicUsize = AtomicUsize::new(0);
static O_OPEN: AtomicUsize = AtomicUsize::new(0);
static O_TEXT: AtomicUsize = AtomicUsize::new(0);
static TEXTS: std::sync::OnceLock<Vec<(u16, Vec<u8>)>> = std::sync::OnceLock::new();

/// `s` in KH2's text encoding, ended with 0. Characters it has no code for are left out.
pub fn encode(s: &str) -> Vec<u8> {
    let mut out: Vec<u8> = s
        .chars()
        .filter_map(|c| match c {
            ' ' => Some(0x01),
            '0'..='9' => Some(0x90 + (c as u8 - b'0')),
            'A'..='Z' => Some(0x2E + (c as u8 - b'A')),
            'a'..='z' => Some(0x9A + (c as u8 - b'a')),
            '/' => Some(0x4B),
            ':' => Some(0x52),
            '(' => Some(0x5A),
            ')' => Some(0x5B),
            '-' => Some(0x66),
            _ => None,
        })
        .collect();
    out.push(0);
    out
}

fn texts() -> &'static [(u16, Vec<u8>)] {
    TEXTS.get_or_init(|| {
        vec![(T_LABEL, encode("KH1 in KH2")), (T_VALUE, encode("Open")), (T_HELP, encode("KH1-Combat-in-KH2: KH1 or KH2 for each system (coming soon)"))]
    })
}

type TextFn = unsafe extern "system" fn(i32) -> usize;
unsafe extern "system" fn text(id: i32) -> usize {
    let key = (id & 0x7FFF) as u16;
    if let Some((_, t)) = texts().iter().find(|(k, _)| *k == key) {
        return t.as_ptr() as usize;
    }
    let o: TextFn = std::mem::transmute(O_TEXT.load(Ordering::Acquire));
    o(id)
}

type OpenFn = unsafe extern "system" fn();
unsafe extern "system" fn open() {
    let v = *((BASE.load(Ordering::Relaxed) + VALUES) as *const usize);
    if v != 0 {
        *((v + ROWS) as *mut u8) = 0;
    }
    let o: OpenFn = std::mem::transmute(O_OPEN.load(Ordering::Acquire));
    o()
}

pub unsafe fn install(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    // Every site must be what we expect before anything changes.
    for &(site, target) in &SITES {
        let disp = std::ptr::read_unaligned((base + site) as *const i32);
        if (base + site + 4).wrapping_add(disp as isize as usize) != base + target {
            log!("config menu: the table reference at {:#x} isn't the one expected: the KH1 in KH2 row isn't added", 0x1_4000_0000usize + site);
            return;
        }
    }
    let Some(table) = hook::alloc_near(base + TABLE, 0x1000) else {
        log!("config menu: no memory near the table");
        return;
    };
    std::ptr::copy_nonoverlapping((base + TABLE) as *const u8, table as *mut u8, ROWS * ROW);
    let ours = table + ROWS * ROW;
    let f = |o: usize, v: u16| std::ptr::write_unaligned((ours + o) as *mut u16, v);
    f(0, 1);
    f(2, 0x8000 | T_LABEL);
    f(4, 0x8000 | T_VALUE);
    f(0xC, 0x8000 | T_HELP);
    if !hook::patch(base + COUNT_IMM, &[8, 0, 0, 0], &[9, 0, 0, 0], "Config row count")
        || !hook::patch(base + ALLOC_IMM, &[8, 0, 0, 0], &[16, 0, 0, 0], "Config values array")
    {
        return;
    }
    if hook::install(base + TEXT, &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18], text as *const () as usize, &O_TEXT, "text lookup 0x1403AD9A0").is_none()
        || hook::install(base + OPEN, &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x30], open as *const () as usize, &O_OPEN, "Config open 0x140365580").is_none()
    {
        return;
    }
    for &(site, target) in &SITES {
        let disp = std::ptr::read_unaligned((base + site) as *const i32);
        let new = (table + (target - TABLE)) as i64 - (base + site + 4) as i64;
        let Ok(new) = i32::try_from(new) else {
            log!("config menu: our table is out of reach of {:#x}", 0x1_4000_0000usize + site);
            return;
        };
        hook::patch(base + site, &disp.to_le_bytes(), &new.to_le_bytes(), "Config table reference");
    }
    log!("config menu: KH1 in KH2 row added (9 rows, table at {table:#x})");
}
