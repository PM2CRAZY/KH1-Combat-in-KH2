//! Adds sounds of another SEDBSSCF bank (KH1's system bank `se000`) to a KH2 bank (KH2's `se000`)
//! as new sound ids, so KH2's own sound engine plays them by `bank × 1000 + index` like its own.
//!
//! Layout (Checked (files): KH2's `se/se000.win32.scd`, version 4, and KH1's `amusic/se000.dat`
//! HD asset `se000.win32.scd`, version 3; the same engine reads both, the sister project's
//! `docs/kh1-sound-native.md`):
//!
//! ```text
//! 0x00  "SEDBSSCF", u32 version, ..., u32 file size at 0x10 (0x30 bytes)
//! 0x30  u16 sounds, u16 tracks, u16 audio streams, u16 bank number
//! 0x38  u32 track table, u32 audio table, u32 layout table (one per sound), u32 routing (0),
//!       u32 attribute table, u32 0
//! 0x50  the sound table, then the others, each 16-aligned; then the blocks
//! sound  byte 0 track count, byte 1 bus, f32 volume at 0x08, u32 id (its own index) at 0x0C; its
//!        last 4 × count bytes (u16 track, u16 audio) pairs. KH2's v4 records are 0x40 bytes before
//!        the pairs (a name at 0x30); KH1's v3 ones 0x10.
//! track  a command script; layout 0x80 bytes; audio a 0x20-byte header, extra, data
//! ```
//!
//! The KH2 bank's blocks keep their bytes and order (moved down by the larger tables: no block
//! holds a file offset, as `scd_cut` relies on too); the added sounds follow, each a KH2 record
//! made on a template of KH2's own (one of its one-track sounds) with the other bank's volume, bus
//! and pairs, its layout, its tracks and its audio as they are.

use crate::{bad, Error, Result};

fn u16_at(b: &[u8], o: usize) -> Result<u16> {
    b.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]])).ok_or_else(|| Error::Format("scd merge: short".into()))
}

fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    b.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]])).ok_or_else(|| Error::Format("scd merge: short".into()))
}

fn align16(v: &mut Vec<u8>) {
    while v.len() % 16 != 0 {
        v.push(0);
    }
}

struct Bank<'a> {
    data: &'a [u8],
    sounds: Vec<usize>,
    tracks: Vec<usize>,
    audio: Vec<usize>,
    layouts: Vec<usize>,
    t_attr: usize,
    first: usize,
}

impl<'a> Bank<'a> {
    fn parse(data: &'a [u8]) -> Result<Bank<'a>> {
        if data.get(0..8) != Some(b"SEDBSSCF") {
            return bad("scd merge: not SEDBSSCF");
        }
        let (ns, nt, na) = (u16_at(data, 0x30)? as usize, u16_at(data, 0x32)? as usize, u16_at(data, 0x34)? as usize);
        let (t_track, t_audio, t_layout, t_routing, t_attr) = (u32_at(data, 0x38)? as usize, u32_at(data, 0x3C)? as usize, u32_at(data, 0x40)? as usize, u32_at(data, 0x44)?, u32_at(data, 0x48)? as usize);
        if t_routing != 0 {
            return bad("scd merge: a routing table");
        }
        let table = |o: usize, n: usize| -> Result<Vec<usize>> { (0..n).map(|i| Ok(u32_at(data, o + 4 * i)? as usize)).collect() };
        let sounds = table(0x50, ns)?;
        let tracks = table(t_track, nt)?;
        let audio = table(t_audio, na)?;
        let layouts = if t_layout != 0 { table(t_layout, ns)? } else { vec![0; ns] };
        let first = sounds.iter().chain(&tracks).chain(&audio).chain(&layouts).copied().filter(|&o| o != 0).min().unwrap_or(data.len());
        Ok(Bank { data, sounds, tracks, audio, layouts, t_attr, first })
    }

    /// The block at `o`, up to the next block start.
    fn block(&self, o: usize) -> &'a [u8] {
        let end = self.sounds.iter().chain(&self.tracks).chain(&self.audio).chain(&self.layouts).copied().filter(|&s| s > o).min().unwrap_or(self.data.len());
        &self.data[o..end]
    }

