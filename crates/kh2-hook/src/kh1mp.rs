//! KH1's MP on KH2's MP fields (`mp = kh1`; the owner, 2026-11-13: KH1's bars, KH1's refill from
//! hits, KH1's level gains plus KH2's MP-up rewards). Pure logic; `win/mp.rs` puts it in KH2.
//!
//! KH1 (the sister project's `kh1-kh2-mp.md` §3, Checked (decomp) there): MP is whole bars and a
//! yellow gauge of units; the next bar needs `(bars + 1) × 30` units (`0x1402A4CB0`), the gauge
//! holds at most `max × 30` at full MP (`0x1402A4AE0`). A cost of 100 and up is `cost / 100` whole
//! bars; under 100 it is gauge units, a bar broken into `bars × 30` units when the gauge runs out
//! (`0x1402A5B60`); the check (`0x1402A4C10`): bars `cost / 100 <= bars`, units `cost <= gauge +
//! 30 × bars (bars + 1) / 2`.
//!
//! On KH2 (Checked (decomp) `0x1403C1D90`): max MP is the save record's byte `+7`, so a bar is 10 of
//! KH2's MP (25 bars at most); the yellow gauge's share of the next bar is the last digit, and the
//! gauge itself is kept in the status block's MP Charge time `+0x1BC` (unused: KH1 has no MP Charge
//! and its length `+0x1C0`, which KH2 reads as "charging", stays 0).

/// KH2 MP a KH1 bar is worth.
pub const MP_PER_BAR: i32 = 10;
/// KH1's units a bar needs per bar already held.
pub const UNITS_PER_STEP: i32 = 30;
/// KH2's cost byte for "all MP" (Cure, the Limits).
pub const COST_ALL: i32 = 255;

/// KH1's MP: whole bars and gauge units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mp {
    pub bars: i32,
    pub units: i32,
}

impl Mp {
    /// From KH2's MP and the kept gauge (clamped to what the bar shown allows).
    pub fn from_kh2(mp: i32, units: i32, max_bars: i32) -> Mp {
        let bars = (mp / MP_PER_BAR).clamp(0, max_bars);
        Mp { bars, units: units.clamp(0, cap(bars, max_bars)) }
    }

    /// KH2's MP shown: the bars and the gauge's share of the next one.
    pub fn kh2(self, max_bars: i32) -> i32 {
        if self.bars >= max_bars {
            return max_bars * MP_PER_BAR;
        }
        let need = (self.bars + 1) * UNITS_PER_STEP;
        self.bars * MP_PER_BAR + (self.units * MP_PER_BAR / need).min(MP_PER_BAR - 1)
    }

    /// KH1's gauge add (`0x1402A4AE0` and the bar step `0x1402A4CB0`).
    pub fn add_units(mut self, n: i32, max_bars: i32) -> Mp {
        self.units = (self.units + n).max(0);
        while self.bars < max_bars && self.units >= (self.bars + 1) * UNITS_PER_STEP {
            self.units -= (self.bars + 1) * UNITS_PER_STEP;
            self.bars += 1;
        }
        self.units = self.units.min(cap(self.bars, max_bars));
        self
    }

    /// A gain KH2 gives in its own MP (items such as Ether, scripts): 10 a bar, so whole bars,
    /// and the rest as that share of the next bar's gauge.
    pub fn add_kh2(mut self, mp: i32, max_bars: i32) -> Mp {
        if mp <= 0 {
            return self;
        }
        self.bars = (self.bars + mp / MP_PER_BAR).min(max_bars);
        let rest = mp % MP_PER_BAR;
        let need = (self.bars + 1) * UNITS_PER_STEP;
        let units = self.units;
        Mp { bars: self.bars, units: 0 }.add_units(units + rest * need / MP_PER_BAR, max_bars)
    }

    /// KH1's check (`0x1402A4C10`), with KH2's "all" (at least one bar, the owner's Limits rule).
    pub fn can_pay(self, cost: i32) -> bool {
        if cost <= 0 {
            true
        } else if cost == COST_ALL {
            self.bars >= 1
        } else if cost >= 100 {
            cost / 100 <= self.bars
        } else {
            cost <= self.units + UNITS_PER_STEP * self.bars * (self.bars + 1) / 2
        }
    }

