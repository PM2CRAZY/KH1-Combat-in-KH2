//! KH1's action abilities in their KH2 pairs' slots (`docs/abilities.md`;
//! `kh1_assets::abilities::PAIRS`). At `abilities = kh1` KH2's `item` table (pointer
//! `0x142A25370`, rows of 24 from `+8`, found by a walk: `0x1403E39A0`) is a copy in which each
//! paired KH2 ability row carries KH1's ability: its name and description (`0x8000 | message`,
//! `+8` / `+0xA`), KH1's AP cost (`+7`, summed by KH2's AP total `0x140351560`) and an ability
//! number of ours (`+4`, the bit KH2's ability block sets for an equipped ability:
//! `0x140401730`'s last lines). Everything else in the row stays KH2's (kind, its "not in a Drive
//! Form" byte `+6`, icons). Nothing of ours runs after that: KH2's ability screen, AP total, equip
//! and ability block read the row like any other.
//!
//! What it does to KH2's own reads (Checked (decomp)): Sora's save list keeps KH2's item id
//! (`0x1403A1330` adds by id), so KH2 gives the KH1 ability wherever it gives the pair, and its
//! scripts' "is this item equipped" (`0x1401B40E0`, by id) still see KH2's item. KH2's combo rows
//! (`ptya +0x40`, `0x140402750`) and its other ability checks test KH2's ability number, which the
//! paired row no longer sets: the KH2 move is off while KH1's stands in its slot. The ability
//! switch `0x140401730` keys on the item id, so no paired item has a case there
//! (`kh1_assets::abilities`' test).

use kh1_assets::abilities::{number, PAIRS};
use kh1_assets::spell_names::{ability_description, ability_name};

pub const ITEM_ROW: usize = 24;

fn count(t: &[u8]) -> Option<usize> {
    Some(u32::from_le_bytes(t.get(4..8)?.try_into().ok()?) as usize)
}

/// KH2's `item` table (header and rows) with each paired row carrying KH1's ability. `None` if
/// the table is short, a paired row is missing or isn't an ability (kind `0x13`).
pub fn kh1_items(t: &[u8]) -> Option<Vec<u8>> {
    let n = count(t)?;
    if t.len() < 8 + n * ITEM_ROW {
        return None;
    }
    let mut out = t[..8 + n * ITEM_ROW].to_vec();
    for (k, p) in PAIRS.iter().enumerate() {
        let i = (0..n).find(|&i| u16::from_le_bytes([out[8 + i * ITEM_ROW], out[9 + i * ITEM_ROW]]) == p.kh2_item)?;
        let r = &mut out[8 + i * ITEM_ROW..8 + (i + 1) * ITEM_ROW];
        if r[2] != 0x13 {
            return None;
        }
        r[4..6].copy_from_slice(&number(k).to_le_bytes());
        r[7] = p.ap;
        r[8..10].copy_from_slice(&(0x8000 | ability_name(k)).to_le_bytes());
        r[10..12].copy_from_slice(&(0x8000 | ability_description(k)).to_le_bytes());
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(rows: &[(u16, u8, u16, u8)]) -> Vec<u8> {
        let mut t = vec![0u8; 8];
        t[4..8].copy_from_slice(&(rows.len() as u32).to_le_bytes());
        for &(id, kind, num, ap) in rows {
            let mut r = [0u8; ITEM_ROW];
            r[0..2].copy_from_slice(&id.to_le_bytes());
            r[2] = kind;
            r[4..6].copy_from_slice(&num.to_le_bytes());
            r[6] = 2;
            r[7] = ap;
            r[8..10].copy_from_slice(&0x8123u16.to_le_bytes());
            r[0x17] = 3;
            t.extend_from_slice(&r);
        }
        t
    }

    #[test]
    fn paired_rows_carry_kh1s_ability_and_the_rest_stays() {
        let mut rows: Vec<_> = PAIRS.iter().map(|p| (p.kh2_item, 0x13, 90, 2)).collect();
        rows.push((162, 0x13, 21, 1));
        let t = table(&rows);
        let out = kh1_items(&t).unwrap();
        assert_eq!(out.len(), t.len());
        for (k, p) in PAIRS.iter().enumerate() {
            let r = &out[8 + k * ITEM_ROW..8 + (k + 1) * ITEM_ROW];
            assert_eq!(u16::from_le_bytes([r[0], r[1]]), p.kh2_item);
            assert_eq!(u16::from_le_bytes([r[4], r[5]]), 200 + k as u16);
            assert_eq!(r[6], 2);
            assert_eq!(r[7], p.ap);
            assert_eq!(u16::from_le_bytes([r[8], r[9]]), 0x8000 | ability_name(k));
            assert_eq!(r[0x17], 3);
        }
        let last = PAIRS.len();
        assert_eq!(&out[8 + last * ITEM_ROW..], &t[8 + last * ITEM_ROW..]);
        // A missing pair: nothing.
        assert!(kh1_items(&table(&rows[1..])).is_none());
    }

    /// With the player's `03system.bin` (`KH2_SYSTEM`): every paired item is a Sora action
    /// ability (`+6` = 2), the copy differs from KH2's only in the paired rows' `+4..+12`.
    #[test]
    fn players_items() {
        let Ok(p) = std::env::var("KH2_SYSTEM") else { return };
        let sys = std::fs::read(p).unwrap();
        let bar = kh2_formats::bar::Bar::parse(&sys).unwrap();
        let t = bar.entries.iter().find(|e| e.name == "item").unwrap().data;
        let n = count(t).unwrap();
        let out = kh1_items(t).unwrap();
        let mut changed = 0;
        for i in 0..n {
            let (a, b) = (&t[8 + i * ITEM_ROW..8 + (i + 1) * ITEM_ROW], &out[8 + i * ITEM_ROW..8 + (i + 1) * ITEM_ROW]);
            let id = u16::from_le_bytes([a[0], a[1]]);
            if PAIRS.iter().any(|p| p.kh2_item == id) {
                assert_eq!((a[2], a[6]), (0x13, 2), "item {id}");
                assert_eq!(&a[..4], &b[..4]);
                assert_eq!(&a[12..], &b[12..]);
                changed += 1;
            } else {
                assert_eq!(a, b);
                // Our numbers are no KH2 ability's.
                if a[2] == 0x13 {
                    assert!(u16::from_le_bytes([a[4], a[5]]) < 200);
                }
            }
        }
        assert_eq!(changed, PAIRS.len());
    }
}
