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
    fn max_bars_grow() {
        assert_eq!(max_bars(1, 100), 2);
        assert_eq!(max_bars(12, 100), 3);
        assert_eq!(max_bars(60, 180), 2 + 5 + 5);
        assert_eq!(max_bars(30, 130), 2 + 2 + 3);
        assert_eq!(max_bars(30, 140), 2 + 2 + 3);
        assert_eq!(max_bars(30, 170), 2 + 2 + 4);
    }
}