    /// KH1's payment (`0x1402A5B60`); `None` if he can't pay.
    pub fn pay(mut self, cost: i32) -> Option<Mp> {
        if !self.can_pay(cost) {
            return None;
        }
        if cost == COST_ALL {
            return Some(Mp { bars: 0, units: 0 });
        }
        if cost >= 100 {
            self.bars -= cost / 100;
            return Some(self);
        }
        self.units -= cost.max(0);
        while self.units < 0 && self.bars > 0 {
            self.units += self.bars * UNITS_PER_STEP;
            self.bars -= 1;
        }
        self.units = self.units.max(0);
        Some(self)
    }
}

/// The gauge's most: the next bar's need, or at full MP `max × 30` (KH1's cap).
fn cap(bars: i32, max_bars: i32) -> i32 {
    if bars >= max_bars { max_bars * UNITS_PER_STEP } else { (bars + 1) * UNITS_PER_STEP - 1 }
}

/// KH1's starting bars (Assumption: 2, not yet read from KH1's new-game record; the owner asked).
pub const KH1_START_BARS: i32 = 2;
/// KH1's level-ups that give +1 bar (Checked (files): Sora's level table, `btltbl.bin` `+0x3AC0`).
pub const KH1_MP_LEVELS: [u8; 5] = [12, 24, 36, 48, 60];
/// KH2's MP-up rewards' running totals in reward order (Checked (files): `00battle.bin` `bons`,
/// character 1: rewards 23, 26, 29 +10, 51 +40, 68 +10). Assumption: he gets them in that order.
pub const KH2_MP_UP_TOTALS: [i32; 5] = [10, 20, 30, 70, 80];
/// KH2 Sora's starting max MP (record `+7` before any reward).
pub const KH2_BASE_MAX_MP: i32 = 100;

/// Max bars: KH1's start, KH1's level gains, one per KH2 MP-up reward (the owner's choice).
pub fn max_bars(level: u8, kh2_max_mp: i32) -> i32 {
    let levels = KH1_MP_LEVELS.iter().filter(|&&l| level >= l).count() as i32;
    let ups = KH2_MP_UP_TOTALS.iter().filter(|&&t| kh2_max_mp - KH2_BASE_MAX_MP >= t).count() as i32;
    (KH1_START_BARS + levels + ups).min(255 / MP_PER_BAR)
}

/// KH1's costs on KH2's magic commands (`cmd` rows with execute 2, by argument: 0 Fire, 1
/// Blizzard, 2 Thunder, 3 Cure), in KH1's hundredths (Checked (files): `btltbl.bin` attack data
/// `+0x30`, every level: Fire 30, Blizzard 30, Thunder 100, Cure 100; also Gravity 100, Stop 200,
/// Aero 200). Magnet (4) and Reflect (5) keep their KH2 bytes (30, 10), read as KH1 units.
pub const KH1_SPELL_COSTS: [(i16, u8); 4] = [(0, 30), (1, 30), (2, 100), (3, 100)];

/// KH1's MP for a hit Sora lands (attack data `+0x2C`, Checked (files): his plain swings 3, his
/// finishers 6, the spells 0), added as gauge units (`0x1402BF330`). KH1 scales it by the two levels
/// (× (1 + difference × 20 %), within 1 unit and 250 %); KH2's enemies have no KH1 level, so ×1
/// (Assumption, not 1:1 against an enemy of another level).
pub const KH1_HIT_UNITS: i32 = 3;
pub const KH1_FINISHER_UNITS: i32 = 6;

/// KH1's MP from a hit (`0x1402BF330`): the attack's units × (1 + level difference × 20 %
/// (`btltbl` `+0x94`), + 150 % with MP Haste (`+0x95`)), at least 1, at most 250 % (`+0x96`).
/// Checked (decomp). KH2's enemies have no KH1 level: difference 0 (Assumption). `haste` is how
/// many steps of KH2's MP Haste (1), Hastera (2) and Hastega (3) are equipped: they stack (the
/// owner, 2026-11-13), each step another 150 % and KH1's cap 150 % higher.
pub fn hit_units(base: i32, haste: i32) -> i32 {
    let haste = haste.max(0);
    let f = 1.0 + 1.5 * haste as f32;
    let cap = base * (250 + 150 * (haste - 1).max(0)) / 100;
    ((base as f32 * f) as i32).max(1).min(cap.max(1))
}

