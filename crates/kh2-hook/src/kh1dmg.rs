//! KH1's damage for Sora's hits on KH2's enemies (`damage = kh1`; owner, 2026-11-14: KH1's
//! formula, curve, floor / cap and resistances; Sora's Strength is his KH2 Strength; enemies keep
//! KH2's HP; damage taken stays KH2's).
//!
//! KH1's damage function `0x1402BFA80` (Checked (decomp); KH1-in-ReCom's `kh1-core::damage`, a port
//! tested there): the attack's power + its stat (Strength, or max MP × 1 / 2 / 3 / 5 by the data's
//! stat source), a critical roll (chance × 0.05 × the weapon's crit × 0.05, + the bonus); element 8
//! (Gravity): HP × power % × resist, at least the floor (1 under 2); elements 9 / 11 / 12: the power as
//! it is; else KH1's curve at (power − Defence), clamped to the curve, the data's min / max, the
//! record's floor / cap, × the element's resistance (%), at least 1.

/// KH1's attack data, the fields the damage reads (btltbl's attack rows: `+0` stat source, `+4`
/// element, `+0x18` crit, `+0x1C` crit bonus, `+0x48` / `+0x4C` / `+0x50` power, min, max).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Attack {
    pub stat: u8,
    pub element: u8,
    pub power: i32,
    pub min: i32,
    pub max: i32,
    pub crit: i32,
    pub crit_bonus: i32,
}

const fn a(stat: u8, element: u8, power: i32, crit: i32, crit_bonus: i32) -> Attack {
    Attack { stat, element, power, min: 0, max: 0, crit, crit_bonus }
}

/// KH1's rows for base Sora's KH2 attacks (Checked (files): the owner's `btltbl.bin`, read
/// 2026-11-14): plain swings (`0x4B78` / `0x4BE8`), finishers (`0x4C58`), Fire `0x5F28..`, Blizzard
/// `0x6078..`, Thunder `0x61C8..`, Cure `0x6318..`, Gravity `0x6468..`, Aerora / Aeroga's wind
/// `0x5CF8`; min and max are 0 on every one.
pub const SWING: Attack = a(0, 1, 0, 0, 0);
pub const FINISHER: Attack = a(0, 1, 4, 4, 4);
pub const FIRE: [i32; 3] = [20, 28, 36];
pub const BLIZZARD: [i32; 3] = [22, 27, 34];
pub const THUNDER: [i32; 3] = [16, 20, 26];
pub const CURE: [i32; 3] = [15, 27, 36];
pub const GRAVITY: [i32; 3] = [40, 55, 70];
pub const WIND: Attack = a(3, 0, 8, 0, 0);

/// KH1's Kingdom Key's crit (weapon row `+0x31`: 20; Checked (files)). KH2's Keyblades have no KH1
/// row: every one takes the Kingdom Key's (Assumption).
pub const WEAPON_CRIT: i32 = 20;

/// The KH1 data for base Sora's KH2 attack (ATKP id, sub); None: not one of KH1's (KH2's own
/// damage stays).
pub fn sora_attack(id: u16, sub: u8) -> Option<Attack> {
    let level = ((sub % 10) / 2).min(2) as usize;
    if sub >= 10 {
        return None; // a Drive Form's
    }
    Some(match id {
        126 | 127 | 128 | 131 | 132 => SWING,
        129 | 130 | 133 | 134 => FINISHER,
        502 => a(1, 2, FIRE[level], 0, 0),
        248 => a(1, 3, BLIZZARD[level], 0, 0),
        250 => a(3, 4, THUNDER[level], 0, 0),
        708 => a(1, 11, CURE[level], 0, 0),
        2066 => a(5, 8, GRAVITY[level], 0, 0),
        2069 => WIND,
        _ => return None,
    })
}

/// The target's KH1 numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    pub hp: i32,
    pub defence: i32,
    pub floor: u16,
    pub cap: u16,
    /// %, by KH1 element.
    pub resist: [i32; 16],
}

