//! KH1's combo modifiers in KH2's own combo picker (`docs/abilities.md` t93). Pure: Sora's `ptya`
//! set rewritten in KH1's row order ([`kh1_set`]) and KH1's row tests ([`valid`]), which
//! `win::modifiers` puts into KH2's row checks for our rows only.
//!
//! KH2's picker (Steam, Checked (decomp)): `0x140402D20` builds a pick context (`0x1404028D0`:
//! `+0` the target object, `+8` its part (radius `+0xC`, centre `+0x20`), `+0x30` the aim (toward
//! the target, else Sora's facing), `+0x40` Sora, `+0x48` the set, `+0x50` Sora's position, `+0x60`
//! the valid mask); for every enemy part it tests the kind-2 rows (`0x1404029F0(ctx, part, 2)`),
//! then the target's kind-1 rows (`(ctx, ctx + 8, 1)`), then the kind-3 rows by
//! `0x140402750(row, Sora)` alone (ability bit `+0x40`, air bit, finisher stage); kind 1 / 2 rows
//! also by `0x1404027F0(row, Sora, gap, dy, dot, part)`: `near +0x20 ≤ gap < far +0x24`, `high
//! +0x2C ∓ r < dy ≤ low +0x28 ± r`, `inner_min +0x30 < dot ≤ inner_max +0x34`, where gap = the
//! flat distance less the part's radius, dy = the part's centre's height less Sora's (y down, as
//! KH1), dot = the flat direction · the aim. `0x140402C50` takes the first valid row in set order
//! whose motion isn't playing. KH1's pickers (`0x1402A6EE0`, `0x1402A6C70`) take the lowest flagged
//! row whose request isn't playing: the same rule, so KH1's row order is the set order.
//!
//! KH1's ground picker (Checked (decomp), KH1-in-ReCom's `kh1_core::combo`): at the finisher press
//! one draw r: Stun Impact, Gravity Break, Zantetsuken (those equipped, in that order) each take
//! the next 0.3 of it; none → the finisher row, and Blitz if no side enemy was found; Ripple Drive
//! beside it. With a target (gap, h = its height above Sora): h ≤ 160 or h ≥ reach + 150 or gap ≥
//! 300 → (gap ≥ 300 or gap ≤ 180 → Sliding Dash if equipped, gap ≥ 300 and h < 160, else the
//! scan; otherwise the lunge, with Vortex); else the jump-up attack (Aerial Sweep). The scan
//! (also with no target): a part with wide < dot ≤ narrow and gap < 180 is a side enemy (Vortex,
//! row 11). Air: the finisher and Hurricane Blast with a target no more than 100 above.

/// Our rows (KH1's), by the KH1 ability they need.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    AerialSweep,
    StunImpact,
    GravityBreak,
    Zantetsuken,
    Blitz,
    RippleDrive,
    HurricaneBlast,
    SlidingDash,
    VortexTarget,
    VortexSide,
    Counterattack,
}

/// KH1's combo numbers (`kh1_core::params::Params::kh1_defaults`, Checked (decomp) there).
pub const CLOSE: f32 = 180.0;
pub const LUNGE: f32 = 300.0;
pub const JUMP_UP_MIN: f32 = 160.0;
pub const JUMP_UP_REACH_EXTRA: f32 = 150.0;
pub const AIR_TARGET_ABOVE: f32 = 100.0;
pub const FINISHER_SLICE: f32 = 0.3;

/// KH1's cone cosines: the half-angle in whole degrees (45 → 22, 240 → 120).
pub fn narrow_cos() -> f32 {
    (22.0f32 / 180.0 * std::f32::consts::PI).cos()
}
pub fn wide_cos() -> f32 {
    (120.0f32 / 180.0 * std::f32::consts::PI).cos()
}

/// The target as KH1's picker sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    pub gap: f32,
    /// Its centre's height above Sora (KH1's h = −dy).
    pub h: f32,
}

/// What KH1's ground picker does with a target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Branch {
    Scan,
    SlidingDash,
    Lunge,
    JumpUp,
}

pub fn branch(t: Option<Target>, reach: f32, sliding_dash: bool) -> Branch {
    let Some(t) = t else { return Branch::Scan };
    if t.h <= JUMP_UP_MIN || reach + JUMP_UP_REACH_EXTRA <= t.h || LUNGE <= t.gap {
        if LUNGE <= t.gap || t.gap <= CLOSE {
            if !sliding_dash || JUMP_UP_MIN <= t.h || t.gap < LUNGE {
                Branch::Scan
            } else {
                Branch::SlidingDash
            }
        } else {
            Branch::Lunge
        }
    } else {
        Branch::JumpUp
    }
}

