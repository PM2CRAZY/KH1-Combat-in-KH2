//! KH1's whole skeleton pass, offline: a KH1 motion's pose with its IK helpers, aims and two-bone
//! IK, as KH1 itself poses it. The sister project's `mset::Motion::apply` gives only the keyed
//! channels; KH1 Sora is driven mostly by 11 helpers (root, spine, legs, arms), so a pose for
//! retargeting needs the rest. KH1-Combat-in-KH2 t11; our own code, from the KH1 Steam decomp
//! (plain-language reading, no decompiled code):
//!
//! - **Motion start** (`0x14038F550`): every bone's sampled values reset to the bind, then the
//!   motion's constant channels written over them. A bone with joint-word bit 25 keeps the local
//!   matrix built from those (its keys are ignored).
//! - **Sampling** (`0x140391C70`): keyed channels at the frame; the helper channels into the
//!   helper table; helper `k ≥ 1` becomes matrix `k` = its parent's matrix × its own (scale,
//!   Euler X→Y→Z, translation); matrix 0 is the world (identity here: model space).
//! - **Bones** (`0x1403900B0`), in order, under their parent bone or matrix 0 (`links & 0x3FF`
//!   = 0x3FF). The joint word: bit 26 skip; 25 static; 24 under matrix 0; 23 keep the position
//!   already placed; bits 21–22 the mode; bits 0–1 (`lo`); `A` = bits 2–10 with 27 as bit 9;
//!   `P` = bits 11–19 with 28 as bit 9 (both 1-based: helper matrix `n − 1`).
//!   - mode 0, FK: the position from helper `P` (or kept, or the parent's transform of the
//!     sampled translation); `lo` 0 the Euler rotation, 1 aimed at helper `A` (up +y) and rolled
//!     by the sampled x rotation, 2 helper `A`'s rotation copied.
//!   - mode 1, one-bone aim: the bone aims at helper `P` of the next bone's word (the solver
//!     without its second bone), the pole helper `A` when `lo` = 3, rolled by its x rotation.
//!   - modes 2 / 3, two-bone IK (`0x14038FC20`): this bone and the next reach helper `P` of the
//!     word two bones on; bend flag mode − 2 (0 legs, 1 arms); the pole helper `A` when `lo` = 3;
//!     the twists are the two bones' sampled x rotations; the lengths the next two bones' x
//!     offsets. The bone after them gets its bind offset from the second.
//! - Not done offline (they need the game's world): KH1's ground lift of a foot target, the head's
//!   look-at. Scale is carried, as KH1's pass does, on each axis with an explicit position.
//!
//! The order in which KH1's multiply (`0x1401DB4D0 (out, a, b)`) composes was taken from what the
//! solver needs to mean (a roll about the bone's own axis keeps the aim; the bend turns the bone
//! within its own plane): `out = a · b` in glam's column convention. Assumption, checked by the
//! tests (bone lengths kept, the IK end on its target).

use crate::mset::Motion;
use glam::{Mat4, Quat, Vec3};
use kh2_formats::anim::Bone;

/// A pose: model-space matrices for the bones and for the helper matrices (0 = the world).
#[derive(Clone, Debug, Default)]
pub struct RigPose {
    pub bones: Vec<Mat4>,
    pub helpers: Vec<Mat4>,
}

fn euler(r: [f32; 3]) -> Quat {
    kh2_anim::euler_quat(Vec3::from(r))
}

fn rot_x(a: f32) -> Mat4 {
    Mat4::from_rotation_x(a)
}

/// The rotation part of `m` with its axes made unit length (KH1's affine inverse drops scale).
fn unscaled(m: &Mat4) -> Mat4 {
    Mat4::from_cols(m.x_axis.truncate().normalize_or_zero().extend(0.0), m.y_axis.truncate().normalize_or_zero().extend(0.0), m.z_axis.truncate().normalize_or_zero().extend(0.0), m.w_axis)
}

/// A basis whose x is `x`, y `y`, z `z`, placed at `at`.
fn basis(x: Vec3, y: Vec3, z: Vec3, at: Vec3) -> Mat4 {
    Mat4::from_cols(x.extend(0.0), y.extend(0.0), z.extend(0.0), at.extend(1.0))
}

