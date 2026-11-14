//! KH1's enemy stat records, read from the player's own KH1 files, for `damage = kh1` (owner,
//! 2026-11-14: KH1's damage formula; enemies keep KH2's HP; KH2's enemies take their KH1 twin's
//! record where KH1 has the enemy, a stand-in otherwise).
//!
//! Where KH1 keeps them (KH1-in-ReCom `docs/recom-kh1-attack.md` § Damage and reactions and its
//! asset step `kh1-recom-assets::stats`, Checked (decomp + files) there; the code here is a port of
//! its reader): an ordinary Heartless's AI script is part 5 of its own `.mdls`; at init it hands a
//! 0x6C-byte record of its script data to AI syscall 45 (`0x1402BBDF0` → `0x1402A58E0`): r0 HP,
//! r1 / r2 Strength / Defence as indices into btltbl `0xC8A8` (21 rows of {Strength, Defence}),
//! r3 / r4 the damage floor / cap per hit (u16), r5 level, r7 EXP, r10..r25 resistances (%). The
//! script picks its first-visit (LV1) or LV2 record by the story bit, and the Shadow by world; the
//! offsets below were found there by evaluating each script for every world and both values.

use kh1_data::{Kh1Install, Source};

#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    /// The model the record came from, and its script offset (for the log).
    pub model: &'static str,
    pub offset: u32,
    pub hp: i32,
    pub strength: i32,
    pub defence: i32,
    pub floor: u16,
    pub cap: u16,
    pub level: i32,
    /// %, by KH1 element (1 physical, 2 fire, 3 blizzard, 4 thunder, 8 Gravity, 9 Stop, …).
    pub resist: [i32; 16],
}

/// KH1's ordinary enemies: model and the offsets of every record its script can pick (KH1-in-
/// ReCom's `TWINS`, Checked (files) there; "assumed" pairings there are about Re:CoM's classes,
/// not these records).
pub const MODELS: &[(&str, &[u32])] = &[
    ("xa_ex_2020.mdls", &[0x14, 0xEC, 0x158, 0x1C4, 0x230, 0x29C, 0x3E0, 0x44C, 0x590, 0x5FC, 0x6D4, 0x740]), // Shadow (by world)
    ("xa_ex_2010.mdls", &[0x14, 0x80]),  // Soldier
    ("xa_ex_2050.mdls", &[0x14, 0x80]),  // Large Body
    ("xa_ex_2110.mdls", &[0x14, 0x80]),  // Red Nocturne
    ("xa_ex_2120.mdls", &[0x14, 0x80]),  // Blue Rhapsody
    ("xa_ex_2130.mdls", &[0x14, 0x80]),  // Yellow Opera
    ("xa_ex_2140.mdls", &[0x14, 0x80]),  // Green Requiem
    ("xa_ex_2030.mdls", &[0x14, 0x80]),  // Powerwild
    ("xa_ex_2040.mdls", &[0x14, 0x80]),  // Bouncywild
    ("xa_ex_2160.mdls", &[0x14, 0x80]),  // Air Soldier
    ("xa_ex_2090.mdls", &[0x14, 0x80]),  // Bandit
    ("xa_ex_2060.mdls", &[0x14, 0x80]),  // Fat Bandit
    ("xa_ex_2230.mdls", &[0x14, 0x80]),  // Search Ghost
    ("xa_ex_2100.mdls", &[0x14, 0x80]),  // Pirate
    ("xa_ex_2210.mdls", &[0x14, 0x80]),  // Air Pirate
    ("xa_ex_2200.mdls", &[0x14, 0x80]),  // Wight Knight
    ("xa_ex_2220.mdls", &[0x14, 0x80]),  // Gargoyle
    ("xa_ex_2280.mdls", &[0x14, 0x80]),  // Darkball
    ("xa_ex_2340.mdls", &[0x14, 0x80]),  // Defender
    ("xa_ex_2150.mdls", &[0x14, 0x80]),  // Wizard
    ("xa_ex_2320.mdls", &[0x14, 0x80]),  // Wyvern
    ("xa_ex_2420.mdls", &[0x14]),        // Neoshadow
];

/// KH2's enemies that KH1 has (KH2's enemy id, `00battle.bin` `enmp` row `+0` = objentry `+0x4C`,
/// and the KH1 model). Checked (files): KH2's `M_EX010` / `M_EX020` / `M_EX050` carry ids 1000 /
/// 1001 / 1002 (`00objentry.bin`). Assumption: those three are KH2's Shadow, Soldier and Large Body
/// (the same model numbers KH1's `xa_ex_2020` / `2010` / `2050` play the parts of). More twins wait
/// for KH2's enemy names to be read from its files.
pub const KH2_TWINS: &[(u16, &str)] = &[(1000, "xa_ex_2020.mdls"), (1001, "xa_ex_2010.mdls"), (1002, "xa_ex_2050.mdls")];

/// Part `k` of a `.mdls` (an i32 count, then count + 1 offsets).
pub fn mdls_part(d: &[u8], k: usize) -> Option<&[u8]> {
    let n = i32::from_le_bytes(d.get(0..4)?.try_into().ok()?) as usize;
    if !(k + 1..=64).contains(&n) {
        return None;
    }
    let off = |i: usize| -> Option<usize> { Some(i32::from_le_bytes(d.get(4 + 4 * i..8 + 4 * i)?.try_into().ok()?) as usize) };
    let (a, b) = (off(k)?, off(k + 1)?);
    d.get(a..b)
}