/// One pick, as KH1 would see it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pick {
    pub target: Option<Target>,
    pub reach: f32,
    /// The draw for the finisher slices.
    pub r: f32,
    /// Equipped: Stun Impact, Gravity Break, Zantetsuken, Sliding Dash.
    pub stun: bool,
    pub gravity: bool,
    pub zantetsuken: bool,
    pub sliding_dash: bool,
    /// A side enemy found by the scan (any part with wide < dot ≤ narrow and gap < 180).
    pub side: bool,
}

/// The finisher slice the draw lands in, if any.
pub fn slice(p: &Pick) -> Option<Row> {
    let mut r = p.r;
    for (on, row) in [(p.stun, Row::StunImpact), (p.gravity, Row::GravityBreak), (p.zantetsuken, Row::Zantetsuken)] {
        if !on {
            continue;
        }
        r -= FINISHER_SLICE;
        if r < 0.0 {
            return Some(row);
        }
    }
    None
}

/// Whether a part is a side enemy for KH1's scan.
pub fn side_part(gap: f32, dot: f32) -> bool {
    dot <= narrow_cos() && wide_cos() < dot && gap < CLOSE
}

/// KH1's test for our row beyond what KH2 already checks (ability equipped, air bit, finisher
/// stage). `part`: the part's (gap, dot) for a kind-1 / kind-2 row's test; `dy` for the air
/// finisher comes from the pick's target.
pub fn valid(row: Row, p: &Pick, part: Option<(f32, f32)>) -> bool {
    match row {
        Row::StunImpact | Row::GravityBreak | Row::Zantetsuken => slice(p) == Some(row),
        Row::Blitz => slice(p).is_none() && !(p.side && branch(p.target, p.reach, p.sliding_dash) == Branch::Scan),
        Row::RippleDrive | Row::Counterattack => true,
        Row::HurricaneBlast => p.target.is_some_and(|t| t.h <= AIR_TARGET_ABOVE),
        Row::SlidingDash => branch(p.target, p.reach, p.sliding_dash) == Branch::SlidingDash,
        Row::VortexTarget => branch(p.target, p.reach, p.sliding_dash) == Branch::Lunge,
        Row::VortexSide => branch(p.target, p.reach, p.sliding_dash) == Branch::Scan && part.is_some_and(|(g, d)| side_part(g, d)),
        Row::AerialSweep => branch(p.target, p.reach, p.sliding_dash) == Branch::JumpUp,
    }
}

pub const PTYA_ROW: usize = 0x44;

/// Our rows: (row, KH1 ability (its pair index in `kh1_assets::abilities::PAIRS`), KH2 template
/// row id, our id, motion). Templates (Sora's set, Checked (files)): 0 KH2's Aerial Sweep (kind 1,
/// jump approach), 27 Guard Break (kind 3, finisher), 42 Explosion (kind 3, finisher), 8 the plain
/// air finisher (kind 3, air + finisher), 28 Slide Dash (kind 1), 44 / 34 Dodge Slash (kind 1 /
/// 2), 39 Counterguard (kind 0, taken by id `0x27` after a guard: `0x1403A76B0`). Motions: the
/// pairs' slots, holding KH1's (`kh1_assets::MODIFIERS`).
pub const ROWS: [(Row, usize, u8, u8, u16); 11] = [
    (Row::AerialSweep, 6, 33, 33, 191),
    (Row::StunImpact, 10, 27, 60, 165),
    (Row::GravityBreak, 11, 27, 61, 167),
    (Row::Zantetsuken, 12, 27, 62, 172),
    (Row::Blitz, 8, 27, 63, 169),
    (Row::RippleDrive, 9, 42, 64, 166),
    (Row::HurricaneBlast, 7, 8, 65, 192),
    (Row::SlidingDash, 4, 28, 28, 164),
    (Row::VortexTarget, 5, 44, 44, 163),
    (Row::VortexSide, 5, 34, 34, 163),
    (Row::Counterattack, 13, 39, 39, 171),
];

/// Which of our rows an ability number (the row's `+0x40`) and id stand for.
pub fn row_of(number: u16, id: u8) -> Option<Row> {
    ROWS.iter().find(|r| kh1_assets::abilities::number(r.1) == number && r.3 == id).map(|r| r.0)
}

fn rows(set: &[u8]) -> Option<Vec<&[u8]>> {
    let n = u32::from_le_bytes(set.get(0..4)?.try_into().ok()?) as usize;
    (0..n).map(|i| set.get(4 + i * PTYA_ROW..4 + (i + 1) * PTYA_ROW)).collect()
}

