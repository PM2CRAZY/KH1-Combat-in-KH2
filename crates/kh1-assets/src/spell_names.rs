//! Our names in KH2's own text (`msg/us/sys.bar`), built from the player's file: KH2's messages
//! unchanged, ours added after them (ids past KH2's last, so the table stays sorted). Gravity, Stop
//! and Aero (step 4 of `docs/command-menu-rows.md`) and KH1's abilities (`docs/abilities.md`).
//! Built at every launch whatever the settings (`crate::build_text`), so a switch in the Config
//! menu finds them: KH2 reads this file once, at boot (Assumption).
//!
//! KH2's text (Checked (files): `msg/us/sys.bar`, its `sys` entry): `1`, a count, then (id,
//! offset) pairs sorted by id, then 0-ended strings in KH2's own letters: `A`–`Z` from `0x2E`,
//! `a`–`z` from `0x9A`, space `0x01`, `.` `0x4F`, a new line `0x02`. A command or item names its
//! message as `0x8000 | id`. The magic items' descriptions end with the same line ("Use from the
//! Magic Command.", `0x635`).

/// The first id of ours (KH2's last is `0x562D`).
pub const FIRST: u16 = 0x5700;
/// Command names: element `k` (0 Gravity, 1 Stop, 2 Aero), level `l`: `FIRST + 3k + l`.
pub const COMMANDS: [[&str; 3]; 3] = [["Gravity", "Gravira", "Graviga"], ["Stop", "Stopra", "Stopga"], ["Aero", "Aerora", "Aeroga"]];
/// Item name `FIRST + 9 + 2k`, description `FIRST + 10 + 2k`.
pub const ITEMS: [(&str, &str); 3] = [
    ("Gravity Element", "Crushes foes with gravity. The more HP they have the more it takes."),
    ("Stop Element", "Stops the flow of time around foes for a while."),
    ("Aero Element", "Wraps you in a barrier of wind that softens blows."),
];
/// "\n" then the magic items' last line, as KH2 writes it (`0x635`).
const TAIL: &[u8] = &[
    0x02, 0x0B, 0x5A, 0x42, 0xAC, 0x9E, 0x01, 0x9F, 0xAB, 0xA8, 0xA6, 0x01, 0xAD, 0xA1, 0x9E, 0x01, 0x3A, 0x9A, 0xA0, 0xA2, 0x9C, 0x01, 0x30, 0xA8, 0xA6, 0xA6, 0x9A, 0xA7, 0x9D, 0x4F, 0x01, 0x07, 0xF0, 0xF0,
];

pub fn command_message(k: usize, l: usize) -> u16 {
    FIRST + (3 * k + l) as u16
}
pub fn item_name(k: usize) -> u16 {
    FIRST + 9 + 2 * k as u16
}
pub fn item_description(k: usize) -> u16 {
    FIRST + 10 + 2 * k as u16
}
/// KH1's abilities ([`crate::abilities::PAIRS`]): name `FIRST + 0x20 + 2k`, description `+ 1`.
pub fn ability_name(k: usize) -> u16 {
    FIRST + 0x20 + 2 * k as u16
}
pub fn ability_description(k: usize) -> u16 {
    FIRST + 0x21 + 2 * k as u16
}

/// KH2's letters for plain text (letters, digits, space, `.`, `-`, a new line). Checked (files):
/// KH2's own descriptions (`-` `0x66`, digits from `0x90`, a new line `0x02`).
pub fn encode(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| match c {
            'A'..='Z' => 0x2E + (c as u8 - b'A'),
            'a'..='z' => 0x9A + (c as u8 - b'a'),
            '0'..='9' => 0x90 + (c as u8 - b'0'),
            '.' => 0x4F,
            '-' => 0x66,
            '\n' => 0x02,
            _ => 0x01,
        })
        .collect()
}

/// Every message of ours: (id, bytes without the 0).
pub fn messages() -> Vec<(u16, Vec<u8>)> {
    let mut out = Vec::new();
    for (k, names) in COMMANDS.iter().enumerate() {
        for (l, n) in names.iter().enumerate() {
            out.push((command_message(k, l), encode(n)));
        }
    }
    for (k, (name, text)) in ITEMS.iter().enumerate() {
        out.push((item_name(k), encode(name)));
        let mut d = encode(text);
        d.extend_from_slice(TAIL);
        out.push((item_description(k), d));
    }
    for (k, a) in crate::abilities::PAIRS.iter().enumerate() {
        out.push((ability_name(k), encode(a.name)));
        out.push((ability_description(k), encode(a.description)));
    }
    out
}

fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

/// The `sys` message table with ours added; `None` if it isn't one we know, or already has ours.
pub fn add_to_table(m: &[u8]) -> Option<Vec<u8>> {
    let n = u32_at(m, 4)? as usize;
    if u32_at(m, 0)? != 1 || 8 + 8 * n > m.len() {
        return None;
    }
    let last = u32_at(m, 8 + 8 * (n - 1))?;
    if last >= FIRST as u32 {
        return None;
    }
    let ours = messages();
    let grow = 8 * ours.len();
    let mut out = Vec::with_capacity(m.len() + grow + 0x400);
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&((n + ours.len()) as u32).to_le_bytes());
    for i in 0..n {
        out.extend_from_slice(&u32_at(m, 8 + 8 * i)?.to_le_bytes());
        out.extend_from_slice(&(u32_at(m, 12 + 8 * i)? + grow as u32).to_le_bytes());
    }
    let mut strings = m[8 + 8 * n..].to_vec();
    let base = 8 + 8 * (n + ours.len());
    for (id, text) in &ours {
        out.extend_from_slice(&(*id as u32).to_le_bytes());
        out.extend_from_slice(&((base + strings.len()) as u32).to_le_bytes());
        strings.extend_from_slice(text);
        strings.push(0);
    }
    out.extend_from_slice(&strings);
    Some(out)
}

