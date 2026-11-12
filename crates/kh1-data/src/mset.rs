//! KH1 motion sets (`.mset`): the motion table and plain forward evaluation of a motion's
//! keyed channels, enough to pose a motion on its skeleton (for telling motions apart, and
//! for knowing which one the game is playing).
//!
//! Layout, from the user's `xa_ex_0010.mset` and the executable's skeleton pass
//! (`0x140391E20`, which reads the motion header fields below, and `0x14038F9D0`, which
//! evaluates one channel):
//! - the file: `count u32`, then section offsets; section 0 is `MMTN`, a motion table:
//!   `"MMTN"`, size, ..., and at `+0x40` motion offsets (from `+0x40`), ended by `0xFFFFFFFF`.
//! - a motion (offsets from its own start): `+0x04` frame count u32, `+0x08` frames per second
//!   f32, `+0x10` bone count, `+0x18` channel count, `+0x1C` channels, `+0x20` / `+0x24` a
//!   second channel list (IK helpers; not evaluated here), `+0x28` keys, `+0x2C` / `+0x30` the
//!   helpers.
//! - a channel (6 bytes): `bone u16`, `kind u8` (low nibble: 1-3 scale x/y/z, 4-6 rotation,
//!   7-9 translation; bits 4-5 / 6-7 before / after the keys: 2 repeats), `count u8`,
//!   `first key u16`.
//! - a key (16 bytes): `time << 8 | interpolation` as i32 (time in 1/256 frames; interpolation
//!   0 holds the value, 1 is linear, else cubic Hermite), value, out tangent, in tangent (per
//!   frame). Between keys `k` and `k+1`: Hermite with `k`'s value and its tangent at `+0xC`,
//!   `k+1`'s value and its tangent at `+0x8`, tangents scaled by the gap.

use kh2_formats::anim::Bone;
use kh2_formats::{Error, Result};

const MMTN: u32 = u32::from_le_bytes(*b"MMTN");

fn u32_at(b: &[u8], at: usize) -> Result<u32> {
    b.get(at..at + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap())).ok_or_else(|| Error::Format(format!("mset: read past the end at {at:#x}")))
}

#[derive(Clone, Copy, Debug)]
pub struct Key {
    pub time: f32,
    pub interpolation: u8,
    pub value: f32,
    pub tangent_out: f32,
    pub tangent_in: f32,
}

#[derive(Clone, Debug)]
pub struct Channel {
    pub bone: u16,
    /// 1-3 scale, 4-6 rotation, 7-9 translation (x, y, z).
    pub kind: u8,
    pub before: u8,
    pub after: u8,
    pub keys: Vec<Key>,
}

#[derive(Clone, Debug)]
pub struct Motion {
    /// Offset of the motion in the motion table section (what the game's pointer is relative
    /// to, once the section is loaded).
    pub offset: usize,
    pub frames: u32,
    pub fps: f32,
    pub bones: u32,
    pub channels: Vec<Channel>,
    /// The IK helpers' channel count (`+0x20`) and helper count (`+0x2C`).
    pub helper_channels: u32,
    pub helpers: u32,
    /// The rest of KH1's skeleton pass (`crate::rig`, KH1-Combat-in-KH2 t11; Checked (decomp):
    /// KH1 `0x14038F550`, `0x1403900B0`): one joint word a bone (`+0x14`), the helper table
    /// (`+0x30`, 40 bytes a helper), the helper channels (`+0x24`, same layout as `channels`, the
    /// bone being the helper), the constant channels (`+0x38`, `+0x34` of them: bone u16, kind u16,
    /// value f32).
    pub joint_words: Vec<u32>,
    pub helper_table: Vec<HelperRest>,
    pub helper_chans: Vec<Channel>,
    pub constants: Vec<(u16, u16, f32)>,
}

/// One IK helper's rest values (40 bytes: scale `+0`, parent `+0xC`, rotation `+0x10`,
/// translation `+0x1C`).
#[derive(Clone, Copy, Debug)]
pub struct HelperRest {
    pub scale: [f32; 3],
    pub parent: i32,
    pub rotation: [f32; 3],
    pub translation: [f32; 3],
}

/// The motions of a `.mset`, in table order (`None` where a slot is empty or unreadable).
pub fn parse(mset: &[u8]) -> Result<Vec<Option<Motion>>> {
    let count = u32_at(mset, 0)? as usize;
    if count == 0 || count > 64 {
        return Err(Error::Format(format!("mset: {count} sections")));
    }
    let sec = u32_at(mset, 4)? as usize;
    if u32_at(mset, sec)? != MMTN {
        return Err(Error::Format("mset: section 0 is not MMTN".into()));
    }
    let table = sec + 0x40;
    let mut out = Vec::new();
    for i in 0..4096 {
        let off = u32_at(mset, table + i * 4)?;
        if off == 0xFFFF_FFFF {
            break;
        }
        out.push(motion(mset, table + off as usize).ok().map(|mut m| {
            m.offset = off as usize + 0x40;
            m
        }));
    }
    Ok(out)
}

