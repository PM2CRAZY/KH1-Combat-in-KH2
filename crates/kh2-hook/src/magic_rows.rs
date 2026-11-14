//! Gravity, Stop and Aero in KH2's own `cmd` and `item` tables (step 4 of
//! `docs/command-menu-rows.md`): after KH2's table loader, each table is copied with our rows
//! added and KH2's pointer set to the copy; KH2's own lookups then find them like any other row.
//! Nothing of ours runs after that.
//!
//! - `cmd` (`03system.bin`, pointer `0x142A161E8`, rows of `0x30` from `+8`, count `+4`, found by
//!   `bsearch` on the id, so sorted; Checked (decomp) `0x1403B68B0`): a magic command is execute
//!   2 (`+2`), its element in `+4`, its name `0x8000 | message` in `+8`, its MP cost in `+0x1C`
//!   (Checked (files): Fire 49 / 119 / 120 … Reflect 177..179). Ours: 797..805 (KH2's last is
//!   796), Thunder's rows (51, 123, 124) with our element, name and cost.
//! - `item` (pointer `0x142A25370`, rows of 24 from `+8`, found by a walk; `0x1403E39A0`): a magic
//!   item is kind `0x12`, `+4` its element, `+8` / `+0xA` name and description, `+0x10` its
//!   command, `+0x12` its slot in the counts `0x1409ACE30` (the count is the spell's level,
//!   `0x1403C4720`). Ours: items 84, 85, 86 (unused ids), slots 75, 76, 77 (unused by any item,
//!   Checked (files)), on Fire's row (21).

use crate::menu_rows::{AERO, GRAVITY, ITEMS, STOP};
use kh1_assets::spell_names;

pub const CMD_ROW: usize = 0x30;
pub const ITEM_ROW: usize = 24;
/// Our commands: element `k` (0 Gravity, 1 Stop, 2 Aero), level `l`.
pub const FIRST_COMMAND: u16 = 797;
pub const ELEMENTS: [u8; 3] = [GRAVITY, STOP, AERO];
/// Slots in KH2's item counts.
pub const SLOTS: [u16; 3] = [75, 76, 77];
/// Thunder's commands (levels 1..3) and Fire's item: the rows ours are made on.
const CMD_TEMPLATES: [u16; 3] = [51, 123, 124];
const ITEM_TEMPLATE: u16 = 21;

pub fn command(k: usize, l: usize) -> u16 {
    FIRST_COMMAND + (3 * k + l) as u16
}

fn count(t: &[u8]) -> Option<usize> {
    Some(u32::from_le_bytes(t.get(4..8)?.try_into().ok()?) as usize)
}

fn row<'a>(t: &'a [u8], size: usize, id: u16) -> Option<&'a [u8]> {
    (0..count(t)?).map(|i| &t[8 + i * size..8 + (i + 1) * size]).find(|r| u16::from_le_bytes([r[0], r[1]]) == id)
}

/// KH2's `cmd` table (header and rows) with ours added; `None` if it already has them or lacks
/// the template rows.
pub fn extend_commands(t: &[u8]) -> Option<Vec<u8>> {
    let n = count(t)?;
    if t.len() < 8 + n * CMD_ROW || row(t, CMD_ROW, FIRST_COMMAND).is_some() {
        return None;
    }
    let last = u16::from_le_bytes(t[8 + (n - 1) * CMD_ROW..][..2].try_into().ok()?);
    if last >= FIRST_COMMAND {
        return None;
    }
    let mut out = t[..8 + n * CMD_ROW].to_vec();
    for k in 0..3 {
        for l in 0..3 {
            let mut r = row(t, CMD_ROW, CMD_TEMPLATES[l])?.to_vec();
            r[0..2].copy_from_slice(&command(k, l).to_le_bytes());
            r[4..6].copy_from_slice(&(ELEMENTS[k] as i16).to_le_bytes());
            r[8..12].copy_from_slice(&(0x8000u32 | spell_names::command_message(k, l) as u32).to_le_bytes());
            out.extend_from_slice(&r);
        }
    }
    out[4..8].copy_from_slice(&((n + 9) as u32).to_le_bytes());
    Some(out)
}

/// KH2's `item` rows (header and rows) with ours added.
pub fn extend_items(t: &[u8]) -> Option<Vec<u8>> {
    let n = count(t)?;
    if t.len() < 8 + n * ITEM_ROW || row(t, ITEM_ROW, ITEMS[GRAVITY as usize]).is_some() {
        return None;
    }
    let template = row(t, ITEM_ROW, ITEM_TEMPLATE)?.to_vec();
    let mut out = t[..8 + n * ITEM_ROW].to_vec();
    for k in 0..3 {
        let mut r = template.clone();
        r[0..2].copy_from_slice(&ITEMS[ELEMENTS[k] as usize].to_le_bytes());
        r[4] = ELEMENTS[k];
        r[8..10].copy_from_slice(&(0x8000 | spell_names::item_name(k)).to_le_bytes());
        r[10..12].copy_from_slice(&(0x8000 | spell_names::item_description(k)).to_le_bytes());
        r[0x10..0x12].copy_from_slice(&command(k, 0).to_le_bytes());
        r[0x12..0x14].copy_from_slice(&SLOTS[k].to_le_bytes());
        out.extend_from_slice(&r);
    }
    out[4..8].copy_from_slice(&((n + 3) as u32).to_le_bytes());
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(rows: &[Vec<u8>]) -> Vec<u8> {
        let mut t = vec![1, 0, 0, 0];
        t.extend((rows.len() as u32).to_le_bytes());
        for r in rows {
            t.extend(r);
        }
        t
    }

    #[test]
    fn commands_added_sorted_on_thunders() {
        let mut rows = Vec::new();
        for id in [2u16, 51, 123, 124, 796] {
            let mut r = vec![0u8; CMD_ROW];
            r[0..2].copy_from_slice(&id.to_le_bytes());
            r[2] = 2;
            r[0x1C] = 18;
            rows.push(r);
        }
        let t = extend_commands(&table(&rows)).unwrap();
        assert_eq!(count(&t), Some(14));
        let ids: Vec<u16> = (0..14).map(|i| u16::from_le_bytes([t[8 + i * CMD_ROW], t[9 + i * CMD_ROW]])).collect();
        assert!(ids.windows(2).all(|w| w[0] < w[1]));
        let r = row(&t, CMD_ROW, command(1, 2)).unwrap();
        assert_eq!((r[2], r[4], r[0x1C]), (2, STOP, 18));
        assert_eq!(u32::from_le_bytes(r[8..12].try_into().unwrap()), 0x8000 | 0x5705);
        assert!(extend_commands(&t).is_none());
    }

    /// With the player's `03system.bin` (`KH2_SYSTEM`).
    #[test]
    fn players_tables() {
        let Ok(p) = std::env::var("KH2_SYSTEM") else { return };
        let d = std::fs::read(p).unwrap();
        let bar = kh2_formats::bar::Bar::parse(&d).unwrap();
        let get = |n: &str| bar.entries.iter().find(|e| e.name == n).unwrap().data.to_vec();
        let c = extend_commands(&get("cmd")).unwrap();
        assert_eq!(count(&c), Some(751 + 9));
        let i = extend_items(&get("item")).unwrap();
        let r = row(&i, ITEM_ROW, 86).unwrap();
        assert_eq!((r[2], r[4], u16::from_le_bytes([r[0x10], r[0x11]]), u16::from_le_bytes([r[0x12], r[0x13]])), (0x12, AERO, command(2, 0), 77));
    }
}