/// KH1's aim and two-bone IK solver (`0x14038FC20`), in the parent's space: `origin` the first
/// bone's place, `target` where the chain ends, `pole` (or the default plane), `twist` / `twist2`
/// the rolls, `l1` / `l2` the segment lengths (`l2` = 0: aim only). Returns the first bone's local
/// matrix and, for two bones, the second's local matrix (relative to the first).
pub fn solve(origin: Vec3, target: Vec3, pole: Option<Vec3>, l1: f32, l2: Option<f32>, bend_flag: bool, twist: f32, twist2: f32) -> (Mat4, Option<Mat4>) {
    let d = target - origin;
    let dist = d.length();
    let x = d.normalize_or_zero();
    let (y, z) = match pole {
        None => {
            let a = Vec3::new(-x.y, x.x, 0.0).normalize_or_zero();
            (a, x.cross(a).normalize_or_zero())
        }
        Some(p) => {
            let a = (p - origin).normalize_or_zero();
            let z = x.cross(a).normalize_or_zero();
            (z.cross(x).normalize_or_zero(), z)
        }
    };
    let mut base = basis(x, y, z, Vec3::ZERO);
    if twist != 0.0 {
        base *= rot_x(twist);
    }
    let Some(l2) = l2 else {
        return (Mat4::from_translation(origin) * base, None);
    };
    let c = ((dist * dist - l1 * l1 - l2 * l2) / (2.0 * l1 * l2)).clamp(-1.0, 1.0);
    let mut s = (1.0 - c * c).max(0.0).sqrt();
    if bend_flag {
        s = -s;
    }
    // The second bone: turned by the bend about z, at the first's length along x.
    let mut second = Mat4::from_cols(glam::Vec4::new(c, s, 0.0, 0.0), glam::Vec4::new(-s, c, 0.0, 0.0), glam::Vec4::Z, glam::Vec4::new(l1, 0.0, 0.0, 1.0));
    if twist2 != 0.0 {
        second *= rot_x(twist2);
    }
    // The first bone: off the target line so the chain's end lands on it.
    let (a, b) = (c * l2 + l1, -(s * l2));
    let n = (a * a + b * b).sqrt();
    let (c1, s1) = if n > 0.0 { (a / n, b / n) } else { (1.0, 0.0) };
    let bend1 = Mat4::from_cols(glam::Vec4::new(c1, s1, 0.0, 0.0), glam::Vec4::new(-s1, c1, 0.0, 0.0), glam::Vec4::Z, glam::Vec4::W);
    (Mat4::from_translation(origin) * base * bend1, Some(second))
}

/// The local matrix (rotation, translation; scale carried separately) of sampled values.
fn local(rot: [f32; 3], trans: [f32; 3]) -> Mat4 {
    Mat4::from_rotation_translation(euler(rot), Vec3::from(trans))
}

