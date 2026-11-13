//! KH1's swing timing on KH2's motion triggers (Phase 2, hit and combo windows). A KH1 swing
//! written into a KH2 swing slot (`crate::SWINGS`) keeps that slot's KH2 triggers, timed to KH1:
//! - the Keyblade's hit windows (range 10, its ATKP and collision group kept) and trail (range 12)
//!   are KH1's own (the motion's events: kind 1 hit window of attack group 0, kind 7 weapon
//!   trail, `kh1_data::mset::Event`);
//! - Sora's combo window (range 11) is KH1's (kind 5); a KH1 swing without one (the finishers)
//!   has none;
//! - every other trigger (Reaction Command window, turning to the lock-on, sounds, effects, the
//!   finisher's Finishing Plus window) moves with the swing: its frame is mapped from KH2's swing
//!   onto KH1's piecewise linearly by the hit (KH2's first hit window's start onto KH1's, its last
//!   one's end onto KH1's, the motion's end onto KH1's), clamped to KH1's length.
//!
//! Frames: KH1's events are in its motion's 60 Hz frames, KH2's triggers in its motion's key-time
//! units, the 60 Hz frames our KH1 motions are written in (`convert::write_motion`: a key per
//! frame). Checked (files): KH2's A300 ends its triggers at 48, its motion's key count.

use kh1_data::mset::Motion;
use kh2_formats::motion::{range_id, MotionTriggers, RangeTrigger};

/// The span of a trigger list's Keyblade hit windows (first start, last end).
pub fn hit_span(t: &MotionTriggers) -> Option<(i16, i16)> {
    let hits: Vec<_> = t.ranges.iter().filter(|r| r.id == range_id::ATTACK_HITBOX || r.id == range_id::ATTACK_HITBOX_COMBO).collect();
    Some((hits.iter().map(|r| r.start).min()?, hits.iter().map(|r| r.end).max()?))
}

/// KH2 swing frame `f` on KH1's swing.
fn warp(f: i16, kh2_hit: Option<(i16, i16)>, kh2_len: f32, kh1: &Motion) -> i16 {
    if f < 0 {
        return f;
    }
    let kh1_len = kh1.frames as f32;
    let kh1_hit = kh1.level_ranges(1, 0);
    let mut pts = vec![(0.0f32, 0.0f32)];
    if let (Some((a, b)), Some(first), Some(last)) = (kh2_hit, kh1_hit.first(), kh1_hit.last()) {
        pts.push((a as f32, first.0));
        pts.push((b as f32, last.1));
    }
    pts.push((kh2_len.max(1.0), kh1_len));
    pts.dedup_by(|x, y| x.0 <= y.0);
    let x = f as f32;
    let mut out = kh1_len;
    for w in pts.windows(2) {
        let ((x0, y0), (x1, y1)) = (w[0], w[1]);
        if x <= x1 {
            out = if x1 > x0 { y0 + (y1 - y0) * (x - x0).max(0.0) / (x1 - x0) } else { y1 };
            break;
        }
    }
    out.round().clamp(0.0, kh1_len) as i16
}

fn ranges_of(kh1: &Motion, kind: u16, id: u16) -> Vec<(i16, i16)> {
    kh1.level_ranges(kind, id).into_iter().map(|(a, b)| (a.round() as i16, b.round() as i16)).collect()
}

/// Sora's triggers for KH1 swing `kh1` in a KH2 slot whose own are `kh2` (its motion `kh2_len`
/// frames long, the Keyblade's hit windows `kh2_hit`).
pub fn sora(kh2: &MotionTriggers, kh2_hit: Option<(i16, i16)>, kh2_len: f32, kh1: &Motion) -> MotionTriggers {
    let w = |f: i16| warp(f, kh2_hit, kh2_len, kh1);
    let mut out = MotionTriggers::default();
    for r in &kh2.ranges {
        if r.id == range_id::ALLOW_COMBO {
            continue;
        }
        out.ranges.push(RangeTrigger { start: w(r.start), end: w(r.end), ..r.clone() });
    }
    for (start, end) in ranges_of(kh1, 5, 0) {
        out.ranges.push(RangeTrigger { start, end, id: range_id::ALLOW_COMBO, params: Vec::new() });
    }
    out.frames = kh2.frames.iter().map(|f| kh2_formats::motion::FrameTrigger { frame: w(f.frame), ..f.clone() }).collect();
    out
}