/// One record of a script (`btltbl` for the Strength / Defence table).
pub fn record(model: &'static str, script: &[u8], at: u32, btltbl: &[u8]) -> Option<Record> {
    let o = at as usize;
    let r = |i: usize| -> Option<i32> { Some(i32::from_le_bytes(script.get(o + 4 * i..o + 4 * i + 4)?.try_into().ok()?)) };
    let idx = |i: usize| -> Option<(u8, u8)> {
        let v = r(i)?;
        if !(0..=20).contains(&v) {
            return None;
        }
        let t = 0xC8A8 + 2 * v as usize;
        Some((*btltbl.get(t)?, *btltbl.get(t + 1)?))
    };
    let hp = r(0)?;
    if hp <= 0 {
        return None;
    }
    let mut resist = [0; 16];
    for (k, x) in resist.iter_mut().enumerate() {
        *x = r(10 + k)?;
    }
    Some(Record { model, offset: at, hp, strength: idx(1)?.0 as i32, defence: idx(2)?.1 as i32, floor: (r(3)? & 0xFFFF) as u16, cap: (r(4)? & 0xFFFF) as u16, level: r(5)?, resist })
}

/// A record that deals a fixed damage (floor ≥ cap): not a stand-in.
pub fn fixed_damage(r: &Record) -> bool {
    r.cap != 0 && r.floor >= r.cap
}

/// Every record of `MODELS` that reads, and the notes for the log.
pub fn read_all(kh1: &Kh1Install) -> (Vec<Record>, Vec<String>) {
    let mut notes = Vec::new();
    let Ok(btltbl) = kh1.read(Source::Original, "btltbl.bin") else {
        notes.push("btltbl.bin not read: no KH1 records".into());
        return (Vec::new(), notes);
    };
    let mut out = Vec::new();
    for (model, offsets) in MODELS {
        let Ok(d) = kh1.read(Source::Original, model) else {
            notes.push(format!("{model}: not read"));
            continue;
        };
        let Some(script) = mdls_part(&d, 5) else {
            notes.push(format!("{model}: no AI script"));
            continue;
        };
        for &at in *offsets {
            match record(model, script, at, &btltbl) {
                Some(r) => out.push(r),
                None => notes.push(format!("{model}: no record at {at:#x}")),
            }
        }
    }
    (out, notes)
}

/// KH1's damage curve (btltbl `+0`: a u16 count, then the values; Checked (files): 42 values, 1 …
/// 300), from the player's `btltbl.bin`.
pub fn curve(kh1: &Kh1Install) -> Option<Vec<u16>> {
    let b = kh1.read(Source::Original, "btltbl.bin").ok()?;
    let n = u16::from_le_bytes(b.get(0..2)?.try_into().ok()?) as usize;
    (0..n).map(|i| Some(u16::from_le_bytes(b.get(2 + 2 * i..4 + 2 * i)?.try_into().ok()?))).collect()
}

/// The record a KH2 enemy takes: its twin's records (the one nearest its level) where KH1 has it,
/// else the stand-in: the ordinary record nearest its level (fixed-damage ones left out; ties: the
/// lower Defence, then the model's name), with KH1's ordinary 100 % resistances.
pub fn pick(records: &[Record], kh2_enemy: u16, level: i32) -> Option<Record> {
    let near = |set: Vec<&Record>| -> Option<Record> {
        set.into_iter().min_by(|a, b| (a.level - level).abs().cmp(&(b.level - level).abs()).then(a.defence.cmp(&b.defence)).then(a.model.cmp(b.model)).then(a.offset.cmp(&b.offset))).cloned()
    };
    if let Some((_, model)) = KH2_TWINS.iter().find(|t| t.0 == kh2_enemy) {
        if let Some(r) = near(records.iter().filter(|r| r.model == *model).collect()) {
            return Some(r);
        }
    }
    let mut r = near(records.iter().filter(|r| !fixed_damage(r)).collect())?;
    r.resist = [100; 16];
    Some(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(model: &'static str, level: i32, defence: i32) -> Record {
        Record { model, offset: 0x14, hp: 10, strength: 5, defence, floor: 1, cap: 300, level, resist: [50; 16] }
    }

    #[test]
    fn twins_and_stand_ins() {
        let all = vec![rec("xa_ex_2020.mdls", 2, 2), rec("xa_ex_2020.mdls", 30, 20), rec("xa_ex_2110.mdls", 10, 8), rec("xa_ex_2130.mdls", 10, 6), Record { floor: 10, cap: 10, ..rec("xa_ex_2350.mdls", 11, 1) }];
        // A Shadow takes the Shadow record nearest its level, with its own resistances.
        let s = pick(&all, 1000, 25).unwrap();
        assert_eq!((s.level, s.resist[1]), (30, 50));
        // Anything else: the nearest ordinary record, the lower Defence on a tie, 100 %.
        let o = pick(&all, 1234, 11).unwrap();
        assert_eq!((o.model, o.defence, o.resist[1]), ("xa_ex_2130.mdls", 6, 100));
    }
}

/// Every record and KH1's curve, from the collection at `root` (the folder with both games'
/// executables), and notes for the log.
pub fn load(root: impl AsRef<std::path::Path>) -> (Vec<Record>, Option<Vec<u16>>, Vec<String>) {
    match Kh1Install::locate(root.as_ref()) {
        Ok(kh1) => {
            let (r, n) = read_all(&kh1);
            (r, curve(&kh1), n)
        }
        Err(e) => (Vec::new(), None, vec![format!("KH1's files not found: {e}")]),
    }
}