fn motion(b: &[u8], at: usize) -> Result<Motion> {
    let frames = u32_at(b, at + 4)?;
    let fps = f32::from_bits(u32_at(b, at + 8)?);
    let bones = u32_at(b, at + 0x10)?;
    let n = u32_at(b, at + 0x18)? as usize;
    let chans = at + u32_at(b, at + 0x1C)? as usize;
    let keys = at + u32_at(b, at + 0x28)? as usize;
    if n > 4096 || !(1.0..=240.0).contains(&fps) {
        return Err(Error::Format("mset: not a motion".into()));
    }
    let channels = read_channels(b, chans, n, keys)?;
    let words_at = at + u32_at(b, at + 0x14)? as usize;
    let bone_n = bones as usize;
    let joint_words = (0..bone_n.min(4096)).map(|i| u32_at(b, words_at + i * 4)).collect::<Result<Vec<_>>>()?;
    let hc_n = u32_at(b, at + 0x20)? as usize;
    let hc_at = at + u32_at(b, at + 0x24)? as usize;
    let helper_chans = if hc_n > 0 && hc_n < 4096 { read_channels(b, hc_at, hc_n, keys)? } else { Vec::new() };
    let h_n = u32_at(b, at + 0x2C)? as usize;
    let h_at = at + u32_at(b, at + 0x30)? as usize;
    let f = |o: usize| -> Result<f32> { Ok(f32::from_bits(u32_at(b, o)?)) };
    let helper_table = (0..h_n.min(256))
        .map(|i| {
            let e = h_at + i * 40;
            Ok(HelperRest {
                scale: [f(e)?, f(e + 4)?, f(e + 8)?],
                parent: u32_at(b, e + 0xC)? as i32,
                rotation: [f(e + 0x10)?, f(e + 0x14)?, f(e + 0x18)?],
                translation: [f(e + 0x1C)?, f(e + 0x20)?, f(e + 0x24)?],
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let c_n = u32_at(b, at + 0x34)? as usize;
    let c_at = at + u32_at(b, at + 0x38)? as usize;
    let constants = (0..c_n.min(8192))
        .map(|i| {
            let e = c_at + i * 8;
            let w = u32_at(b, e)?;
            Ok(((w & 0xFFFF) as u16, (w >> 16) as u16, f(e + 4)?))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Motion { offset: 0, frames, fps, bones, channels, helper_channels: hc_n as u32, helpers: h_n as u32, joint_words, helper_table, helper_chans, constants })
}

fn read_channels(b: &[u8], chans: usize, n: usize, keys: usize) -> Result<Vec<Channel>> {
    let mut channels = Vec::with_capacity(n);
    for c in 0..n {
        let e = chans + c * 6;
        let s = b.get(e..e + 6).ok_or_else(|| Error::Format("mset: channel past the end".into()))?;
        let bone = u16::from_le_bytes([s[0], s[1]]);
        let kind = s[2];
        let count = s[3] as usize;
        let first = u16::from_le_bytes([s[4], s[5]]) as usize;
        let mut ks = Vec::with_capacity(count);
        for k in first..first + count {
            let p = keys + k * 16;
            let head = u32_at(b, p)? as i32;
            ks.push(Key {
                time: (head >> 8) as f32 / 256.0,
                interpolation: (head & 0xFF) as u8,
                value: f32::from_bits(u32_at(b, p + 4)?),
                tangent_out: f32::from_bits(u32_at(b, p + 8)?),
                tangent_in: f32::from_bits(u32_at(b, p + 12)?),
            });
        }
        channels.push(Channel { bone, kind: kind & 0xF, before: (kind >> 4) & 3, after: kind >> 6, keys: ks });
    }
    Ok(channels)
}

impl Channel {
    /// The value at `frame` (repeating past the ends when the channel says so, else held).
    pub fn sample(&self, frame: f32) -> Option<f32> {
        let (first, last) = (self.keys.first()?, self.keys.last()?);
        if self.keys.len() == 1 {
            return Some(first.value);
        }
        let span = last.time - first.time;
        let mut t = frame;
        if t < first.time {
            if self.before == 2 && span > 0.0 {
                t = last.time - (first.time - t) % span;
            } else {
                return Some(first.value);
            }
        }
        if t >= last.time {
            if self.after == 2 && span > 0.0 {
                t = first.time + (t - first.time) % span;
            } else {
                return Some(last.value);
            }
        }
        let i = self.keys.partition_point(|k| k.time <= t).saturating_sub(1).min(self.keys.len() - 2);
        let (a, b) = (self.keys[i], self.keys[i + 1]);
        let gap = b.time - a.time;
        if gap <= 0.0 || a.interpolation == 0 {
            return Some(a.value);
        }
        let x = t - a.time;
        if a.interpolation == 1 {
            return Some(a.value + (b.value - a.value) * x / gap);
        }
        let s = x / gap;
        let (s2, s3) = (s * s, s * s * s);
        let h00 = 2.0 * s3 - 3.0 * s2 + 1.0;
        let h01 = 3.0 * s2 - 2.0 * s3;
        let h10 = s3 - 2.0 * s2 + s;
        let h11 = s3 - s2;
        Some(h00 * a.value + h01 * b.value + gap * (h10 * a.tangent_in + h11 * b.tangent_out))
    }
}

impl Motion {
    /// The skeleton with this motion's channels applied at `frame` (plain FK: the IK helper
    /// list is not evaluated).
    pub fn apply(&self, skeleton: &[Bone], frame: f32) -> Vec<Bone> {
        let mut s = skeleton.to_vec();
        for c in &self.channels {
            let (Some(b), Some(v)) = (s.get_mut(c.bone as usize), c.sample(frame)) else { continue };
            match c.kind {
                1..=3 => b.scale[(c.kind - 1) as usize] = v,
                4..=6 => b.rotation[(c.kind - 4) as usize] = v,
                7..=9 => b.translation[(c.kind - 7) as usize] = v,
                _ => {}
            }
        }
        s
    }
}