    fn audio_block(&self, a: usize) -> Result<&'a [u8]> {
        let o = *self.audio.get(a).ok_or_else(|| Error::Format(format!("scd merge: no audio {a}")))?;
        let len = 0x20 + u32_at(self.data, o + 0x18)? as usize + u32_at(self.data, o)? as usize;
        self.data.get(o..o + len).ok_or_else(|| Error::Format(format!("scd merge: audio {a} runs past the bank")))
    }

    /// (count, bus, volume, pairs) of sound `i`; the pairs are the record's last 4 × count bytes.
    fn sound(&self, i: usize) -> Result<(u8, u8, [u8; 4], Vec<(u16, u16)>)> {
        let o = *self.sounds.get(i).ok_or_else(|| Error::Format(format!("scd merge: no sound {i}")))?;
        let rec = self.block(o);
        let count = *rec.first().ok_or_else(|| Error::Format("scd merge: empty record".into()))? as usize;
        let at = rec.len().checked_sub(4 * count).filter(|&a| a >= 0x10).ok_or_else(|| Error::Format(format!("scd merge: sound {i} short")))?;
        // A record may end in padding before the next block: its pairs are right after its fixed
        // part (KH1's v3: 0x10), so read them there when the bank is v3.
        let at = if u32_at(self.data, 8)? <= 3 { 0x10 } else { at };
        let pairs = (0..count).map(|k| Ok((u16_at(rec, at + 4 * k)?, u16_at(rec, at + 4 * k + 2)?))).collect::<Result<Vec<_>>>()?;
        Ok((count as u8, rec[1], rec[8..12].try_into().unwrap(), pairs))
    }
}

