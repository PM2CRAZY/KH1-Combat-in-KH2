//! `kh2-kh1.ini` in the game folder: one KH2 / KH1 toggle per system (all KH2 by default, which is
//! vanilla KH2). Read at start; the Config menu's mod page changes a system at once ([`set_kh1`],
//! read by the systems through [`kh1`]) and writes the file. Parsing is plain Rust so it is tested
//! on any platform; the file itself is read and written by the Windows side.

use std::sync::atomic::{AtomicBool, Ordering};

/// `SYSTEMS` indices, for [`kh1`].
pub const MOVEMENT: usize = 0;
pub const COMBOS: usize = 1;
pub const DEFENCE: usize = 2;
pub const MP: usize = 3;
pub const MAGIC: usize = 4;
pub const ABILITIES: usize = 5;
pub const CAMERA: usize = 6;
pub const MODEL: usize = 7;
pub const DAMAGE: usize = 8;

static LIVE: [AtomicBool; 9] = [const { AtomicBool::new(false) }; 9];

/// Whether system `i` (`SYSTEMS` order) plays like KH1 now.
#[inline]
pub fn kh1(i: usize) -> bool {
    LIVE.get(i).is_some_and(|b| b.load(Ordering::Relaxed))
}

/// Sets system `i` now; returns whether it changed.
pub fn set_kh1(i: usize, on: bool) -> bool {
    LIVE.get(i).is_some_and(|b| b.swap(on, Ordering::Relaxed) != on)
}

/// Every system's switch from `s` (at start).
pub fn set_all(s: &Settings) {
    for (i, st) in s.styles.iter().enumerate() {
        set_kh1(i, *st == Style::Kh1);
    }
}

/// The systems whose change waits for the next launch: Sora's model (Panacea keeps the HD
/// texture list of the model it first loads, so another model in the same run would get the
/// first one's textures).
pub const NEXT_LAUNCH: &[usize] = &[MODEL];

/// Which game a system plays like.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    Kh2,
    Kh1,
}

impl Style {
    pub fn name(self) -> &'static str {
        match self {
            Style::Kh2 => "kh2",
            Style::Kh1 => "kh1",
        }
    }
}

/// The systems that get a toggle, in the roadmap's order (`docs/ROADMAP.md`). A system's row
/// does nothing until its version lands; the row is here so the ini's layout stays stable.
pub const SYSTEMS: &[(&str, &str)] = &[
    ("movement", "Run, walk, jump, High Jump, air control, fall"),
    ("combos", "Ground and air combos, finishers, combo abilities"),
    ("defence", "Guard, Dodge Roll, hit reactions, knockback, i-frames"),
    ("mp", "The MP bar, MP gained by landing hits"),
    ("magic", "Fire, Blizzard, Thunder, Cure, Gravity, Stop, Aero"),
    ("abilities", "Sonic Blade, Ars Arcanum, Strike Raid, Ragnarok and the rest"),
    ("camera", "Field camera and lock-on"),
    ("model", "Sora's player model: KH1 Sora's own model, skeleton and textures"),
    ("damage", "KH1's damage formula for Sora's swings and spells (enemies keep KH2's HP)"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// One per `SYSTEMS` entry, same order.
    pub styles: Vec<Style>,
    /// The debug trace (`trace = on`): F10 starts and stops a recording of Sora each frame.
    pub trace: bool,
    /// Debug (`new_spells = 0..3`): Sora has KH1's new spells (Gravity so far) at that level,
    /// without their unlocks; 0 off.
    pub new_spells: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { styles: vec![Style::Kh2; SYSTEMS.len()], trace: false, new_spells: 0 }
    }
}

impl Settings {
    /// The style of a system by its key (`SYSTEMS`); KH2 for an unknown key.
    pub fn style(&self, key: &str) -> Style {
        SYSTEMS.iter().position(|(k, _)| *k == key).map_or(Style::Kh2, |i| self.styles[i])
    }

