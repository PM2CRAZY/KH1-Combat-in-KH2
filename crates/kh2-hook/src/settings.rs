//! `kh2-kh1.ini` in the game folder: one KH2 / KH1 toggle per system (all KH2 by default, which is
//! vanilla KH2). Read once at start. Parsing is plain Rust so it is tested on any platform; the
//! file itself is read and written by the Windows side.

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
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// One per `SYSTEMS` entry, same order.
    pub styles: Vec<Style>,
    /// The debug trace (`trace = on`): F10 starts and stops a recording of Sora each frame.
    pub trace: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { styles: vec![Style::Kh2; SYSTEMS.len()], trace: false }
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
        !(SYSTEMS.iter().all(|(k, _)| has(k)) && has("trace"))
    }

    /// The ini as written on first launch (and the layout `parse` reads).
    pub fn render(&self) -> String {
        let mut out = String::from(
            "# KH1-Combat-in-KH2 settings. Each system plays like kh2 (vanilla) or kh1.\n\
             # Read when the game starts.\n\n",
        );
        for ((k, what), st) in SYSTEMS.iter().zip(&self.styles) {
            out.push_str(&format!("# {what}\n{k} = {}\n\n", st.name()));
        }
        out.push_str(&format!(
            "# Debug: F10 starts / stops a trace of Sora each frame (kh2-kh1-trace-N.csv)\ntrace = {}\n",
            if self.trace { "on" } else { "off" }
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(!Settings::missing_rows(&s.render()));
        assert!(Settings::missing_rows("movement = kh1\n"));
        let p = Settings::parse("Combos = KH1 ; note\nmovement=kh3\nnope = kh1\n# camera = kh1");
        assert_eq!(p.style("combos"), Style::Kh1);
        assert_eq!(p.style("movement"), Style::Kh2);
        assert_eq!(p.style("camera"), Style::Kh2);
    }
}
