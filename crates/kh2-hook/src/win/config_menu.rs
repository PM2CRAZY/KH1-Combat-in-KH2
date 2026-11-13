//! The mod's page in KH2's own Config screen (the owner: "make the config menu tab between main
//! config and mod config by using r1 and l1"; the settings stay in `kh2-kh1.ini`). L1 or R1 flips
//! the screen between KH2's 8 rows and ours: the systems, KH2 / KH1 each, drawn, moved through
//! and changed by KH2's own Config code with our rows in its table. A value changed on our page
//! switches that system at once (`super::switch`; Sora's model at the next launch) and writes the
//! ini; leaving the screen on our page gives KH2's rows and values back before KH2 saves its own.
//!
//! KH2's Config screen (camp mode `0x24`, Checked (decomp / exe), `docs/kh2-config-menu.md`):
//! - its rows come from the static table `0x1405BC340` (8 rows of 0x14: option count, label text,
//!   up to four value texts, up to four help texts), which its functions reach by
//!   `lea reg, [rip + disp32]` at 15 places (`0x140364974` … `0x140365C67`, the bases `+0`, `+0xC`
//!   and `+0x90`). Those all point at a 9-row copy of ours instead (KH2's 8 rows, then ours).
//! - its list widget (pointer at `0x140BEECD8`): `+0` the cursor, `+0x12` the scroll, `+0x16` the
//!   row count (`0x1402F8F10`; `0x1402F7FB0` wraps at it), `+0x1C` the rows on screen (8). Our page
//!   has 9 rows (the list scrolls: KH2's draw `0x140365A20` reads the table at scroll + n).
//! - the row values are a byte each in an array (`0x140365390`: `mov ecx, 8` before `new`): 16
//!   bytes now, for our 9.
//! - row 7 is drawn as one value (KH2's draw tests the row's table address, `+0x90`), so on our
//!   page row 7 is a line of help with one value, the systems on rows 0..6 and 8.
//! - left / right in its input step `0x140364D70` cycle the current row's value; on row 5 KH2 also
//!   calls `0x14019AFB0` (value 0) or `0x14019AFA0` at once (the vibration), which we put back as
//!   KH2's own value says when it happens on our page. Leaving (its events -4 / -2) runs the apply
//!   `0x140365460`, which writes the 8 values into the save options.
//! - L1 / R1: the pad record's raw word (`*(*(0x140BF31A0 + 0x48))`, PS2 bits: L1 0x400, R1 0x800;
//!   the sister project's `docs/kh2-pad.md`, Checked (decomp) there).
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
const ALLOC_IMM: usize = 0x36_539A;
const OPEN: usize = 0x36_5580;
const STEP: usize = 0x36_4D70;
const APPLY: usize = 0x36_5460;
const REDRAW: usize = 0x36_5A20;
const HELP: usize = 0x36_59E0;
const TEXT: usize = 0x3A_D9A0;
const VALUES: usize = 0xBF_0150;
const LIST: usize = 0xBE_ECD8;
const PAD: usize = 0xBF_31A0;
/// What KH2 calls for row 5's value 0 / other (`0x140364D70`).
const VIBRATION_0: usize = 0x19_AFB0;
const VIBRATION_1: usize = 0x19_AFA0;
const L1: u64 = 0x400;
const R1: u64 = 0x800;

/// Our page: (setting key, row label) in row order; row 7 is the help line.
const PAGE: [Option<(&str, &str)>; 9] = [
    Some(("movement", "Movement")),
    Some(("combos", "Combos")),
    Some(("defence", "Defence")),
    Some(("mp", "MP")),
    Some(("magic", "Magic")),
    Some(("abilities", "Abilities")),
    Some(("camera", "Camera")),
    None,
    Some(("model", "Player Model")),
];