/// Sora's set (`u32 n`, then n rows) in KH1's order, the same size: our rows first where KH1
/// tests them first, KH2's plain rows (ability 0) and Guard in KH2's order, the kind-0 rows, then
/// KH2's ability rows (none can be valid at `abilities = kh1`: their numbers are set by no item
/// then) to keep the count. `None` if a template is missing.
pub fn kh1_set(set: &[u8]) -> Option<Vec<u8>> {
    let all = rows(set)?;
    let by_id = |id: u8| all.iter().find(|r| r[0] == id).copied();
    let ours = |row: Row| -> Option<Vec<u8>> {
        let &(_, k, template, id, motion) = ROWS.iter().find(|r| r.0 == row)?;
        let mut r = by_id(template)?.to_vec();
        r[0] = id;
        r[2] = 0xFF; // no row of its own to fall back on
        r[8..10].copy_from_slice(&motion.to_le_bytes());
        r[0x40..0x42].copy_from_slice(&kh1_assets::abilities::number(k).to_le_bytes());
        if row != Row::Counterattack && row != Row::AerialSweep && row != Row::HurricaneBlast {
            // KH1's test decides (`valid`); KH2's windows open.
            for (o, v) in [(0x20, -1.0e5f32), (0x24, 1.0e5), (0x28, 1.0e5), (0x2C, -1.0e5), (0x30, -2.0), (0x34, 2.0)] {
                r[o..o + 4].copy_from_slice(&v.to_le_bytes());
            }
            r[0x18..0x20].copy_from_slice(&[0; 8]);
        }
        Some(r)
    };
    let plain = |r: &&[u8]| u16::from_le_bytes([r[0x40], r[0x41]]) == 0;
    let mut out: Vec<Vec<u8>> = Vec::new();
    out.push(ours(Row::AerialSweep)?);
    for row in [Row::StunImpact, Row::GravityBreak, Row::Zantetsuken, Row::Blitz, Row::RippleDrive, Row::HurricaneBlast] {
        out.push(ours(row)?);
    }
    let mut placed_ground = false;
    for r in &all {
        let kind = r[1];
        if kind == 0 {
            continue;
        }
        if plain(r) {
            // Our ground approaches go before KH2's first plain kind 1 / 2 ground row that isn't a
            // finisher (Sora's id 6 / 4: KH1's 13 / 14 come after its 9 / 10).
            let air = u32::from_le_bytes(r[4..8].try_into().unwrap()) & 1 != 0;
            let finisher = u32::from_le_bytes(r[4..8].try_into().unwrap()) & 4 != 0;
            if !placed_ground && !air && !finisher && (kind == 1 || kind == 2 || kind == 3) {
                for row in [Row::SlidingDash, Row::VortexTarget, Row::VortexSide] {
                    out.push(ours(row)?);
                }
                placed_ground = true;
            }
            out.push(r.to_vec());
        }
    }
    if !placed_ground {
        return None;
    }
    for r in &all {
        if r[1] == 0 {
            out.push(if r[0] == 39 { ours(Row::Counterattack)? } else { r.to_vec() });
        }
    }
    // Fill to KH2's count with its dead ability rows.
    for r in &all {
        if out.len() >= all.len() {
            break;
        }
        if r[1] != 0 && !plain(r) && !out.iter().any(|o| o[0] == r[0]) {
            out.push(r.to_vec());
        }
    }
    if out.len() != all.len() {
        return None;
    }
    let mut v = set[..4].to_vec();
    for r in out {
        v.extend_from_slice(&r);
    }
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pick() -> Pick {
        Pick { target: None, reach: 190.0, r: 0.5, stun: false, gravity: false, zantetsuken: false, sliding_dash: true, side: false }
    }

    /// The cases of KH1-in-ReCom's `kh1_core::combo` tests.
    #[test]
    fn kh1s_branches() {
        let t = |gap, h| Some(Target { gap, h });
        assert_eq!(branch(None, 190.0, true), Branch::Scan);
        assert_eq!(branch(t(330.0 - 30.0, 50.0), 190.0, true), Branch::SlidingDash);
        assert_eq!(branch(t(300.0, 50.0), 190.0, false), Branch::Scan);
        assert_eq!(branch(t(470.0, 160.0), 190.0, true), Branch::Scan);
        assert_eq!(branch(t(250.0, 50.0), 190.0, true), Branch::Lunge);
        assert_eq!(branch(t(100.0, 250.0), 190.0, true), Branch::JumpUp);
        assert_eq!(branch(t(100.0, 50.0), 190.0, true), Branch::Scan);
        assert_eq!(branch(t(100.0, 340.0), 190.0, true), Branch::Scan);
    }

    #[test]
    fn finisher_slices_in_kh1s_order() {
        let mut p = pick();
        p.stun = true;
        p.zantetsuken = true;
        p.r = 0.1;
        assert!(valid(Row::StunImpact, &p, None) && !valid(Row::Zantetsuken, &p, None) && !valid(Row::Blitz, &p, None));
        p.r = 0.45;
        assert!(valid(Row::Zantetsuken, &p, None) && !valid(Row::StunImpact, &p, None));
        p.r = 0.7;
        assert!(slice(&p).is_none() && valid(Row::Blitz, &p, None));
        p.side = true;
        assert!(!valid(Row::Blitz, &p, None));
        // A target in the lunge band: no scan, so no side enemy for Blitz.
        p.target = Some(Target { gap: 250.0, h: 50.0 });
        assert!(valid(Row::Blitz, &p, None));
    }

    #[test]
    fn side_cone() {
        assert!(side_part(100.0, 0.5));
        assert!(!side_part(100.0, 0.95));
        assert!(!side_part(100.0, -0.6));
        assert!(!side_part(200.0, 0.5));
        let mut p = pick();
        assert!(valid(Row::VortexSide, &p, Some((100.0, 0.5))));
        p.target = Some(Target { gap: 250.0, h: 50.0 });
        assert!(!valid(Row::VortexSide, &p, Some((100.0, 0.5))));
        assert!(valid(Row::VortexTarget, &p, None));
        assert!(!valid(Row::HurricaneBlast, &Pick { target: Some(Target { gap: 50.0, h: 150.0 }), ..p }, None));
        assert!(valid(Row::HurricaneBlast, &Pick { target: Some(Target { gap: 50.0, h: 60.0 }), ..p }, None));
    }

    /// With the player's `00battle.bin` (`KH2_BATTLE`): Sora's set rewritten, same size, every
    /// KH2 plain row kept in KH2's order, ours where KH1 tests them.
    #[test]
    fn players_set() {
        let Ok(p) = std::env::var("KH2_BATTLE") else { return };
        let b = std::fs::read(p).unwrap();
        let bar = kh2_formats::bar::Bar::parse(&b).unwrap();
        let t = bar.entries.iter().find(|e| e.name == "ptya").unwrap().data;
        let off = u32::from_le_bytes(t[8 + 4..8 + 8].try_into().unwrap()) as usize;
        let n = u32::from_le_bytes(t[off..off + 4].try_into().unwrap()) as usize;
        let set = &t[off..off + 4 + n * PTYA_ROW];
        let out = kh1_set(set).unwrap();
        assert_eq!(out.len(), set.len());
        let rs = rows(&out).unwrap();
        let ids: Vec<u8> = rs.iter().map(|r| r[0]).collect();
        eprintln!("ids {ids:?}");
        let pos = |id: u8| ids.iter().position(|&x| x == id).unwrap();
        // KH1's order: Aerial Sweep before the jump approaches; the finisher modifiers before the
        // plain finishers; Hurricane Blast before the air finishers; Sliding Dash and Vortex before
        // the plain ground swings.
        assert!(pos(33) < pos(0));
        for id in [60, 61, 62, 63, 64] {
            assert!(pos(id) < pos(2) && pos(id) < pos(3));
        }
        assert!(pos(65) < pos(7) && pos(65) < pos(8));
        assert!(pos(28) < pos(44) && pos(44) < pos(34) && pos(34) < pos(6) && pos(34) < pos(4));
        // Plain rows keep KH2's relative order.
        let plain: Vec<u8> = rows(set).unwrap().iter().filter(|r| r[1] != 0 && r[0x40] == 0 && r[0x41] == 0).map(|r| r[0]).collect();
        let kept: Vec<u8> = ids.iter().copied().filter(|i| plain.contains(i)).collect();
        assert_eq!(plain, kept);
        // Counterattack on Counterguard's id, our number, KH1's motion slot.
        let c = rs.iter().find(|r| r[0] == 39).unwrap();
        assert_eq!(u16::from_le_bytes([c[0x40], c[0x41]]), kh1_assets::abilities::number(13));
        assert_eq!(u16::from_le_bytes([c[8], c[9]]), 171);
        for r in &rs {
            let num = u16::from_le_bytes([r[0x40], r[0x41]]);
            if num >= 200 {
                assert!(row_of(num, r[0]).is_some(), "row {}", r[0]);
            }
        }
    }
}