/// `kh2` with the sounds `ids` of `other` added; returns the bank and each added sound's new index
/// (in `ids`' order). A sound of `other` that plays nothing gets `None`.
pub fn merge(kh2: &[u8], other: &[u8], ids: &[usize]) -> Result<(Vec<u8>, Vec<Option<usize>>)> {
    let a = Bank::parse(kh2)?;
    let b = Bank::parse(other)?;
    // KH2's template record: its first one-track sound's fixed part (0x40 bytes).
    let template = a
        .sounds
        .iter()
        .map(|&o| a.block(o))
        .find(|r| r.first() == Some(&1) && r.len() >= 0x44)
        .map(|r| r[..0x40].to_vec())
        .ok_or_else(|| Error::Format("scd merge: no one-track sound in the KH2 bank".into()))?;
    struct New {
        record: Vec<u8>,
        layout: Option<Vec<u8>>,
        tracks: Vec<Vec<u8>>,
        audio: Vec<Vec<u8>>,
    }
    let mut added: Vec<New> = Vec::new();
    let mut index = Vec::new();
    let (mut nt, mut na) = (a.tracks.len(), a.audio.len());
    for &id in ids {
        let (count, bus, vol, pairs) = b.sound(id)?;
        if count == 0 {
            index.push(None);
            continue;
        }
        let new_id = a.sounds.len() + added.len();
        let mut record = template.clone();
        record[0] = count;
        record[1] = bus;
        record[8..12].copy_from_slice(&vol);
        record[0x0C..0x10].copy_from_slice(&(new_id as u32).to_le_bytes());
        // KH2 keeps the sound's number as text at 0x30 (Checked (files): "114").
        record[0x30..0x40].fill(0);
        let name = new_id.to_string();
        record[0x30..0x30 + name.len()].copy_from_slice(name.as_bytes());
        let (mut tracks, mut audio) = (Vec::new(), Vec::new());
        for (t, au) in pairs {
            let tb = b.tracks.get(t as usize).ok_or_else(|| Error::Format(format!("scd merge: no track {t}")))?;
            tracks.push(b.block(*tb).to_vec());
            audio.push(b.audio_block(au as usize)?.to_vec());
            record.extend_from_slice(&((nt + tracks.len() - 1) as u16).to_le_bytes());
            record.extend_from_slice(&((na + audio.len() - 1) as u16).to_le_bytes());
        }
        nt += tracks.len();
        na += audio.len();
        let layout = match b.layouts.get(id) {
            Some(&o) if o != 0 => {
                let blk = b.block(o);
                let size = u16_at(other, o)? as usize;
                Some(blk[..size.clamp(4, blk.len())].to_vec())
            }
            _ => None,
        };
        added.push(New { record, layout, tracks, audio });
        index.push(Some(new_id));
    }
    let ns = a.sounds.len() + added.len();
    if ns > 999 || nt > 0xFFFF || na > 0xFFFF {
        return bad("scd merge: too many sounds for one bank");
    }
    // The tables, then KH2's blocks moved by `delta`, then ours.
    let mut out = kh2[..0x50].to_vec();
    let place = |out: &mut Vec<u8>, n: usize| -> usize {
        let at = out.len();
        out.extend(std::iter::repeat(0u8).take(4 * n.max(1)));
        align16(out);
        at
    };
    let tab_sound = place(&mut out, ns);
    debug_assert_eq!(tab_sound, 0x50);
    let tab_track = place(&mut out, nt);
    let tab_audio = place(&mut out, na);
    let tab_layout = place(&mut out, ns);
    // KH2's attribute table (empty in its effect banks: Checked (files)) kept as it was.
    let attr_len = a.first.saturating_sub(a.t_attr).max(0);
    let tab_attr = out.len();
    out.extend_from_slice(&kh2[a.t_attr..a.t_attr + attr_len]);
    align16(&mut out);
    let delta = out.len() as i64 - a.first as i64;
    out.extend_from_slice(&kh2[a.first..]);
    align16(&mut out);
    let set = |out: &mut Vec<u8>, at: usize, v: usize| out[at..at + 4].copy_from_slice(&(v as u32).to_le_bytes());
    let moved = |o: usize| if o == 0 { 0 } else { (o as i64 + delta) as usize };
    for (i, &o) in a.sounds.iter().enumerate() {
        set(&mut out, tab_sound + 4 * i, moved(o));
    }
    for (i, &o) in a.tracks.iter().enumerate() {
        set(&mut out, tab_track + 4 * i, moved(o));
    }
    for (i, &o) in a.audio.iter().enumerate() {
        set(&mut out, tab_audio + 4 * i, moved(o));
    }
    for (i, &o) in a.layouts.iter().enumerate() {
        set(&mut out, tab_layout + 4 * i, moved(o));
    }
    // KH2 packs its records one after another (68 bytes for one track: Checked (files)); ours too,
    // then the layouts, tracks and audio, each 16-aligned.
    for (k, n) in added.iter().enumerate() {
        let at = out.len();
        out.extend_from_slice(&n.record);
        set(&mut out, tab_sound + 4 * (a.sounds.len() + k), at);
    }
    align16(&mut out);
    let (mut ti, mut ai) = (a.tracks.len(), a.audio.len());
    for (k, n) in added.iter().enumerate() {
        if let Some(l) = &n.layout {
            let at = out.len();
            out.extend_from_slice(l);
            align16(&mut out);
            set(&mut out, tab_layout + 4 * (a.sounds.len() + k), at);
        }
        for t in &n.tracks {
            let at = out.len();
            out.extend_from_slice(t);
            align16(&mut out);
            set(&mut out, tab_track + 4 * ti, at);
            ti += 1;
        }
        for au in &n.audio {
            let at = out.len();
            out.extend_from_slice(au);
            align16(&mut out);
            set(&mut out, tab_audio + 4 * ai, at);
            ai += 1;
        }
    }
    let size = out.len();
    out[0x10..0x14].copy_from_slice(&(size as u32).to_le_bytes());
    out[0x30..0x32].copy_from_slice(&(ns as u16).to_le_bytes());
    out[0x32..0x34].copy_from_slice(&(nt as u16).to_le_bytes());
    out[0x34..0x36].copy_from_slice(&(na as u16).to_le_bytes());
    set(&mut out, 0x38, tab_track);
    set(&mut out, 0x3C, tab_audio);
    set(&mut out, 0x40, tab_layout);
    set(&mut out, 0x44, 0);
    set(&mut out, 0x48, tab_attr);
    Ok((out, index))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With the player's banks (`KH2_SE000`, `KH1_SE000`: the two `se000.win32.scd`): every KH2
    /// sound keeps its record and audio bytes, and each added KH1 sound's record points at a copy
    /// of KH1's own audio and track, with KH1's volume and bus.
    #[test]
    fn players_banks() {
        let (Ok(k2), Ok(k1)) = (std::env::var("KH2_SE000"), std::env::var("KH1_SE000")) else { return };
        let (k2, k1) = (std::fs::read(k2).unwrap(), std::fs::read(k1).unwrap());
        let ids: Vec<usize> = (95..=175).collect();
        let (m, idx) = merge(&k2, &k1, &ids).unwrap();
        let (a, b, c) = (Bank::parse(&k2).unwrap(), Bank::parse(&k1).unwrap(), Bank::parse(&m).unwrap());
        assert_eq!(c.sounds.len(), a.sounds.len() + idx.iter().flatten().count());
        assert_eq!(u32_at(&m, 0x10).unwrap() as usize, m.len());
        for i in 0..a.sounds.len() {
            let (x, y) = (a.sound(i).unwrap(), c.sound(i).unwrap());
            assert_eq!(x, y, "KH2 sound {i}");
            for (t, au) in x.3 {
                assert_eq!(a.audio_block(au as usize).unwrap(), c.audio_block(au as usize).unwrap());
                assert_eq!(a.block(a.tracks[t as usize]), &c.block(c.tracks[t as usize])[..a.block(a.tracks[t as usize]).len()]);
            }
        }
        for (k, &id) in ids.iter().enumerate() {
            let Some(n) = idx[k] else { continue };
            let (x, y) = (b.sound(id).unwrap(), c.sound(n).unwrap());
            assert_eq!((x.0, x.1, x.2), (y.0, y.1, y.2), "KH1 sound {id} as {n}");
            assert_eq!(u32_at(&m, c.sounds[n] + 0x0C).unwrap() as usize, n);
            for ((t1, a1), (t2, a2)) in x.3.iter().zip(&y.3) {
                assert_eq!(b.audio_block(*a1 as usize).unwrap(), c.audio_block(*a2 as usize).unwrap(), "KH1 sound {id}");
                let tb = b.block(b.tracks[*t1 as usize]);
                assert_eq!(tb, &c.block(c.tracks[*t2 as usize])[..tb.len()]);
            }
        }
    }
}