/// Our text ids (0x8000 set in the table, as KH2's system texts).
const T_KH2: u16 = 0x7F00;
const T_KH1: u16 = 0x7F01;
const T_INFO: u16 = 0x7F02;
const T_INFO_VALUE: u16 = 0x7F03;
const T_ROW: u16 = 0x7F10;
const T_HELP: u16 = 0x7F20;

static BASE: AtomicUsize = AtomicUsize::new(0);
static TABLE_AT: AtomicUsize = AtomicUsize::new(0);
static O_OPEN: AtomicUsize = AtomicUsize::new(0);
static O_STEP: AtomicUsize = AtomicUsize::new(0);
static O_APPLY: AtomicUsize = AtomicUsize::new(0);
static O_TEXT: AtomicUsize = AtomicUsize::new(0);
static TEXTS: std::sync::OnceLock<Vec<(u16, Vec<u8>)>> = std::sync::OnceLock::new();

struct State {
    ours: bool,
    /// KH2's values while our page is up.
    kh2_values: [u8; ROWS],
    /// Our page's values (0 KH2, 1 KH1), from the ini when the screen opens.
    mod_values: [u8; 9],
    pad: u64,
}
static STATE: std::sync::Mutex<State> = std::sync::Mutex::new(State { ours: false, kh2_values: [0; ROWS], mod_values: [0; 9], pad: 0 });

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
        let mut t = vec![
            (T_KH2, encode("KH2")),
            (T_KH1, encode("KH1")),
            (T_INFO, encode("KH1 in KH2")),
            (T_INFO_VALUE, encode("L1 / R1: KH2 Config")),
        ];
        for (i, row) in PAGE.iter().enumerate() {
            let Some((key, label)) = row else { continue };
            t.push((T_ROW + i as u16, encode(label)));
            let what = crate::settings::SYSTEMS.iter().find(|(k, _)| k == key).map_or("", |(_, w)| *w);
            let when = match crate::settings::SYSTEMS.iter().position(|(k, _)| k == key) {
                Some(n) if crate::settings::NEXT_LAUNCH.contains(&n) => "Applies at the next launch",
                _ => "Applies at once",
            };
            t.push((T_HELP + 2 * i as u16, encode(&format!("KH2: {what}. {when}"))));
            t.push((T_HELP + 2 * i as u16 + 1, encode(&format!("KH1: {what}. {when}"))));
        }
        t.push((T_HELP + 2 * 7, encode("L1 / R1: back to KH2's Config. Changes apply and are saved at once")));
        t
    })
}

