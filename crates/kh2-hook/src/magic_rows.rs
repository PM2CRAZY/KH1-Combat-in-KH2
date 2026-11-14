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

/// Gravity's attack (`kh1_assets::spells::gravity::ATKP`): KH2's `atkp` (pointer `0x142AE57B0`,
/// rows of `0x30` from `+8`, `bsearch` on (id, sub), so sorted; Checked (decomp) `0x1403EAFC0`;
/// `+0` level, `+1` sub, `+2` id, `+4` type, `+5` stat source, `+6` power, `+9` element, `+0xB`
/// hit effect group) gets 2066's rows: Blizzard's shard rows (248: every sub, the Drive Forms'
/// too) with type `GRAVITY_TYPE` (a KH2 attack type of ours, computed by `gravity_damage` in
/// KH2's damage function), KH1's power for the sub's level, no element, our hit-effect group.
pub const ATKP_ROW: usize = 0x30;
pub const GRAVITY_ATKP: u16 = kh1_assets::spells::gravity::ATKP as u16;
pub const GRAVITY_TYPE: u8 = 7;
/// KH1's Gravity powers (btltbl `0x6468` / `0x64D8` / `0x6548`, Checked (files) in KH1-in-ReCom).
pub const GRAVITY_POWER: [u16; 3] = [40, 55, 70];
const ATKP_TEMPLATE: u16 = 248;
/// Stop's attack: Thunder's burst rows (250, every sub) with type `STOP_TYPE` (KH2's own stop on
/// the struck object for KH1's time, in the damage function), no power, no reaction, no element.
pub const STOP_ATKP: u16 = kh1_assets::spells::stop::ATKP as u16;
pub const STOP_TYPE: u8 = 8;
const STOP_ATKP_TEMPLATE: u16 = 250;
/// KH2's stop kinds (`00battle.bin` `stop`, pointer `0x142AE59D8`: `1`, a count, rows of (`u16`
/// id, `u16` flags) sorted by id, `bsearch`; Checked (decomp) `0x1403EED10`): an object's stop
/// block (`+0x64C`: flags, then 4 (id, frames left) slots, `0x14040A920` / `0x14040A830`) ORs
/// the flags of its ids; flag 1 is "stopped" (`0x14040A910`; the object update `0x1403BFD30`
/// then skips its think and motion). KH1's Stop gets an id of its own (`STOP_ID`) with flag 1.
pub const STOP_ID: u16 = 0x20;
pub const STOP_FLAGS: u16 = 1;
/// KH1's Aero barrier: a kind with no flags, so KH2 only counts it down on Sora (his damage
/// taken is halved while it runs, `aero_halve`).
pub const AERO_ID: u16 = 0x21;
/// KH1's Gravity on an enemy's height (`crate::squash`): under the falling sphere (the script
/// puts this kind on each enemy touching it, one frame at a time: KH1's `+0x88 |= 4`), no flags;
/// and flattened by the hit (KH1's reaction kind 8: its 68 / 75 frames it does nothing), flag 1.
pub const SQUASH_ID: u16 = kh1_assets::spells::gravity::SQUASH_KIND as u16;
pub const FLATTEN_ID: u16 = 0x23;
/// Aero's barrier strike (Cure's 708 rows: type `AERO_TYPE` of ours, nothing else) and its wind
/// (Thunder's burst rows 250: KH2's damage, no element; KH1's power not ported yet, as the other
/// spells').
pub const AERO_ATKP: u16 = kh1_assets::spells::aero::ATKP as u16;
pub const AERO_TYPE: u8 = 9;
pub const WIND_ATKP: u16 = kh1_assets::spells::aero::WIND_ATKP as u16;
const AERO_ATKP_TEMPLATE: u16 = 708;

/// KH1's Aero barrier in KH2 frames ((18 + max MP bars) × 60, every level).
pub fn aero_frames(bars: i32) -> f32 {
    ((18 + bars) * 60) as f32
}

/// KH1's halving while the barrier runs (after the rest, `0x1402BFA80`): ÷ 2, at least 1, 0
/// stays 0.
pub fn aero_halve(d: i32) -> i32 {
    if d > 0 { (d / 2).max(1) } else { d }
}