/// KH1's MP orbs (`0x1402AC080`, prize kinds 5 / 6 → the gauge add `0x1402A4AE0`): small 2 units,
/// large 15. Checked (decomp).
pub const KH1_ORB_SMALL: i32 = 2;
pub const KH1_ORB_LARGE: i32 = 15;

/// KH1's MP Rage (`0x14029BBD0`): the gauge gains the HP lost × the copies equipped. Checked (decomp).
pub fn rage_units(hp_lost: i32, copies: i32) -> i32 {
    (hp_lost.max(0) as f32 * copies.max(0) as f32) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gauge_and_bars() {
        let m = Mp { bars: 2, units: 0 };
        // The third bar needs 90 units.
        let m = m.add_units(89, 5);
        assert_eq!(m, Mp { bars: 2, units: 89 });
        assert_eq!(m.kh2(5), 29);
        let m = m.add_units(1, 5);
        assert_eq!(m, Mp { bars: 3, units: 0 });
        assert_eq!(m.kh2(5), 30);
        // Full: the gauge holds up to max × 30.
        let f = Mp { bars: 5, units: 0 }.add_units(1000, 5);
        assert_eq!(f, Mp { bars: 5, units: 150 });
        assert_eq!(f.kh2(5), 50);
    }

    #[test]
    fn kh1_payment() {
        let m = Mp { bars: 2, units: 10 };
        // Fire: 30 units; the gauge has 10, so bar 2 (60 units) is broken.
        assert_eq!(m.pay(30), Some(Mp { bars: 1, units: 40 }));
        assert_eq!(m.pay(100), Some(Mp { bars: 1, units: 10 }));
        assert_eq!(m.pay(300), None);
        // Units: 10 + 30 × 2 × 3 / 2 = 100 at most.
        assert!(m.can_pay(100) && !Mp { bars: 0, units: 29 }.can_pay(30));
        assert_eq!(m.pay(COST_ALL), Some(Mp { bars: 0, units: 0 }));
        assert_eq!(Mp { bars: 0, units: 20 }.pay(COST_ALL), None);
    }

    #[test]
    fn haste_steps() {
        assert_eq!(hit_units(3, 0), 3);
        assert_eq!(hit_units(6, 0), 6);
        // KH1's own MP Haste: 3 × 2.5 = 7 (its cap 7), finisher 15.
        assert_eq!(hit_units(3, 1), 7);
        assert_eq!(hit_units(6, 1), 15);
        // Stacked: 3 × 4 = 12 under the cap of 400 %.
        assert_eq!(hit_units(3, 2), 12);
        assert_eq!(hit_units(3, 6), 30);
    }

    #[test]
    fn kh2_gains_and_rage() {
        // Ether's 30 KH2 MP: three bars.
        assert_eq!(Mp { bars: 1, units: 5 }.add_kh2(30, 9), Mp { bars: 4, units: 5 });
        assert_eq!(Mp { bars: 8, units: 0 }.add_kh2(30, 9), Mp { bars: 9, units: 0 });
        // 5 KH2 MP: half of the next bar (bar 3 needs 90 units).
        assert_eq!(Mp { bars: 2, units: 10 }.add_kh2(5, 9), Mp { bars: 2, units: 55 });
        assert_eq!(rage_units(12, 2), 24);
        assert_eq!(rage_units(-3, 2), 0);
    }

    #[test]
    fn max_bars_grow() {
        assert_eq!(max_bars(1, 100), 2);
        assert_eq!(max_bars(12, 100), 3);
        assert_eq!(max_bars(60, 180), 2 + 5 + 5);
        assert_eq!(max_bars(30, 130), 2 + 2 + 3);
        assert_eq!(max_bars(30, 140), 2 + 2 + 3);
        assert_eq!(max_bars(30, 170), 2 + 2 + 4);
    }
}