/// KH1 Sora's pose for `motion` at `frame` on `skeleton` (the `.mdls` bones, `parent` -1 for 0x3FF).
pub fn pose(motion: &Motion, skeleton: &[Bone], frame: f32) -> RigPose {
    let n = skeleton.len();
    // Motion start: the bind, then the constant channels.
    let mut start = skeleton.to_vec();
    for &(bone, kind, v) in &motion.constants {
        if let Some(b) = start.get_mut(bone as usize) {
            match kind {
                1..=3 => b.scale[(kind - 1) as usize] = v,
                4..=6 => b.rotation[(kind - 4) as usize] = v,
                7..=9 => b.translation[(kind - 7) as usize] = v,
                _ => {}
            }
        }
    }
    let sampled = motion.apply(&start, frame);

    // Helpers.
    let mut hs: Vec<_> = motion.helper_table.clone();
    for c in &motion.helper_chans {
        let (Some(h), Some(v)) = (hs.get_mut(c.bone as usize), c.sample(frame)) else { continue };
        match c.kind {
            1..=3 => h.scale[(c.kind - 1) as usize] = v,
            4..=6 => h.rotation[(c.kind - 4) as usize] = v,
            7..=9 => h.translation[(c.kind - 7) as usize] = v,
            _ => {}
        }
    }
    let mut helpers = vec![Mat4::IDENTITY; hs.len().max(1)];
    for k in 1..hs.len() {
        let h = &hs[k];
        let own = Mat4::from_scale_rotation_translation(Vec3::from(h.scale), euler(h.rotation), Vec3::from(h.translation));
        let p = h.parent;
        let parent = if p >= 0 && (p as usize) < k { helpers[p as usize] } else { helpers[0] };
        helpers[k] = parent * own;
    }
    let hpos = |n1: u32| -> Option<Vec3> { (n1 >= 1).then(|| helpers.get(n1 as usize - 1).map(|m| m.w_axis.truncate())).flatten() };
    let hmat = |n1: u32| -> Option<Mat4> { (n1 >= 1).then(|| helpers.get(n1 as usize - 1).copied()).flatten() };

    let word = |i: usize| motion.joint_words.get(i).copied().unwrap_or(0);
    let a_of = |w: u32| ((w >> 16 & 0x800) | (w & 0x7FC)) >> 2;
    let p_of = |w: u32| ((w >> 8 & 0x10_0000) | (w & 0xF_F800)) >> 11;

    let mut world = vec![Mat4::IDENTITY; n];
    let mut placed = vec![false; n];
    let mut i = 0;
    while i < n {
        let w = word(i);
        let parent = match skeleton[i].parent {
            p if p >= 0 && (p as usize) < i => unscaled(&world[p as usize]),
            _ => helpers[0],
        };
        let inv = parent.inverse();
        let s = &sampled[i];
        let st = &start[i];
        let mode = (w >> 21) & 3;
        if mode == 2 || mode == 3 {
            // Two-bone IK on i and i + 1, the end on helper P of word i + 2.
            let end = i + 2;
            if end < n {
                let target = hpos(p_of(word(end))).map(|t| inv.transform_point3(t));
                let pole = if w & 3 == 3 { hpos(a_of(w)).map(|t| inv.transform_point3(t)) } else { None };
                let l1 = skeleton[i + 1].translation[0] * s.scale[0];
                let l2 = s.scale[0] * sampled[i + 1].scale[0] * skeleton[end].translation[0];
                if let Some(t) = target {
                    let (first, second) = solve(Vec3::from(s.translation), t, pole, l1, Some(l2), mode == 3, s.rotation[0], sampled[i + 1].rotation[0]);
                    world[i] = parent * first;
                    world[i + 1] = world[i] * second.unwrap_or(Mat4::IDENTITY);
                    world[end] = Mat4::from_translation(world[i + 1].transform_point3(Vec3::from(skeleton[end].translation)));
                    placed[i] = true;
                    placed[i + 1] = true;
                    placed[end] = true;
                    i += 2;
                    continue;
                }
            }
        }
        if mode == 1 && i + 1 < n {
            // One-bone aim at helper P of the next bone's word.
            let target = hpos(p_of(word(i + 1))).map(|t| inv.transform_point3(t));
            let pole = if w & 3 == 3 { hpos(a_of(w)).map(|t| inv.transform_point3(t)) } else { None };
            if let Some(t) = target {
                let l1 = skeleton[i + 1].translation[0] * s.scale[0];
                let (first, _) = solve(Vec3::from(s.translation), t, pole, l1, None, false, s.rotation[0], 0.0);
                world[i] = parent * first;
                placed[i] = true;
                i += 1;
                continue;
            }
        }
        // Mode 0: FK.
        let keep = w & 0x80_0000 != 0;
        let p = p_of(w);
        let pos = if keep && placed[i] {
            world[i].w_axis.truncate()
        } else if let Some(h) = hpos(p) {
            h
        } else {
            parent.transform_point3(Vec3::from(s.translation))
        };
        let lo = w & 3;
        let rot: Mat4 = if lo == 2 {
            hmat(a_of(w)).map(|m| unscaled(&m)).unwrap_or(parent)
        } else if lo == 1 {
            let local_pos = inv.transform_point3(pos);
            let t = hpos(a_of(w)).map(|h| inv.transform_point3(h)).unwrap_or(local_pos + Vec3::X);
            let x = (t - local_pos).normalize_or_zero();
            let c = x.cross(Vec3::Y);
            let y = c.cross(x).normalize_or_zero();
            let z = x.cross(y).normalize_or_zero();
            parent * basis(x, y, z, Vec3::ZERO) * rot_x(s.rotation[0])
        } else if w >> 24 & 1 != 0 {
            helpers[0] * local(s.rotation, s.translation)
        } else if w >> 25 & 1 != 0 {
            parent * local(st.rotation, st.translation)
        } else {
            parent * local(s.rotation, s.translation)
        };
        world[i] = Mat4::from_cols(rot.x_axis, rot.y_axis, rot.z_axis, pos.extend(1.0));
        placed[i] = true;
        i += 1;
    }
    RigPose { bones: world, helpers }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_bone_ik_reaches_its_target_and_keeps_its_lengths() {
        for (t, pole, flag) in [
            (Vec3::new(30.0, -40.0, 10.0), None, false),
            (Vec3::new(10.0, 50.0, -20.0), Some(Vec3::new(0.0, 0.0, 80.0)), true),
        ] {
            let (a, b) = solve(Vec3::ZERO, t, pole, 40.0, Some(35.0), flag, 0.3, -0.2);
            let knee = a.transform_point3(Vec3::new(40.0, 0.0, 0.0));
            let second = a * b.unwrap();
            let foot = second.transform_point3(Vec3::new(35.0, 0.0, 0.0));
            assert!((knee.length() - 40.0).abs() < 1e-3, "thigh {}", knee.length());
            assert!(((foot - knee).length() - 35.0).abs() < 1e-3);
            assert!((foot - t).length() < 1e-2, "end {foot:?} target {t:?}");
        }
    }

    #[test]
    fn aim_points_x_at_the_target() {
        let (a, _) = solve(Vec3::new(1.0, 2.0, 3.0), Vec3::new(1.0, 12.0, 3.0), None, 10.0, None, false, 0.7, 0.0);
        let tip = a.transform_point3(Vec3::new(10.0, 0.0, 0.0));
        assert!((tip - Vec3::new(1.0, 12.0, 3.0)).length() < 1e-3);
    }
}