/// KH1's damage. `roll`: KH1's random in [0, 1) for the critical (drawn only when the attack can
/// crit).
pub fn damage(atk: &Attack, strength: i32, max_mp: i32, t: &Target, curve: &[u16], roll: f32) -> i32 {
    let stat = match atk.stat {
        0 => strength,
        1 => max_mp,
        2 => 0,
        3 => max_mp * 2,
        4 => max_mp * 3,
        5 => max_mp * 5,
        _ => strength,
    };
    let mut power = atk.power + stat;
    if power == 0 {
        return 0;
    }
    let chance = atk.crit as f32 * 0.05 * (WEAPON_CRIT as f32 * 0.05);
    if chance > 0.0 && roll < chance {
        power += atk.crit_bonus;
    }
    let element = (atk.element & 0xF) as usize;
    let resist = t.resist[element] as f32 / 100.0;
    match element {
        8 => {
            let floor = if t.floor < 2 { 1 } else { t.floor as i32 };
            let v = (t.hp as f32 * (power as f32 / 100.0) * resist) as i32;
            return v.max(floor);
        }
        9 | 11 | 12 => return power,
        _ => {}
    }
    let n = curve.len() as i32;
    let i = (power - t.defence).clamp(0, (n - 1).max(0));
    let mut d = curve.get(i as usize).copied().unwrap_or(1) as i32;
    if atk.min != 0 {
        d = d.max(atk.min);
    }
    if atk.max != 0 {
        d = d.min(atk.max);
    }
    if t.floor == 0 || t.floor as i32 <= d {
        if t.cap != 0 && (t.cap as i32) < d {
            d = t.cap as i32;
        }
    } else {
        d = t.floor as i32;
    }
    let d = (d as f32 * resist) as i32;
    d.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURVE: [u16; 42] = [1, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 14, 17, 20, 24, 28, 34, 40, 48, 57, 68, 80, 95, 113, 135, 160, 190, 227, 270, 300, 300, 300, 300, 300, 300, 300, 300, 300, 300, 300, 300, 300];

    fn t(defence: i32) -> Target {
        Target { hp: 100, defence, floor: 0, cap: 0, resist: [100; 16] }
    }

    #[test]
    fn kh1s_numbers() {
        // KH1-in-ReCom's examples (Checked there): Fire 20 + max MP 3 − Defence 5 = curve[18] = 48;
        // Firaga 36 + 8 − 5 = 39: the curve's top, 300.
        assert_eq!(damage(&sora_attack(502, 0).unwrap(), 50, 3, &t(5), &CURVE, 0.9), 48);
        assert_eq!(damage(&sora_attack(502, 4).unwrap(), 50, 8, &t(5), &CURVE, 0.9), 300);
        // Fire resistance 50 %.
        let mut r = t(5);
        r.resist[2] = 50;
        assert_eq!(damage(&sora_attack(502, 0).unwrap(), 50, 3, &r, &CURVE, 0.9), 24);
        // A cap of 20.
        r = t(5);
        r.cap = 20;
        assert_eq!(damage(&sora_attack(502, 0).unwrap(), 50, 3, &r, &CURVE, 0.9), 20);
        // A swing: Strength 20 − Defence 8 = curve[12] = 17; a finisher +4: curve[16] = 34, a
        // critical (20 %) another +4: curve[20] = 68.
        assert_eq!(damage(&SWING, 20, 3, &t(8), &CURVE, 0.0), 17);
        assert_eq!(damage(&FINISHER, 20, 3, &t(8), &CURVE, 0.5), 34);
        assert_eq!(damage(&FINISHER, 20, 3, &t(8), &CURVE, 0.1), 68);
        // Thunder: 16 + max MP × 2. Aero's wind: 8 + max MP × 2.
        assert_eq!(damage(&sora_attack(250, 0).unwrap(), 0, 3, &t(0), &CURVE, 0.9), CURVE[22] as i32);
        assert_eq!(damage(&WIND, 0, 3, &t(0), &CURVE, 0.9), CURVE[14] as i32);
        // Cure: the power as it is (15 + max MP).
        assert_eq!(damage(&sora_attack(708, 0).unwrap(), 0, 3, &t(0), &CURVE, 0.9), 18);
        // Gravity: HP × (40 + 5 × 3) %; a floor.
        assert_eq!(damage(&sora_attack(2066, 0).unwrap(), 0, 3, &t(0), &CURVE, 0.9), 55);
        let mut g = t(0);
        g.hp = 1;
        g.floor = 2;
        assert_eq!(damage(&sora_attack(2066, 0).unwrap(), 0, 3, &g, &CURVE, 0.9), 2);
        // Drive Forms and KH2-only attacks keep KH2's.
        assert!(sora_attack(502, 10).is_none() && sora_attack(161, 0).is_none());
    }
}