/// KH2's `atkp` table with Gravity's rows added.
pub fn extend_atkp(t: &[u8]) -> Option<Vec<u8>> {
    let n = count(t)?;
    if t.len() < 8 + n * ATKP_ROW {
        return None;
    }
    let rows: Vec<&[u8]> = (0..n).map(|i| &t[8 + i * ATKP_ROW..8 + (i + 1) * ATKP_ROW]).collect();
    let id = |r: &[u8]| u16::from_le_bytes([r[2], r[3]]);
    if rows.last().map(|r| id(r)) >= Some(GRAVITY_ATKP) {
        return None;
    }
    let mut out = t[..8 + n * ATKP_ROW].to_vec();
    let mut added = 0;
    for r in rows.iter().filter(|r| id(r) == ATKP_TEMPLATE) {
        let mut g = r.to_vec();
        g[2..4].copy_from_slice(&GRAVITY_ATKP.to_le_bytes());
        g[4] = GRAVITY_TYPE;
        let level = ((g[1] % 10) / 2).min(2) as usize;
        g[6..8].copy_from_slice(&GRAVITY_POWER[level].to_le_bytes());
        g[9] = 0;
        g[0x0B] = kh1_assets::spells::HIT_GROUP;
        out.extend_from_slice(&g);
        added += 1;
    }
    for (template, new, kind) in [(STOP_ATKP_TEMPLATE, STOP_ATKP, Some(STOP_TYPE)), (AERO_ATKP_TEMPLATE, AERO_ATKP, Some(AERO_TYPE)), (STOP_ATKP_TEMPLATE, WIND_ATKP, None)] {
        for r in rows.iter().filter(|r| id(r) == template) {
            let mut g = r.to_vec();
            g[2..4].copy_from_slice(&new.to_le_bytes());
            if let Some(k) = kind {
                g[4] = k;
                g[6..8].copy_from_slice(&0u16.to_le_bytes());
                g[0x0A] = 0;
            }
            g[9] = 0;
            g[0x0B] = kh1_assets::spells::HIT_GROUP;
            out.extend_from_slice(&g);
            added += 1;
        }
    }
    if added == 0 {
        return None;
    }
    out[4..8].copy_from_slice(&((n + added) as u32).to_le_bytes());
    Some(out)
}

/// KH1's Gravity damage (`0x1402BFA80`, element 8; KH1-in-ReCom §7.8): current HP × (power + 5 ×
/// max MP bars)%, at least 1; every enemy at KH1's full share (owner, 2026-11-14: "(c) Everyone
/// full"). KH2's own immunity (the general resistance `+0x1AE` at 0, scripted invulnerability)
/// still stops it (Assumption: the owner's "full" is about KH1's resistances, not KH2's
/// invulnerable phases).
pub fn gravity_damage(hp: i32, power: i32, bars: i32, general: u8) -> i32 {
    if general == 0 || hp <= 0 {
        return 0;
    }
    ((hp as i64 * (power + 5 * bars) as i64 / 100) as i32).max(1)
}

/// KH1's Stop time in KH2 frames (`0x1402BEEE0`, element 9: (2 + max MP bars) × 60 ×
/// resist[9]; every enemy KH2 lets be stopped at KH1's full share, as Gravity, owner
/// 2026-11-14; KH2's own "can't be stopped", object `+0x9B8` bit 2, still holds, `0x1403B6730`).
pub fn stop_frames(bars: i32) -> f32 {
    ((2 + bars) * 60) as f32
}

/// KH2's `stop` table with KH1's Stop kind added.
pub fn extend_stop(t: &[u8]) -> Option<Vec<u8>> {
    let n = count(t)?;
    if t.len() < 8 + n * 4 {
        return None;
    }
    let last = u16::from_le_bytes(t[8 + (n - 1) * 4..][..2].try_into().ok()?);
    if last >= STOP_ID {
        return None;
    }
    let mut out = t[..8 + n * 4].to_vec();
    let ours = [(STOP_ID, STOP_FLAGS), (AERO_ID, 0), (SQUASH_ID, 0), (FLATTEN_ID, 1)];
    for (id, flags) in ours {
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&flags.to_le_bytes());
    }
    out[4..8].copy_from_slice(&((n + ours.len()) as u32).to_le_bytes());
    Some(out)
}