/// The Keyblade's triggers for KH1 swing `kh1` in a KH2 slot whose own are `kh2`.
pub fn keyblade(kh2: &MotionTriggers, kh2_len: f32, kh1: &Motion) -> MotionTriggers {
    let span = hit_span(kh2);
    let w = |f: i16| warp(f, span, kh2_len, kh1);
    let hit = kh2.ranges.iter().find(|r| r.id == range_id::ATTACK_HITBOX || r.id == range_id::ATTACK_HITBOX_COMBO).cloned();
    let trail = kh2.ranges.iter().find(|r| r.id == range_id::WEAPON_TRAIL).cloned();
    let mut out = MotionTriggers::default();
    for r in &kh2.ranges {
        if r.id == range_id::ATTACK_HITBOX || r.id == range_id::ATTACK_HITBOX_COMBO || r.id == range_id::WEAPON_TRAIL {
            continue;
        }
        out.ranges.push(RangeTrigger { start: w(r.start), end: w(r.end), ..r.clone() });
    }
    if let Some(h) = hit {
        for (start, end) in ranges_of(kh1, 1, 0) {
            out.ranges.push(RangeTrigger { start, end, ..h.clone() });
        }
    }
    if let Some(t) = trail {
        for (start, end) in ranges_of(kh1, 7, 0) {
            out.ranges.push(RangeTrigger { start, end, ..t.clone() });
        }
    }
    out.frames = kh2.frames.iter().map(|f| kh2_formats::motion::FrameTrigger { frame: w(f.frame), ..f.clone() }).collect();
    out
}

/// The KH2 Keyblade's hit-window span for each swing slot (`crate::SWINGS`), from KH2's
/// `W_EX010.mset`.
pub fn keyblade_spans(w_ex010: &[u8]) -> kh2_formats::Result<std::collections::HashMap<usize, (i16, i16)>> {
    let bar = kh2_formats::bar::Bar::parse(w_ex010)?;
    let mut out = std::collections::HashMap::new();
    for &(id, _) in &crate::SWINGS {
        let Some(e) = bar.entries.get(id * 4).filter(|e| !e.data.is_empty()) else { continue };
        let anb = kh2_formats::motion::Anb::parse(e.data)?;
        if let Some(s) = hit_span(&anb.triggers) {
            out.insert(id, s);
        }
    }
    Ok(out)
}

/// The frame count of an ANB's motion.
pub fn anb_frames(anb: &[u8]) -> f32 {
    kh2_formats::motion::Anb::parse(anb).ok().and_then(|a| a.pose.map(|p| p.frame_count as f32)).unwrap_or(0.0)
}

/// KH2's `W_EX010.mset` with the swing slots' Keyblade triggers timed to KH1 (`keyblade`); the
/// Keyblade's motions themselves stay KH2's. Never longer than KH2's own file (the room KH2 keeps
/// for it: the t27–t33 crash).
pub fn build_keyblade(w_ex010: &[u8], kh1_motions: &[Option<Motion>]) -> kh2_formats::Result<Vec<u8>> {
    use kh2_formats::motion::{BAR_TYPE_ANB, BAR_TYPE_MOTION_TRIGGERS};
    let out = crate::convert::map_bar(w_ex010, |row, kind, data| {
        let Some(k) = crate::kh1_motion_at(row).filter(|_| kind == BAR_TYPE_ANB && !data.is_empty()) else { return Ok(data.to_vec()) };
        let Some(m) = kh1_motions.get(k).and_then(|m| m.as_ref()) else { return Ok(data.to_vec()) };
        let len = anb_frames(data);
        crate::convert::map_bar(data, |_, kind, t| {
            if kind != BAR_TYPE_MOTION_TRIGGERS || t.is_empty() {
                return Ok(t.to_vec());
            }
            Ok(keyblade(&MotionTriggers::parse(t)?, len, m).write())
        })
    })?;
    if out.len() > w_ex010.len() {
        return Err(kh2_formats::Error::Format(format!("the Keyblade's motion set came out longer than KH2's ({} > {})", out.len(), w_ex010.len())));
    }
    Ok(out)
}
