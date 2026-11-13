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

/// KH2's "blend to idle" range (OpenKH's trigger list: id 2, from when the motion may end).
const BLEND_TO_IDLE: u8 = 2;
/// KH1's "may be left" event (kind 0: from its time KH1 lets him act; the roll's at 30).
const KH1_MAY_LEAVE: u16 = 0;

/// For a motion without hit windows (the roll): KH2's blend-to-idle start and KH1's "may be
/// left" time, the anchor the rest of KH2's triggers move by.
fn free_anchor(kh2: &MotionTriggers, kh1: &Motion) -> Option<(f32, f32)> {
    let a = kh2.ranges.iter().filter(|r| r.id == BLEND_TO_IDLE && r.start >= 0).map(|r| r.start).min()?;
    let b = kh1.events.iter().filter(|e| e.kind == KH1_MAY_LEAVE).flat_map(|e| e.keys.iter().map(|k| k.0)).fold(f32::NAN, f32::min);
    (!b.is_nan()).then_some((a as f32, b))
}

/// KH2 swing frame `f` on KH1's swing.
fn warp(f: i16, kh2_hit: Option<(i16, i16)>, kh2_len: f32, kh1: &Motion) -> i16 {
    warp_by(f, kh2_hit, None, kh2_len, kh1)
}