/// MAGC (`00battle.bin` `magc`, pointer `0x142A23C68`, rows of `0x38` from `+8`; `0x1403C5DC0`
/// takes an element's rows in order, the highest level the item count reaches; Checked
/// (decomp)): `+0` element, `+1` level, `+2` world (0 any), `+4` file name (32), `+0x24` item,
/// `+0x26` command, then the cast motions. Gravity's: Blizzard's base rows (KH1 casts Gravity on
/// Blizzard's motions), element 6, `magic/GRAVITY_n.mag`, item 84, commands 797..799.
pub const MAGC_ROW: usize = 0x38;
/// The KH2 spell whose cast rows a new one takes (KH1 casts Gravity on Blizzard's motions, Stop
/// on Thunder's).
pub fn cast_like(element: u8) -> u8 {
    if element == STOP || element == AERO { 2 } else { 1 }
}

pub fn extend_magc(t: &[u8], element: u8, file: &str, k: usize) -> Option<Vec<u8>> {
    let n = count(t)?;
    if t.len() < 8 + n * MAGC_ROW {
        return None;
    }
    let rows: Vec<&[u8]> = (0..n).map(|i| &t[8 + i * MAGC_ROW..8 + (i + 1) * MAGC_ROW]).collect();
    if rows.iter().any(|r| r[0] == element) {
        return None;
    }
    let mut out = t[..8 + n * MAGC_ROW].to_vec();
    for level in 1..=3u8 {
        let mut r = rows.iter().find(|r| r[0] == cast_like(element) && r[1] == level && r[2] == 0)?.to_vec();
        r[0] = element;
        let name = format!("magic/{file}_{level}.mag");
        r[4..0x24].fill(0);
        r[4..4 + name.len()].copy_from_slice(name.as_bytes());
        r[0x24..0x26].copy_from_slice(&ITEMS[element as usize].to_le_bytes());
        r[0x26..0x28].copy_from_slice(&command(k, level as usize - 1).to_le_bytes());
        out.extend_from_slice(&r);
    }
    out[4..8].copy_from_slice(&((n + 3) as u32).to_le_bytes());
    Some(out)
}

/// Where Sora learns Gravity, Stop and Aero (owner, 2026-11-14: approved): each level as the
/// second bonus item (`+0xE`, 0 on every Sora row of KH2's: Checked (files)) of the KH2 victory
/// bonus (`00battle.bin` `bons`, pointer `0x142AE58F8`, rows of 16: `+0` reward id, `+1`
/// character (1 Sora), applied by `0x1403EDCA0` → `0x1403EDF90`; Checked (decomp)). Reward ids
/// matched by their rewards against the game's list (Checked (files): the item names from
/// `sys.bar`): 57 Phil's Training (Aerial Dive), 19 Oogie Boogie (Item Slot, Jack +15),
/// 8 Hades (Magnet Burst), 16 the Cornerstone of Light (Dodge Slash), 4 Pete at Timeless River
/// (Reflect, King Mickey's row), 24 Luxord (Magnet Element), 3 the Dark Thorn (Retaliating Slash,
/// Beast +35), 10 the Storm Rider (Thunder Element, Mulan +25), 72 the Transport to Remembrance
/// (Item Slot, Donald +3, Goofy +4: Assumption, the last of KH2 FM's added rows with that reward).
pub const UNLOCKS: [(u8, u8); 9] = [(57, GRAVITY), (19, GRAVITY), (8, GRAVITY), (16, STOP), (4, STOP), (24, STOP), (3, AERO), (10, AERO), (72, AERO)];
pub const BONS_ROW: usize = 16;