/// `msg/us/sys.bar` with ours added to its `sys` entry (the BAR's other entries as they were).
pub fn add_to_bar(bar: &[u8]) -> Option<Vec<u8>> {
    if bar.get(0..4)? != b"BAR\x01" {
        return None;
    }
    let n = u32_at(bar, 4)? as usize;
    let mut entries = Vec::new();
    for i in 0..n {
        let e = 16 + 16 * i;
        let head = bar.get(e..e + 8)?.to_vec();
        let (off, size) = (u32_at(bar, e + 8)? as usize, u32_at(bar, e + 12)? as usize);
        let data = bar.get(off..off + size)?;
        let data = if &head[4..8] == b"sys\0" { add_to_table(data)? } else { data.to_vec() };
        entries.push((head, data));
    }
    let mut out = bar[..16].to_vec();
    let mut at = 16 + 16 * n;
    let mut body = Vec::new();
    for (head, data) in &entries {
        out.extend_from_slice(head);
        out.extend_from_slice(&(at as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        body.extend_from_slice(data);
        at += data.len();
        while at % 16 != 0 {
            body.push(0);
            at += 1;
        }
    }
    out.extend_from_slice(&body);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_are_kh2s() {
        // KH2's "Fire" (0x508) and "Blizzard" (0x50E).
        assert_eq!(encode("Fire"), vec![0x33, 0xA2, 0xAB, 0x9E]);
        assert_eq!(encode("Blizzard"), vec![0x2F, 0xA5, 0xA2, 0xB3, 0xB3, 0x9A, 0xAB, 0x9D]);
    }

    #[test]
    fn table_keeps_kh2s_and_adds_ours_sorted() {
        // A table of two: 0x508 "Fire", 0x509 "Fira".
        let mut m = vec![];
        m.extend(1u32.to_le_bytes());
        m.extend(2u32.to_le_bytes());
        m.extend(0x508u32.to_le_bytes());
        m.extend(24u32.to_le_bytes());
        m.extend(0x509u32.to_le_bytes());
        m.extend(29u32.to_le_bytes());
        m.extend([0x33, 0xA2, 0xAB, 0x9E, 0, 0x33, 0xA2, 0xAB, 0x9A, 0]);
        let t = add_to_table(&m).unwrap();
        let n = u32_at(&t, 4).unwrap() as usize;
        assert_eq!(n, 2 + 15 + 2 * crate::abilities::PAIRS.len());
        let get = |id: u32| {
            let i = (0..n).find(|&i| u32_at(&t, 8 + 8 * i) == Some(id)).unwrap();
            let o = u32_at(&t, 12 + 8 * i).unwrap() as usize;
            t[o..o + t[o..].iter().position(|&b| b == 0).unwrap()].to_vec()
        };
        assert_eq!(get(0x508), encode("Fire"));
        assert_eq!(get(0x509), encode("Fira"));
        assert_eq!(get(0x5700), encode("Gravity"));
        assert_eq!(get(0x5708), encode("Aeroga"));
        assert!(get(0x570A).ends_with(TAIL));
        assert_eq!(get(0x5720), encode("Sonic Blade"));
        assert_eq!(get(ability_description(13) as u32), encode(crate::abilities::PAIRS[13].description));
        let ids: Vec<u32> = (0..n).map(|i| u32_at(&t, 8 + 8 * i).unwrap()).collect();
        assert!(ids.windows(2).all(|w| w[0] < w[1]));
        assert!(add_to_table(&t).is_none());
    }

    /// With the player's file (`KH2_SYS_BAR`): KH2's messages read back unchanged.
    #[test]
    fn players_file() {
        let Ok(p) = std::env::var("KH2_SYS_BAR") else { return };
        let bar = std::fs::read(p).unwrap();
        let out = add_to_bar(&bar).unwrap();
        let e = |b: &[u8]| (u32_at(b, 24).unwrap() as usize, u32_at(b, 28).unwrap() as usize);
        let ((o1, s1), (o2, s2)) = (e(&bar), e(&out));
        let (a, b) = (&bar[o1..o1 + s1], &out[o2..o2 + s2]);
        let (na, nb) = (u32_at(a, 4).unwrap() as usize, u32_at(b, 4).unwrap() as usize);
        assert_eq!(nb, na + messages().len());
        for i in 0..na {
            let (oa, ob) = (u32_at(a, 12 + 8 * i).unwrap() as usize, u32_at(b, 12 + 8 * i).unwrap() as usize);
            let za = a[oa..].iter().position(|&x| x == 0).unwrap();
            assert_eq!(&a[oa..oa + za], &b[ob..ob + za]);
        }
    }
}