/// Our page's table rows.
fn page_rows() -> [[u8; ROW]; 9] {
    let mut rows = [[0u8; ROW]; 9];
    for (i, r) in rows.iter_mut().enumerate() {
        let mut put = |o: usize, v: u16| r[o..o + 2].copy_from_slice(&v.to_le_bytes());
        if PAGE[i].is_some() {
            put(0, 2);
            put(2, 0x8000 | (T_ROW + i as u16));
            put(4, 0x8000 | T_KH2);
            put(6, 0x8000 | T_KH1);
            put(0xC, 0x8000 | (T_HELP + 2 * i as u16));
            put(0xE, 0x8000 | (T_HELP + 2 * i as u16 + 1));
        } else {
            put(0, 1);
            put(2, 0x8000 | T_INFO);
            put(4, 0x8000 | T_INFO_VALUE);
            put(0xC, 0x8000 | (T_HELP + 14));
        }
    }
    rows
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

unsafe fn base() -> usize {
    BASE.load(Ordering::Relaxed)
}
unsafe fn values() -> usize {
    *((base() + VALUES) as *const usize)
}
unsafe fn list() -> usize {
    *((base() + LIST) as *const usize)
}
unsafe fn call(f: usize) {
    let f: unsafe extern "system" fn() = std::mem::transmute(base() + f);
    f()
}

/// Puts KH2's rows (`false`) or ours into the table, sets the list's row count and cursor.
unsafe fn show(ours: bool) {
    let t = TABLE_AT.load(Ordering::Relaxed);
    if ours {
        for (i, r) in page_rows().iter().enumerate() {
            std::ptr::copy_nonoverlapping(r.as_ptr(), (t + i * ROW) as *mut u8, ROW);
        }
    } else {
        std::ptr::copy_nonoverlapping((base() + TABLE) as *const u8, t as *mut u8, ROWS * ROW);
    }
    let l = list();
    if l != 0 {
        *((l + 0x16) as *mut i16) = if ours { 9 } else { ROWS as i16 };
        *(l as *mut i16) = 0;
        *((l + 0x12) as *mut i16) = 0;
    }
}

/// The ini's current choices on our page.
fn read_ini() -> [u8; 9] {
    let path = format!("{}\\kh2-kh1.ini", super::exe_dir());
    let s = std::fs::read_to_string(&path).map(|t| crate::settings::Settings::parse(&t)).unwrap_or_default();
    let mut v = [0u8; 9];
    for (i, row) in PAGE.iter().enumerate() {
        if let Some((key, _)) = row {
            v[i] = (s.style(key) == crate::settings::Style::Kh1) as u8;
        }
    }
    v
}

fn write_ini(v: &[u8; 9]) {
    let path = format!("{}\\kh2-kh1.ini", super::exe_dir());
    let mut s = std::fs::read_to_string(&path).map(|t| crate::settings::Settings::parse(&t)).unwrap_or_default();
    for (i, row) in PAGE.iter().enumerate() {
        let Some((key, _)) = row else { continue };
        if let Some(n) = crate::settings::SYSTEMS.iter().position(|(k, _)| k == key) {
            s.styles[n] = if v[i] == 1 { crate::settings::Style::Kh1 } else { crate::settings::Style::Kh2 };
        }
    }
    match std::fs::write(&path, s.render()) {
        Ok(()) => log!("config menu: saved {path}: {}", PAGE.iter().zip(v).filter_map(|(r, x)| r.map(|(k, _)| format!("{k} = {}", if *x == 1 { "kh1" } else { "kh2" }))).collect::<Vec<_>>().join(", ")),
        Err(e) => log!("config menu: could not write {path}: {e}"),
    }
}

/// Flips the page: values swapped with the page, the screen drawn again.
unsafe fn flip(st: &mut State) {
    let v = values();
    if v == 0 {
        return;
    }
    if !st.ours {
        std::ptr::copy_nonoverlapping(v as *const u8, st.kh2_values.as_mut_ptr(), ROWS);
        std::ptr::copy_nonoverlapping(st.mod_values.as_ptr(), v as *mut u8, 9);
    } else {
        std::ptr::copy_nonoverlapping(v as *const u8, st.mod_values.as_mut_ptr(), 9);
        std::ptr::copy_nonoverlapping(st.kh2_values.as_ptr(), v as *mut u8, ROWS);
    }
    st.ours = !st.ours;
    show(st.ours);
    call(REDRAW);
    call(HELP);
}

/// Back to KH2's page (values and rows), our choices kept; nothing drawn.
unsafe fn leave(st: &mut State) {
    if !st.ours {
        return;
    }
    let v = values();
    if v != 0 {
        std::ptr::copy_nonoverlapping(v as *const u8, st.mod_values.as_mut_ptr(), 9);
        std::ptr::copy_nonoverlapping(st.kh2_values.as_ptr(), v as *mut u8, ROWS);
    }
    st.ours = false;
    show(false);
    write_ini(&st.mod_values);
}

unsafe extern "system" fn open() {
    if let Ok(mut st) = STATE.lock() {
        st.ours = false;
        st.mod_values = read_ini();
        st.pad = u64::MAX;
    }
    show(false);
    let o: unsafe extern "system" fn() = std::mem::transmute(O_OPEN.load(Ordering::Acquire));
    o()
}

unsafe fn pad_word() -> u64 {
    let rec = *((base() + PAD + 0x48) as *const usize);
    if rec == 0 {
        0
    } else {
        *(rec as *const u64)
    }
}

/// Our page's 9 values while it is shown.
unsafe fn page_values() -> Option<[u8; 9]> {
    let ours = STATE.lock().map(|st| st.ours).unwrap_or(false);
    let v = values();
    if !ours || v == 0 {
        return None;
    }
    let mut out = [0u8; 9];
    std::ptr::copy_nonoverlapping(v as *const u8, out.as_mut_ptr(), 9);
    Some(out)
}

unsafe extern "system" fn step() {
    let pressed = {
        let Ok(mut st) = STATE.lock() else { return };
        let now = pad_word();
        let edge = now & !st.pad;
        st.pad = now;
        edge & (L1 | R1) != 0
    };
    if pressed {
        if let Ok(mut st) = STATE.lock() {
            flip(&mut st);
        }
    }
    let row5_before = values().checked_add(5).map(|a| *(a as *const u8));
    let before = page_values();
    let o: unsafe extern "system" fn() = std::mem::transmute(O_STEP.load(Ordering::Acquire));
    o();
    // A value changed on our page: that system switches now, and the ini is written.
    if let (Some(b), Some(a)) = (before, page_values()) {
        if a != b {
            for (i, row) in PAGE.iter().enumerate() {
                let Some((key, _)) = row else { continue };
                if a[i] != b[i] {
                    if let Some(n) = crate::settings::SYSTEMS.iter().position(|(k, _)| k == key) {
                        super::switch(n, a[i] == 1);
                    }
                }
            }
            if let Ok(mut st) = STATE.lock() {
                st.mod_values = a;
            }
            write_ini(&a);
        }
    }
    // KH2 switches the vibration at once when row 5 changes; on our page row 5 is ours.
    if let Ok(st) = STATE.lock() {
        let v = values();
        if st.ours && v != 0 && row5_before != Some(*((v + 5) as *const u8)) {
            call(if st.kh2_values[5] == 0 { VIBRATION_0 } else { VIBRATION_1 });
        }
    }
}

unsafe extern "system" fn apply() {
    if let Ok(mut st) = STATE.lock() {
        leave(&mut st);
    }
    let o: unsafe extern "system" fn() = std::mem::transmute(O_APPLY.load(Ordering::Acquire));
    o()
}

pub unsafe fn install(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    for &(site, target) in &SITES {
        let disp = std::ptr::read_unaligned((base + site) as *const i32);
        if (base + site + 4).wrapping_add(disp as isize as usize) != base + target {
            log!("config menu: the table reference at {:#x} isn't the one expected: no mod page", 0x1_4000_0000usize + site);
            return;
        }
    }
    let Some(table) = hook::alloc_near(base + TABLE, 0x1000) else {
        log!("config menu: no memory near the table");
        return;
    };
    TABLE_AT.store(table, Ordering::Relaxed);
    std::ptr::copy_nonoverlapping((base + TABLE) as *const u8, table as *mut u8, ROWS * ROW);
    if !hook::patch(base + ALLOC_IMM, &[8, 0, 0, 0], &[16, 0, 0, 0], "Config values array") {
        return;
    }
    let p15a = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x30];
    let p15b = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x20];
    if hook::install(base + TEXT, &[0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18], text as *const () as usize, &O_TEXT, "text lookup 0x1403AD9A0").is_none()
        || hook::install(base + OPEN, &p15a, open as *const () as usize, &O_OPEN, "Config open 0x140365580").is_none()
        || hook::install(base + STEP, &p15a, step as *const () as usize, &O_STEP, "Config step 0x140364D70").is_none()
        || hook::install(base + APPLY, &p15b, apply as *const () as usize, &O_APPLY, "Config apply 0x140365460").is_none()
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
    log!("config menu: L1 / R1 flips KH2's Config and the mod's page (table at {table:#x})");
}
