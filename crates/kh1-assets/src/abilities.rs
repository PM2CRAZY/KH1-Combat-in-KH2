//! KH1's action abilities in KH2's ability list (Phase 5, `docs/abilities.md`). The owner's answer
//! (2026-11-14): at `abilities = kh1` each KH1 ability stands in the slot of the KH2 ability it is
//! paired with; KH2's list comes back at `kh2`. KH2's save keeps 80 ability slots for Sora and fills
//! about 75 by itself, so KH1's abilities take no slots of their own.
//!
//! What a pair changes (the hook, `kh2_hook::ability_rows`): the KH2 item row's name, description,
//! AP cost and ability number (the bit KH2's ability block sets, `0x140401730`, from the row's
//! `+4`). The save keeps KH2's item id, so KH2 gives the KH1 ability exactly where it gives its
//! pair (level, boss bonus, Drive Form level, event) and KH2's events still see the KH2 item.
//!
//! KH1's ids and AP costs: btltbl `0x33D8`, 12 bytes per ability, row id − 1, u16 cost at `+0`.
//! Checked (files, the owner's `btltbl.bin`). Names: English, as KH1 FM's menus write them
//! (Assumption, strong: the Archipelago KH1 world's list, the sister project's `growth.rs`).
//! Descriptions are ours (KH1's own text isn't read yet).

/// One pair: KH2's item, and KH1's ability that stands in it at `abilities = kh1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pair {
    /// KH2's ability item (`03system.bin` `item`, kind `0x13`).
    pub kh2_item: u16,
    /// KH1's ability id (btltbl's ability table, id − 1 = row).
    pub kh1_id: u8,
    /// KH1's AP cost.
    pub ap: u8,
    pub name: &'static str,
    pub description: &'static str,
}

const fn p(kh2_item: u16, kh1_id: u8, ap: u8, name: &'static str, description: &'static str) -> Pair {
    Pair { kh2_item, kh1_id, ap, name, description }
}

/// The first ability number of ours: KH2's last is 198 (`Protectga`), and its ability block keeps
/// 256 bits (`+0` .. `+0x20`, the growth levels start at `+0x20`: `0x1404014A0`). Checked (files +
/// decomp).
pub const FIRST_NUMBER: u16 = 200;

/// The pairs, by role: the specials take KH2's special-purpose combo abilities, each combo modifier
/// the KH2 ability closest to it. Every KH2 item here is Sora's (a boss bonus, `bons`), has no
/// case in KH2's ability switch `0x140401730` (whose effects key on the item id, not the row) and
/// gates no Limit (`limt +0x32`). Checked (files + decomp). The pairing itself is ours (owner
/// answer (a); the pairs are open to change).
pub const PAIRS: [Pair; 14] = [
    p(137, 11, 3, "Sonic Blade", "Dashes at a foe and slashes through it.\nRepeat it to keep dashing. Uses MP."),
    p(561, 13, 3, "Strike Raid", "Throws the Keyblade at a far foe.\nRepeat it to throw again. Uses MP."),
    p(562, 12, 4, "Ars Arcanum", "Unleashes a flurry of slashes on a foe.\nRepeat it to keep going. Uses MP."),
    p(272, 14, 4, "Ragnarok", "Rises with a combo then fires a hail\nof light at foes. Uses MP."),
    p(264, 54, 1, "Sliding Dash", "Closes in on a far foe with a quick\nslash."),
    p(263, 17, 1, "Vortex", "Closes in on a far foe with a spinning\nslash."),
    p(269, 18, 2, "Aerial Sweep", "Leaps at a foe above you with a\nspinning slash."),
    p(270, 55, 2, "Hurricane Blast", "Ends an air combo with a spinning\nflurry."),
    p(559, 20, 2, "Blitz", "Ends a combo with a dash that strikes\nthe foes ahead."),
    p(266, 56, 3, "Ripple Drive", "Ends a combo with a burst that hits\nfoes all around. Stronger with more MP."),
    p(265, 57, 2, "Stun Impact", "Ends a combo with a blow that stuns\nthe foes around you."),
    p(267, 58, 2, "Gravity Break", "Ends a combo with a crushing blow that\ncuts deep into a foes HP."),
    p(273, 59, 2, "Zantetsuken", "Ends a combo with a slash that can\nfell a foe in one blow."),
    p(268, 19, 2, "Counterattack", "Strikes back right after you block\nan attack."),
];

/// KH2's own combo modifiers and finisher abilities left after the pairs (owner, 2026-11-14:
/// "With abilities set to KH1 I don't want to see any KH2 combo modifiers or finishers at all"):
/// at `abilities = kh1` they are off the ability screen, cost no AP and do nothing (their entries
/// stay in the save, so KH2's scripts still see them). KH2 item ids, names from the player's
/// sys.bar. Checked (files).
pub const HIDDEN: [(u16, &str); 8] = [
    (262, "Slapshot"),
    (271, "Horizontal Slash"),
    (560, "Aerial Dive"),
    (390, "Combo Boost"),
    (391, "Air Combo Boost"),
    (393, "Finishing Plus"),
    (394, "Negative Combo"),
    (395, "Berserk Charge"),
];

/// Whether KH2 item `id` is off at `abilities = kh1` ([`HIDDEN`]).
pub fn hidden(id: u16) -> bool {
    HIDDEN.iter().any(|h| h.0 == id)
}

/// The ability number (the ability block's bit) of pair `k`.
pub const fn number(k: usize) -> u16 {
    FIRST_NUMBER + k as u16
}

/// The pair whose KH1 ability is `kh1_id`.
pub fn by_kh1(kh1_id: u8) -> Option<usize> {
    PAIRS.iter().position(|p| p.kh1_id == kh1_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// KH1's AP costs, as btltbl's ability table holds them (`0x33D8 + 12 × (id − 1)`).
    #[test]
    fn kh1_costs() {
        // From the owner's btltbl.bin (Checked (files)): id → cost.
        let costs = [(11, 3), (12, 4), (13, 3), (14, 4), (17, 1), (18, 2), (19, 2), (20, 2), (54, 1), (55, 2), (56, 3), (57, 2), (58, 2), (59, 2)];
        for (id, ap) in costs {
            assert_eq!(PAIRS[by_kh1(id).unwrap()].ap, ap, "KH1 ability {id}");
        }
        if let Ok(path) = std::env::var("KH1_BTLTBL") {
            let b = std::fs::read(path).unwrap();
            for p in PAIRS {
                let o = 0x33D8 + 12 * (p.kh1_id as usize - 1);
                assert_eq!(u16::from_le_bytes([b[o], b[o + 1]]), p.ap as u16, "{}", p.name);
            }
        }
    }

    #[test]
    fn numbers_fit_kh2s_block_and_items_are_distinct() {
        assert!(number(PAIRS.len() - 1) < 256);
        for (i, a) in PAIRS.iter().enumerate() {
            assert!(PAIRS[i + 1..].iter().all(|b| b.kh2_item != a.kh2_item && b.kh1_id != a.kh1_id));
            // KH2's ability switch keys these on the item id: never pair one of them.
            assert!(!matches!(a.kh2_item, 0xA2 | 0xA3 | 0x186..=0x19E | 0x1A3 | 0x1A5 | 0x1A6 | 0x21C | 0x21E));
            assert!(!hidden(a.kh2_item));
        }
    }
}