/// `warp`, with a motion's free anchor (`free_anchor`) when it has no hit windows.
fn warp_by(f: i16, kh2_hit: Option<(i16, i16)>, free: Option<(f32, f32)>, kh2_len: f32, kh1: &Motion) -> i16 {
    if f < 0 {
        return f;
    }
    let kh1_len = kh1.frames as f32;
    let kh1_hit = kh1.level_ranges(1, 0);
    let mut pts = vec![(0.0f32, 0.0f32)];
    if let (Some((a, b)), Some(first), Some(last)) = (kh2_hit, kh1_hit.first(), kh1_hit.last()) {
        pts.push((a as f32, first.0));
        pts.push((b as f32, last.1));
    } else if let Some(p) = free.filter(|p| p.0 > 0.0 && p.0 < kh2_len && p.1 > 0.0 && p.1 < kh1_len) {
        pts.push(p);
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
    let free = if kh2_hit.is_none() { free_anchor(kh2, kh1) } else { None };
    let w = |f: i16| warp_by(f, kh2_hit, free, kh2_len, kh1);
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

/// Sora's triggers for `row` (`kh1` its KH1 motion, `total` the frames written there with any
/// prelude): a cast's (`crate::CASTS`) or a swing's and the rest (`sora`).
pub fn for_row(row: usize, kh2: &MotionTriggers, kh2_hit: Option<(i16, i16)>, kh2_len: f32, kh1: &Motion, total: i16) -> MotionTriggers {
    match crate::cast_at(row) {
        Some((releases, free)) => cast(kh2, kh2_len, total, releases, free),
        None => sora(kh2, kh2_hit, kh2_len, kh1),
    }
}

/// A KH2 cast motion's triggers on a KH1 cast of `len` frames: each magic-cast event (frame
/// trigger 17) at KH1's release for its parameter (`releases`); everything else moved piecewise
/// so KH2's cast frame lands on KH1's (first) release, KH2's free point (the start of its open
/// Reaction Command range, 14) on KH1's `free`, and its end on KH1's.
pub fn cast(kh2: &MotionTriggers, kh2_len: f32, len: i16, releases: &[(i16, i16)], free: i16) -> MotionTriggers {
    use kh2_formats::motion::frame_id;
    let cast2 = kh2.frames.iter().find(|f| f.id == frame_id::MAGIC_CAST).map_or(0, |f| f.frame);
    let free2 = kh2.ranges.iter().find(|r| r.id == range_id::ALLOW_RC && r.end < 0).map_or(kh2_len as i16, |r| r.start).max(cast2);
    let rel = releases.first().map_or(cast2, |r| r.1);
    let free1 = free.clamp(rel, len);
    let seg = |f: f32, a0: f32, a1: f32, b0: f32, b1: f32| if a1 > a0 { b0 + (f - a0) * (b1 - b0) / (a1 - a0) } else { b0 };
    let w = |f: i16| -> i16 {
        if f < 0 {
            return f;
        }
        let x = f as f32;
        let y = if f <= cast2 {
            seg(x, 0.0, cast2 as f32, 0.0, rel as f32)
        } else if f <= free2 {
            seg(x, cast2 as f32, free2 as f32, rel as f32, free1 as f32)
        } else {
            seg(x, free2 as f32, kh2_len, free1 as f32, len as f32)
        };
        (y.round() as i16).clamp(0, len)
    };
    let mut out = MotionTriggers::default();
    for r in &kh2.ranges {
        out.ranges.push(RangeTrigger { start: w(r.start), end: w(r.end), ..r.clone() });
    }
    out.frames = kh2
        .frames
        .iter()
        .map(|f| {
            let at = if f.id == frame_id::MAGIC_CAST {
                f.params.first().and_then(|p| releases.iter().find(|r| r.0 == *p)).map_or(w(f.frame), |r| r.1)
            } else {
                w(f.frame)
            };
            kh2_formats::motion::FrameTrigger { frame: at, ..f.clone() }
        })
        .collect();
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

/// The KH2 Keyblade's hit-window span for each motion set row we write (`crate::kh1_slots`),
/// from KH2's `W_EX010.mset`.
pub fn keyblade_spans(w_ex010: &[u8]) -> kh2_formats::Result<std::collections::HashMap<usize, (i16, i16)>> {
    let bar = kh2_formats::bar::Bar::parse(w_ex010)?;
    let mut out = std::collections::HashMap::new();
    for (row, _) in crate::kh1_slots() {
        let Some(e) = bar.entries.get(row).filter(|e| !e.data.is_empty() && e.name != "DUMM") else { continue };
        let anb = kh2_formats::motion::Anb::parse(e.data)?;
        if let Some(s) = hit_span(&anb.triggers) {
            out.insert(row, s);
        }
    }
    Ok(out)
}

/// The frame count of an ANB's motion.
pub fn anb_frames(anb: &[u8]) -> f32 {
    kh2_formats::motion::Anb::parse(anb).ok().and_then(|a| a.pose.map(|p| p.frame_count as f32)).unwrap_or(0.0)
}

/// The Keyblade's motion set for KH1's slots: in each row Sora plays a KH1 motion in, the
/// Keyblade's hit windows and trail at KH1's times (`keyblade`). With `hold` (KH1's model: KH1's
/// skeleton, the Keyblade's), also its motion: KH1's own Keyblade bones in its palm. KH2's
/// Keyblade skeleton is KH1's grip chain bone for bone (its bone k is KH1's 280 + k, the same
/// parents, Checked (files)), and KH2 hangs it on KH1's palm (`player_model::PALMS`), 280's parent,
/// so the Keyblade's bones are KH1's 280.. in the palm's space at each frame of KH1's motion, and
/// the blade (bone 2, KH1's 282) goes where KH1 holds it, swing by swing. Refuses a set longer
/// than KH2's (the area heap; t34).
pub fn build_keyblade(w_ex010: &[u8], kh1_motions: &[Option<Motion>], hold: Option<(&[kh2_formats::anim::Bone], &[kh2_formats::anim::Bone])>) -> kh2_formats::Result<Vec<u8>> {
    use kh2_formats::motion::{BAR_TYPE_ANB, BAR_TYPE_MOTION, BAR_TYPE_MOTION_TRIGGERS};
    // The knock-down rows (`crate::KNOCKDOWN`) share their template's Keyblade motion (KH1 holds
    // the blade still in the palm in both, t49), so the set doesn't grow.
    let out = crate::convert::map_bar_shared(w_ex010, &crate::fills(), |row, kind, data| {
        // (KH1's casts keep KH2's Keyblade motion: `crate::CASTS`.)
        let Some(k) = crate::kh1_motion_at(row).filter(|_| kind == BAR_TYPE_ANB && !data.is_empty() && crate::cast_at(row).is_none()) else { return Ok(data.to_vec()) };
        let Some(m) = kh1_motions.get(k).and_then(|m| m.as_ref()) else { return Ok(data.to_vec()) };
        let len = anb_frames(data);
        crate::convert::map_bar(data, |_, kind, t| {
            if kind == BAR_TYPE_MOTION && !t.is_empty() {
                if let Some((kh1, blade)) = hold {
                    return keyblade_motion(t, m, kh1, blade);
                }
            }
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

/// KH1's grip bone (the Keyblade's root, the first of KH1's Keyblade bones).
const KH1_GRIP: usize = 280;

/// The Keyblade's motion for KH1 motion `m`: KH1's Keyblade bones (280 + k for its bone k) in
/// their palm's space, a frame at a time, on the Keyblade's own motion `template`.
pub fn keyblade_motion(template: &[u8], m: &Motion, kh1: &[kh2_formats::anim::Bone], blade: &[kh2_formats::anim::Bone]) -> kh2_formats::Result<Vec<u8>> {
    let palm = kh1.get(KH1_GRIP).map(|b| b.parent).filter(|&p| p >= 0).ok_or_else(|| kh2_formats::Error::Format("KH1's grip bone has no parent".into()))? as usize;
    if KH1_GRIP + blade.len() > kh1.len() {
        return Err(kh2_formats::Error::Format("KH1's skeleton is shorter than the Keyblade's".into()));
    }
    let frames = m.frames.max(1) as usize;
    let poses: Vec<Vec<glam::Mat4>> = (0..frames)
        .map(|f| {
            let p = kh1_data::rig::pose(m, kh1, f as f32).bones;
            let inv = p[palm].inverse();
            (0..blade.len()).map(|k| inv * p[KH1_GRIP + k]).collect()
        })
        .collect();
    // The blade (bones 0..2, its mesh on bone 2 out to 78 units, Checked (files)) turned within
    // what moves its tip `REACH_TOL`; the keychain's links (3 to 21 units) within 1.7 degrees, so
    // the set stays within KH2's size (the keychain's sway is most of the keys).
    let tol: Vec<f32> = (0..blade.len()).map(|k| crate::convert::reach_tolerance(if k <= 2 { BLADE_REACH } else { KEYCHAIN_REACH })).collect();
    crate::convert::write_motion_tol(template, blade, &poses, &vec![glam::Vec3::ZERO; frames], &tol)
}

/// The Kingdom Key's blade from its grip (bone 2's mesh, Checked (files): out to 77.9), and the
/// keychain's reach for its tolerance (0.03 radians, the most `convert::reach_tolerance` gives).
const BLADE_REACH: f32 = 80.0;
const KEYCHAIN_REACH: f32 = 4.0;