/// The bytes to write in KH2's `bons` table (header and rows): (offset of `+0xE`, item).
pub fn unlock_writes(t: &[u8], elements: &[u8]) -> Vec<(usize, u16)> {
    let Some(n) = count(t) else { return vec![] };
    let mut out = Vec::new();
    for i in 0..n {
        let o = 8 + i * BONS_ROW;
        let Some(r) = t.get(o..o + BONS_ROW) else { break };
        if r[1] != 1 || u16::from_le_bytes([r[0xE], r[0xF]]) != 0 {
            continue;
        }
        if let Some(&(_, e)) = UNLOCKS.iter().find(|u| u.0 == r[0] && elements.contains(&u.1)) {
            out.push((o + 0xE, ITEMS[e as usize]));
        }
    }
    out
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
    fn aero_as_kh1s() {
        assert_eq!(aero_frames(3), 1260.0);
        assert_eq!((aero_halve(30), aero_halve(1), aero_halve(0)), (15, 1, 0));
    }

    #[test]
    fn gravity_takes_kh1s_share() {
        // 55 % at max MP 3 (power 40), KH1-in-ReCom's check; at least 1; KH2's immunity.
        assert_eq!(gravity_damage(1000, 40, 3, 100), 550);
        assert_eq!(gravity_damage(1, 40, 3, 100), 1);
        assert_eq!(gravity_damage(1000, 70, 9, 100), 1150);
        assert_eq!(gravity_damage(1000, 40, 3, 0), 0);
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
        let b = kh2_formats::bar::Bar::parse(&std::fs::read(std::env::var("KH2_BATTLE").unwrap()).unwrap()).unwrap().entries.iter().map(|e| (e.name.to_string(), e.data.to_vec())).collect::<Vec<_>>();
        let bget = |n: &str| b.iter().find(|e| e.0 == n).unwrap().1.clone();
        let a = extend_atkp(&bget("atkp")).unwrap();
        let ids: Vec<(u16, u8)> = (0..count(&a).unwrap()).map(|i| (u16::from_le_bytes([a[8 + i * 0x30 + 2], a[8 + i * 0x30 + 3]]), a[8 + i * 0x30 + 1])).collect();
        assert!(ids.windows(2).all(|w| w[0] <= w[1]));
        let g: Vec<_> = (0..count(&a).unwrap()).map(|i| &a[8 + i * 0x30..8 + (i + 1) * 0x30]).filter(|r| u16::from_le_bytes([r[2], r[3]]) == GRAVITY_ATKP).collect();
        assert_eq!(g.len(), 21);
        let st: Vec<_> = (0..count(&a).unwrap()).map(|i| &a[8 + i * 0x30..8 + (i + 1) * 0x30]).filter(|r| u16::from_le_bytes([r[2], r[3]]) == STOP_ATKP).collect();
        assert!(!st.is_empty() && st.iter().all(|r| r[4] == STOP_TYPE && r[0x0A] == 0));
        let w = unlock_writes(&bget("bons"), &[GRAVITY, STOP, AERO]);
        assert_eq!(w.len(), 9);
        assert_eq!(w.iter().filter(|x| x.1 == 84).count(), 3);
        let sp = extend_stop(&bget("stop")).unwrap();
        assert_eq!(count(&sp), Some(19));
        let ids: Vec<u16> = (0..count(&a).unwrap()).map(|i| u16::from_le_bytes([a[8 + i * 0x30 + 2], a[8 + i * 0x30 + 3]])).collect();
        assert!(ids.contains(&AERO_ATKP) && ids.contains(&WIND_ATKP) && ids.windows(2).all(|w| w[0] <= w[1]));
        let m2 = extend_magc(&extend_magc(&bget("magc"), GRAVITY, "GRAVITY", 0).unwrap(), STOP, "STOP", 1).unwrap();
        let last = &m2[8 + 41 * MAGC_ROW..8 + 42 * MAGC_ROW];
        assert_eq!((last[0], last[1], u16::from_le_bytes([last[0x24], last[0x25]]), u16::from_le_bytes([last[0x26], last[0x27]]), i16::from_le_bytes([last[0x28], last[0x29]])), (7, 3, 85, 802, 62));
        assert!(g.iter().all(|r| r[4] == GRAVITY_TYPE && r[9] == 0));
        assert_eq!(u16::from_le_bytes([g[5][6], g[5][7]]), 70);
        let m = extend_magc(&bget("magc"), GRAVITY, "GRAVITY", 0).unwrap();
        let last = &m[8 + 38 * MAGC_ROW..8 + 39 * MAGC_ROW];
        assert_eq!((last[0], last[1], &last[4..25], u16::from_le_bytes([last[0x24], last[0x25]]), u16::from_le_bytes([last[0x26], last[0x27]])), (6, 3, &b"magic/GRAVITY_3.mag\0\0"[..], 84, 799));
        let r = row(&i, ITEM_ROW, 86).unwrap();
        assert_eq!((r[2], r[4], u16::from_le_bytes([r[0x10], r[0x11]]), u16::from_le_bytes([r[0x12], r[0x13]])), (0x12, AERO, command(2, 0), 77));
    }
}