    /// Reads `key = kh1` / `key = kh2` lines; `#` and `;` start comments; anything else (unknown
    /// keys, bad values) is ignored and keeps the default.
    pub fn parse(text: &str) -> Settings {
        let mut s = Settings::default();
        for line in text.lines() {
            let line = line.split(['#', ';']).next().unwrap_or("").trim();
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim().to_ascii_lowercase(), v.trim().to_ascii_lowercase());
            if k == "trace" {
                s.trace = v == "on";
                continue;
            }
            if k == "new_spells" {
                s.new_spells = v.parse::<u8>().unwrap_or(0).min(3);
                continue;
            }
            let Some(i) = SYSTEMS.iter().position(|(key, _)| *key == k) else { continue };
            match v.as_str() {
                "kh1" => s.styles[i] = Style::Kh1,
                "kh2" => s.styles[i] = Style::Kh2,
                _ => {}
            }
        }
        s
    }

    /// Whether `text` lacks a row this version knows (an ini from an older version): then it is
    /// written again with the values it had and the new rows at their defaults.
    pub fn missing_rows(text: &str) -> bool {
        let has = |key: &str| {
            text.lines().any(|l| l.trim_start().to_ascii_lowercase().strip_prefix(key).is_some_and(|r| r.trim_start().starts_with('=')))
        };
        !(SYSTEMS.iter().all(|(k, _)| has(k)) && has("trace") && has("new_spells"))
    }

    /// The ini as written on first launch (and the layout `parse` reads).
    pub fn render(&self) -> String {
        let mut out = String::from(
            "# KH1-Combat-in-KH2 settings. Each system plays like kh2 (vanilla) or kh1.\n\
             # The Config menu's mod page (L1 / R1) changes them in game at once (the player model at\n\
             # the next launch).\n\n",
        );
        for ((k, what), st) in SYSTEMS.iter().zip(&self.styles) {
            out.push_str(&format!("# {what}\n{k} = {}\n\n", st.name()));
        }
        out.push_str(&format!(
            "# Debug: F10 starts / stops a trace of Sora each frame (kh2-kh1-trace-N.csv)\ntrace = {}\n",
            if self.trace { "on" } else { "off" }
        ));
        out.push_str(&format!(
            "\n# Debug: Sora has KH1's new spells (Gravity so far) at this level, 1 to 3, without\n# their unlocks (0 off; needs magic = kh1)\nnew_spells = {}\n",
            self.new_spells
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indices_follow_systems() {
        for (i, k) in [(MOVEMENT, "movement"), (COMBOS, "combos"), (DEFENCE, "defence"), (MP, "mp"), (MAGIC, "magic"), (ABILITIES, "abilities"), (CAMERA, "camera"), (MODEL, "model")] {
            assert_eq!(SYSTEMS[i].0, k);
        }
        assert_eq!(SYSTEMS.len(), LIVE.len());
        assert!(set_kh1(CAMERA, true) && kh1(CAMERA) && !set_kh1(CAMERA, true) && set_kh1(CAMERA, false) && !kh1(CAMERA));
    }

    #[test]
    fn default_is_all_kh2() {
        assert!(Settings::default().styles.iter().all(|s| *s == Style::Kh2));
    }

    #[test]
    fn round_trip_and_bad_lines() {
        let mut s = Settings::default();
        s.styles[1] = Style::Kh1;
        assert_eq!(Settings::parse(&s.render()), s);
        s.trace = true;
        assert_eq!(Settings::parse(&s.render()), s);
        s.new_spells = 2;
        assert_eq!(Settings::parse(&s.render()), s);
        assert!(!Settings::missing_rows(&s.render()));
        assert!(Settings::missing_rows("movement = kh1\n"));
        let p = Settings::parse("Combos = KH1 ; note\nmovement=kh3\nnope = kh1\n# camera = kh1");
        assert_eq!(p.style("combos"), Style::Kh1);
        assert_eq!(p.style("movement"), Style::Kh2);
        assert_eq!(p.style("camera"), Style::Kh2);
    }
}
