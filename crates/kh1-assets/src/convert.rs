//! KH1 motions as KH2 motions (Phase 2 step 1, `docs/combos.md`; KH1-Combat-in-KH2 t12).
//!
//! 1. KH1 Sora's pose each frame: KH1's whole skeleton pass (`kh1_data::rig`).
//! 2. KH1's root travel taken off the body: KH1 keeps it in its last helper (H10) and the body
//!    rides it; KH2 keeps it in its last IK helper (Sora's joint 52, the curve the motion's root
//!    block points at: Checked (files), A300's travel curve on helper 52 z ending at 150, the OG
//!    rewrite's 150). So the body is posed at the travel's start and the travel becomes KH2's.
//! 3. The pose onto KH2 Sora's skeleton: the sister project's reverse retarget
//!    (`kh1_data::inverse`), which places KH2's joints at KH2's own bind offsets.
//! 4. Written as a KH2 interpolated motion on a template motion of KH2's own (its reserved block,
//!    its IK helper table, joint table, bounding box and root block kept): an initial pose with
//!    every bone's first-frame values, Hermite keys where a channel moves (fitted within a small
//!    tolerance), the travel on helper 52, no constraints or expressions (the pose is baked).
//!    Layout: `kh2_formats::anim` (OpenKH's docs), Checked (files) against KH2 Sora's motions.
//!
//! Units: KH1's motions run at 60 fps; KH2's key times are 60 Hz frames, so a KH1 frame is a key
//! time (Checked (files): KH1 Sora's motions are 60 fps; KH2's A300 48 key frames at fps 30).
//! Distances: KH1's values as they are (the 1:1 rule); the travel is KH1's, unscaled.

use glam::{EulerRot, Mat4, Quat, Vec3};
use kh1_data::inverse::Inverse;
use kh1_data::mset::Motion;
use kh1_data::retarget::{Retarget, GRIP_BONE};
use kh2_formats::anim::Bone;
use kh2_formats::{Error, Result};

/// KH2 Sora's travel helper: his joint 52 past the 228 bones (Checked (files), A300).
const TRAVEL_JOINT: i16 = 52;
/// The travel helper of a motion for a new skeleton (`write_motion`): its second helper.
const OWN_TRAVEL_JOINT: i16 = 1;
const ROT_TOL: f32 = 0.004;
const POS_TOL: f32 = 0.05;
/// How far (model units) a rotation's rounding may move the farthest joint a bone carries.
const REACH_TOL: f32 = 0.12;
const ROT_TOL_MAX: f32 = 0.03;
/// The grid key tangents are rounded to (value per frame).
const TANGENT_STEP: f32 = 1.0 / 1024.0;

/// Per bone: `REACH_TOL` over the farthest bind distance from the bone to a joint below it.
fn rotation_tolerances(bones: &[Bone]) -> Vec<f32> {
    let bind = kh2_anim::Pose::bind(bones).bones;
    let at: Vec<Vec3> = bind.iter().map(|m| m.w_axis.truncate()).collect();
    let mut reach = vec![0.0f32; bones.len()];
    for i in 0..bones.len() {
        let mut p = bones[i].parent;
        while p >= 0 && (p as usize) < i {
            let d = (at[i] - at[p as usize]).length();
            reach[p as usize] = reach[p as usize].max(d);
            p = bones[p as usize].parent;
        }
    }
    reach.iter().map(|&r| reach_tolerance(r)).collect()
}
const BASE: usize = 0x90;

/// KH1's torso lean (degrees, hips to neck from up) and what it becomes on KH2's body: KH1's
/// combo leans mapped onto KH2's own (owner's choice (b), t25: KH1's timing, swing path and travel
/// on KH2's proportions). Quantiles over every frame of the combos, KH1's on KH2's skeleton
/// against KH2's own attacks (151–155, 181–184): median 46 → 35, 95th percentile 68 → 60, the
/// largest (KH1 98's 102) → 65. Checked (files: test `dump_kh2_vs_kh1_extremes`).
const LEAN_MAP: [(f32, f32); 5] = [(0.0, 0.0), (46.0, 35.0), (68.0, 60.0), (102.0, 65.0), (180.0, 90.0)];
/// KH2's hips-to-neck bones (the root and the neck), its two thighs, and its foot bones (both
/// sides' ankle, heel and toe). Checked (files: KH2 Sora's skeleton, `retarget::PLUS_X` / `MINUS_X`).
const KH2_HIPS: usize = 0;
const KH2_NECK: usize = 141;
const KH2_THIGHS: [usize; 2] = [11, 40];
const KH2_FEET: [usize; 6] = [13, 16, 17, 42, 45, 46];

fn lean_target(t: f32) -> f32 {
    for w in LEAN_MAP.windows(2) {
        let ((a, fa), (b, fb)) = (w[0], w[1]);
        if t <= b {
            return fa + (fb - fa) * (t - a) / (b - a);
        }
    }
    t
}

/// Tilts the pelvis and everything above it back about the hips so the lean is KH2's
/// (`LEAN_MAP`); the legs keep their turns and hang from the moved hip joints, then the body is
/// moved up or down so the lowest foot is where it was (the feet stay on the ground).
fn soften_lean(kh2: &[Bone], pose: &mut [Mat4]) {
    let n = pose.len();
    let at = |m: &Mat4| m.w_axis.truncate();
    let trunk = at(&pose[KH2_NECK]) - at(&pose[KH2_HIPS]);
    if trunk.length_squared() < 1e-6 {
        return;
    }
    let tilt = trunk.normalize().dot(Vec3::Y).clamp(-1.0, 1.0).acos().to_degrees();
    let want = lean_target(tilt);
    if (tilt - want).abs() < 0.01 {
        return;
    }
    let axis = trunk.cross(Vec3::Y);
    if axis.length_squared() < 1e-8 {
        return;
    }
    // Turning the trunk about (trunk × up) by a positive angle brings it toward up.
    let turn = Quat::from_axis_angle(axis.normalize(), (tilt - want).to_radians());
    let pivot = at(&pose[KH2_HIPS]);
    let low = |p: &[Mat4]| KH2_FEET.iter().filter(|&&b| b < n).map(|&b| at(&p[b]).y).fold(f32::MAX, f32::min);
    let before = low(pose);
    // Which bones are legs (a thigh or below it).
    let mut leg = vec![false; n];
    for b in 0..n {
        let p = usize::try_from(kh2[b].parent).ok().filter(|&p| p < b);
        leg[b] = KH2_THIGHS.contains(&b) || p.is_some_and(|p| leg[p]);
    }
    let old: Vec<Mat4> = pose.to_vec();
    let rot = Mat4::from_translation(pivot) * Mat4::from_quat(turn) * Mat4::from_translation(-pivot);
    for b in 0..n {
        if !leg[b] {
            pose[b] = rot * old[b];
        }
    }
    // Each leg follows its hip joint (the thigh's parent) without turning.
    for b in 0..n {
        if leg[b] {
            let mut root = b;
            while !KH2_THIGHS.contains(&root) {
                root = kh2[root].parent as usize;
            }
            let hip = kh2[root].parent as usize;
            let shift = at(&pose[hip]) - at(&old[hip]);
            pose[b] = Mat4::from_translation(shift) * old[b];
        }
    }
    let dy = before - low(pose);
    if dy.is_finite() {
        let up = Mat4::from_translation(Vec3::new(0.0, dy, 0.0));
        for m in pose.iter_mut() {
            *m = up * *m;
        }
    }
}

/// KH2 model-space matrices for every frame of a KH1 motion, and its travel per frame.
/// KH1's blend into the motion after this one, baked onto its end: `frames` more frames going from
/// the last pose to `next` (the next motion's first pose, `next_travel` its travel), each bone's
/// local turn slerped and its offset and scale lerped, the travel held. KH1 blends every queued
/// motion in (`0x14029FA30(.., 8.0)`, 8 frames, the sister project's `kh1-hit-reactions-native.md`
/// §4, Checked (decomp) there); KH2 switches without one where its own motions meet end to start
/// (Checked (files), test `transition_gaps`: its landing into its field idle 0.6 apart, its air
/// flinch into its fall 1.9; KH1's 15.7 and 16.2), so there the blend is in the data.
pub fn append_blend(poses: &mut Vec<Vec<Mat4>>, travel: &mut Vec<Vec3>, next: &[Mat4], next_travel: Vec3, bones: &[Bone], frames: usize) {
    let (Some(last), Some(&t_end)) = (poses.last().cloned(), travel.last()) else { return };
    if next.len() != last.len() || last.len() != bones.len() {
        return;
    }
    // The next pose where he stands at the end (its travel moved onto the held one).
    let shift = Mat4::from_translation(t_end - next_travel);
    let next: Vec<Mat4> = next.iter().map(|m| shift * *m).collect();
    let local = |w: &[Mat4], i: usize| -> (Vec3, Quat, Vec3) {
        let p = bones[i].parent;
        let m = if p >= 0 && (p as usize) < i { w[p as usize].inverse() * w[i] } else { w[i] };
        m.to_scale_rotation_translation()
    };
    let la: Vec<_> = (0..bones.len()).map(|i| local(&last, i)).collect();
    let lb: Vec<_> = (0..bones.len()).map(|i| local(&next, i)).collect();
    for k in 1..=frames {
        let t = k as f32 / frames as f32;
        let mut w: Vec<Mat4> = Vec::with_capacity(bones.len());
        for i in 0..bones.len() {
            let ((sa, ra, ta), (sb, rb, tb)) = (la[i], lb[i]);
            let m = Mat4::from_scale_rotation_translation(sa.lerp(sb, t), ra.slerp(rb, t), ta.lerp(tb, t));
            let p = bones[i].parent;
            w.push(if p >= 0 && (p as usize) < i { w[p as usize] * m } else { m });
        }
        poses.push(w);
        travel.push(t_end);
    }
}

pub fn kh2_poses(motion: &Motion, kh1: &[Bone], kh2: &[Bone]) -> (Vec<Vec<Mat4>>, Vec<Vec3>) {
    let r = Retarget::new(kh1, kh2);
    let mut inv = Inverse::new(kh1, kh2, &r, |i| i < GRIP_BONE && r.follows_one_part(i));
    // Torso bones KH1 turns by copying a helper's rotation are measured from that helper's frame
    // at rest: its parent helpers' turns (the helper table's), without its own.
    for b in inv.torso_bones() {
        let w = motion.joint_words.get(b).copied().unwrap_or(0);
        if (w >> 21) & 3 == 0 && w & 3 == 2 {
            let a = (((w >> 16 & 0x800) | (w & 0x7FC)) >> 2) as usize;
            if a >= 1 {
                let mut q = glam::Quat::IDENTITY;
                let mut h = motion.helper_table.get(a - 1).map(|x| x.parent).unwrap_or(-1);
                let mut guard = 0;
                while h >= 1 && guard < 16 {
                    let Some(x) = motion.helper_table.get(h as usize) else { break };
                    q = kh2_anim::euler_quat(glam::Vec3::from(x.rotation)) * q;
                    h = x.parent;
                    guard += 1;
                }
                inv.set_torso_rest(b, q);
            }
        }
    }
    let n = motion.frames.max(1) as usize;
    let mut poses = Vec::with_capacity(n);
    let mut travel = Vec::with_capacity(n);
    for f in 0..n {
        let p = kh1_data::rig::pose(motion, kh1, f as f32);
        let t = p.helpers.last().map(|m| m.w_axis.truncate()).unwrap_or(Vec3::ZERO);
        let back = Mat4::from_translation(-t);
        let body: Vec<Mat4> = p.bones.iter().map(|m| back * *m).collect();
        let mut kp = inv.pose(&body);
        soften_lean(kh2, &mut kp);
        // KH2's own motions keep the travel in the bones too (A300's root ends at z 145 with its
        // travel at 150, Checked (files)): KH2 draws the model at the object less the travel, so
        // bones without it were drawn behind Sora and caught up when the swing ended (the owner,
        // t41: "rubberbands").
        let on = Mat4::from_translation(t);
        poses.push(kp.into_iter().map(|m| on * m).collect());
        travel.push(t);
    }
    (poses, travel)
}

/// KH2's own motion `data` (a KH2 Sora motion) as model-space poses every frame, as KH2 draws it
/// (constraints and IK solved), and its travel: the converter's round trip, for testing it.
pub fn kh2_round_trip(data: &[u8], kh2: &[Bone]) -> Result<(Vec<Vec<Mat4>>, Vec<Vec3>)> {
    let m = kh2_formats::anim::Interpolated::parse(data)?.ok_or_else(|| Error::Format("not an interpolated motion".into()))?;
    let info = kh2_formats::motion::MotionInfo::parse(data)?;
    let n = m.frame_count.max(1) as usize;
    // A motion's keys run from its start (header `+0x80`, 30 Hz) on: KH2's walk from 40 to 112
    // in 60 Hz key time, its landing from 84 (Checked (files)). The frame count is its length.
    let t0 = 2.0 * info.frame_start.max(0.0);
    let poses = (0..n).map(|f| kh2_anim::evaluate(kh2, &m, t0 + f as f32, kh2_anim::Options { constraints: true }).bones).collect();
    let travel = (0..n).map(|f| Vec3::from(info.root_at(t0 + f as f32))).collect();
    Ok((poses, travel))
}

/// Euler X→Y→Z (KH2's order, `kh2_anim::euler_quat`) of `q`, the one nearer `prev`.
fn euler_near(q: Quat, prev: Option<[f32; 3]>) -> [f32; 3] {
    let (z, y, x) = q.to_euler(EulerRot::ZYX);
    let a = [x, y, z];
    let b = [x + std::f32::consts::PI, std::f32::consts::PI - y, z + std::f32::consts::PI];
    let Some(p) = prev else { return a };
    let unwrap = |v: [f32; 3]| -> [f32; 3] {
        let mut o = v;
        for i in 0..3 {
            let tau = std::f32::consts::TAU;
            o[i] = v[i] + ((p[i] - v[i]) / tau).round() * tau;
        }
        o
    };
    let (ua, ub) = (unwrap(a), unwrap(b));
    let d = |v: [f32; 3]| (0..3).map(|i| (v[i] - p[i]).abs()).sum::<f32>();
    if d(ub) < d(ua) { ub } else { ua }
}

/// Each bone's local values (scale, Euler rotation, translation) every frame.
/// The first frame's angles are written the way KH2 writes that bone (`seed`, the template's own
/// at its first frame): of a turn's two Euler forms (and their whole turns), the one nearest
/// KH2's. KH2 blends motions by mixing these numbers, so a bone written in the other form turned
/// the long way round during every blend in or out (the owner: "stretched like CRAZY").
fn locals(kh2: &[Bone], poses: &[Vec<Mat4>], seed: &[Option<[f32; 3]>]) -> Vec<Vec<[f32; 9]>> {
    let mut out = vec![Vec::with_capacity(poses.len()); kh2.len()];
    let mut prev: Vec<Option<[f32; 3]>> = (0..kh2.len()).map(|b| seed.get(b).copied().flatten()).collect();
    for pose in poses {
        for (b, bone) in kh2.iter().enumerate() {
            let (_, q, pos) = pose[b].to_scale_rotation_translation();
            let (lq, lt) = match usize::try_from(bone.parent).ok().filter(|&p| p < b) {
                Some(p) => {
                    let (_, pq, _) = pose[p].to_scale_rotation_translation();
                    (pq.inverse() * q, pose[p].inverse().transform_point3(pos))
                }
                None => (q, pos),
            };
            let e = euler_near(lq.normalize(), prev[b]);
            prev[b] = Some(e);
            // The offset from the parent as the pose has it (`Inverse` already places every joint at
            // KH2's own bind distance, so KH2's proportions hold).
            out[b].push([bone.scale[0], bone.scale[1], bone.scale[2], e[0], e[1], e[2], lt.x, lt.y, lt.z]);
        }
    }
    out
}

fn i32_at(b: &[u8], at: usize) -> Result<i32> {
    b.get(at..at + 4).map(|s| i32::from_le_bytes(s.try_into().unwrap())).ok_or_else(|| Error::Format(format!("motion: read past the end at {at:#x}")))
}

/// Each bone's Euler angles in `template` (a KH2 motion) at its first frame: its curve, else its
/// starting value, else KH2's bind.
fn template_angles(template: &[u8], kh2: &[Bone]) -> Vec<Option<[f32; 3]>> {
    let Ok(Some(m)) = kh2_formats::anim::Interpolated::parse(template) else { return vec![None; kh2.len()] };
    (0..kh2.len())
        .map(|b| {
            let mut r = kh2[b].rotation;
            for (i, v) in r.iter_mut().enumerate() {
                let c = (3 + i) as u8;
                if let Some(cur) = m.forward.iter().find(|x| x.joint as usize == b && x.channel == c) {
                    *v = cur.eval(0.0);
                } else if let Some(p) = m.initial_pose.iter().find(|x| x.0 as usize == b && x.1 == c as i16) {
                    *v = p.2;
                }
            }
            Some(r)
        })
        .collect()
}

struct Curve {
    joint: i16,
    channel: u8,
    /// (frame, value, tangent: value per frame, in and out alike).
    keys: Vec<(usize, f32, f32)>,
}

/// Hermite keys for `v` (a value a frame) that stay within `tol` of every frame: KH2's own key
/// kind, its curve between two keys `(2n³-3n²+1)a + (-2n³+3n²)b + (n³-2n²+n)·a_out·dt +
/// (n³-n²)·b_in·dt` (`kh2_formats::anim::FCurve`). Each key's tangent is the samples' slope there
/// (central difference). Starting from the two ends, the frame missed worst gets a key until
/// none is off by more than `tol`. A smooth swing needs a few keys where straight lines between
/// keys needed one every frame or two (the t27 motion set was 3.7 times KH2's size).
fn fit_hermite(v: &[f32], tol: f32) -> Vec<(usize, f32, f32)> {
    let n = v.len();
    if n == 0 {
        return Vec::new();
    }
    // Tangents on a grid of `TANGENT_STEP`: most are near 0 and then share one stored value
    // (the curve is checked against the samples with the rounded tangents, so `tol` holds).
    let slope = |i: usize| -> f32 {
        let s = match (i, n) {
            (_, 1) => 0.0,
            (0, _) => v[1] - v[0],
            (i, n) if i == n - 1 => v[i] - v[i - 1],
            (i, _) => (v[i + 1] - v[i - 1]) / 2.0,
        };
        (s / TANGENT_STEP).round() * TANGENT_STEP
    };
    let at = |a: usize, b: usize, t: usize| -> f32 {
        let dt = (b - a) as f32;
        let x = (t - a) as f32 / dt;
        let (x2, x3) = (x * x, x * x * x);
        (2.0 * x3 - 3.0 * x2 + 1.0) * v[a] + (-2.0 * x3 + 3.0 * x2) * v[b] + (x3 - 2.0 * x2 + x) * slope(a) * dt + (x3 - x2) * slope(b) * dt
    };
    let mut keys = vec![0];
    if n > 1 {
        keys.push(n - 1);
    }
    let mut seg = 0;
    while seg + 1 < keys.len() {
        let (a, b) = (keys[seg], keys[seg + 1]);
        let mut worst = (0.0f32, 0);
        for t in a + 1..b {
            let e = (at(a, b, t) - v[t]).abs();
            if e > worst.0 {
                worst = (e, t);
            }
        }
        if worst.0 > tol {
            keys.insert(seg + 1, worst.1);
        } else {
            seg += 1;
        }
    }
    // Then drop every key its neighbours can do without (the insertion keeps some a later one
    // made needless).
    let fits = |a: usize, b: usize| (a + 1..b).all(|t| (at(a, b, t) - v[t]).abs() <= tol);
    let mut j = 1;
    while j + 1 < keys.len() {
        if fits(keys[j - 1], keys[j + 1]) {
            keys.remove(j);
        } else {
            j += 1;
        }
    }
    keys.into_iter().map(|i| (i, v[i], slope(i))).collect()
}

static OWN_SEED: std::sync::RwLock<Option<Vec<Option<[f32; 3]>>>> = std::sync::RwLock::new(None);

/// The Euler forms every motion for a new skeleton starts from (`write_motion`): the bones' angles
/// in one reference pose, so all motions pick the same form of a turn and KH2's blends between
/// them, which mix these numbers, stay short. Until set, the bind's.
pub fn set_own_seed(bones: &[Bone], reference: &[Mat4]) {
    let bind: Vec<Option<[f32; 3]>> = bones.iter().map(|b| Some(b.rotation)).collect();
    let l = locals(bones, std::slice::from_ref(&reference.to_vec()), &bind);
    let seed = l.iter().map(|c| c.first().map(|x| [x[3], x[4], x[5]])).collect();
    if let Ok(mut s) = OWN_SEED.write() {
        *s = Some(seed);
    }
}

fn own_seed(bones: &[Bone]) -> Vec<Option<[f32; 3]>> {
    OWN_SEED.read().ok().and_then(|s| s.clone()).filter(|s| s.len() == bones.len()).unwrap_or_else(|| bones.iter().map(|b| Some(b.rotation)).collect())
}

/// A KH2 motion (BAR type 9 data) for `poses` / `travel`, on `template` (a KH2 Sora motion).
pub fn write_motion(template: &[u8], kh2: &[Bone], poses: &[Vec<Mat4>], travel: &[Vec3]) -> Result<Vec<u8>> {
    write_motion_ex(template, kh2, poses, travel, false, None)
}

/// As `write_motion`, with each bone's rotation tolerance given (radians), and the channels at
/// the skeleton's bind left out, as KH2's own motions leave them (its Keyblade's keychain
/// lengths, each Keyblade's own, Checked (files)).
pub fn write_motion_tol(template: &[u8], kh2: &[Bone], poses: &[Vec<Mat4>], travel: &[Vec3], rot_tol: &[f32]) -> Result<Vec<u8>> {
    write_motion_ex(template, kh2, poses, travel, false, Some(rot_tol))
}

/// The rotation tolerance that moves a point `reach` away by `REACH_TOL`, within KH2's bounds.
pub fn reach_tolerance(reach: f32) -> f32 {
    if reach > 1e-3 { (REACH_TOL / reach).clamp(ROT_TOL / 2.0, ROT_TOL_MAX) } else { ROT_TOL_MAX }
}

/// As `write_motion`, for a pose taken from `template` itself (a KH2 motion moved onto another
/// skeleton): the template's root turn curves (root block channels 3..5) are kept, sampled a
/// frame at a time. KH2's guard (173) turns Sora a whole turn that way (its root y goes 0 → −2π
/// → 0, Checked (files)); our evaluator leaves it out of the poses.
pub fn write_motion_keep_turn(template: &[u8], kh2: &[Bone], poses: &[Vec<Mat4>], travel: &[Vec3]) -> Result<Vec<u8>> {
    write_motion_ex(template, kh2, poses, travel, true, None)
}

fn write_motion_ex(template: &[u8], kh2: &[Bone], poses: &[Vec<Mat4>], travel: &[Vec3], keep_turn: bool, given_tol: Option<&[f32]>) -> Result<Vec<u8>> {
    let h = BASE + 0x10;
    if i32_at(template, BASE)? != 0 {
        return Err(Error::Format("template is not an interpolated motion".into()));
    }
    let hdr = |o: usize| i32_at(template, h + o);
    let bone_count = (hdr(0)? & 0xFFFF) as usize;
    let total = ((hdr(0)? >> 16) & 0xFFFF) as usize;
    // A template of another skeleton (KH2 Sora's motion for KH1 Sora's bones) gives only the
    // header's fixed fields and the root block: no IK helpers, every bone in index order, and the
    // first frame's Euler forms seeded from the skeleton's bind (the same for every motion, so
    // KH2's blends between them stay short).
    let own = bone_count != kh2.len();
    if own && kh2.iter().enumerate().any(|(i, b)| b.parent >= i as i16) {
        return Err(Error::Format("skeleton bones must follow their parents".into()));
    }
    let frames = poses.len();
    let seed = if own { own_seed(kh2) } else { template_angles(template, kh2) };
    let loc = locals(kh2, poses, &seed);
    // For a new skeleton, each bone's rotation tolerance is what moves the farthest joint it
    // carries by `REACH_TOL` (a finger may turn more than the hips for the same error on screen),
    // within `ROT_TOL / 2` and `ROT_TOL_MAX`.
    let rot_tol: Vec<f32> = match given_tol {
        Some(t) if t.len() == kh2.len() => t.to_vec(),
        _ if own => rotation_tolerances(kh2),
        _ => vec![ROT_TOL; kh2.len()],
    };
    let mut initial: Vec<(i16, i16, f32)> = Vec::new();
    let mut forward: Vec<Curve> = Vec::new();
    for (b, chans) in loc.iter().enumerate() {
        for c in 0..9 {
            let v: Vec<f32> = chans.iter().map(|x| x[c]).collect();
            let tol = if c < 3 { 1e-4 } else if c < 6 { rot_tol[b] } else { POS_TOL };
            // A channel has either a starting value or a curve, never both (as in all of KH2's
            // own motions: KH2 holds a channel that has a starting value at it).
            if c < 3 || v.iter().all(|x| (x - v[0]).abs() <= tol) {
                // A channel left out plays the bone's bind value, as in KH2's own motions (A300
                // sets 196 of its 2,052). For a new skeleton, those at their bind are left out.
                let bind = kh2[b].scale.iter().chain(&kh2[b].rotation).chain(&kh2[b].translation).nth(c).copied().unwrap_or(0.0);
                if !((own || given_tol.is_some()) && (v[0] - bind).abs() <= 1e-5) {
                    initial.push((b as i16, c as i16, v[0]));
                }
                continue;
            }
            let keys = fit_hermite(&v, tol);
            forward.push(Curve { joint: b as i16, channel: c as u8, keys });
        }
    }
    let mut inverse: Vec<Curve> = Vec::new();
    let mut travel_ids = [-1i32; 3];
    for (axis, ch) in [(0usize, 6u8), (1, 7), (2, 8)] {
        let v: Vec<f32> = travel.iter().map(|t| t[axis]).collect();
        if v.iter().all(|x| x.abs() <= POS_TOL) {
            continue;
        }
        travel_ids[axis] = (forward.len() + inverse.len()) as i32;
        let keys = fit_hermite(&v, POS_TOL);
        inverse.push(Curve { joint: if own { OWN_TRAVEL_JOINT } else { TRAVEL_JOINT }, channel: ch, keys });
    }
    let mut turn_ids = [-1i32; 3];
    if keep_turn {
        if let (Ok(Some(t)), Some(ro)) = (kh2_formats::anim::Interpolated::parse(template), usize::try_from(hdr(0x1C)?).ok().filter(|&r| r > 0)) {
            let all: Vec<&kh2_formats::anim::FCurve> = t.forward.iter().chain(&t.inverse).collect();
            for k in 0..3 {
                let id = i32_at(template, BASE + ro + 0x30 + (3 + k) * 4)?;
                let Some(c) = usize::try_from(id).ok().and_then(|i| all.get(i)) else { continue };
                let t0 = 2.0 * f32::from_le_bytes(template[h + 0x80..h + 0x84].try_into().unwrap()).max(0.0);
                let v: Vec<f32> = (0..frames).map(|f| c.eval(t0 + f as f32)).collect();
                turn_ids[k] = (forward.len() + inverse.len()) as i32;
                let keys = fit_hermite(&v, ROT_TOL);
                inverse.push(Curve { joint: if own { OWN_TRAVEL_JOINT } else { TRAVEL_JOINT }, channel: (3 + k) as u8, keys });
            }
        }
    }

    // Tables: times (one a frame), values, one zero tangent.
    let times: Vec<f32> = (0..frames.max(1)).map(|f| f as f32).collect();
    let mut values: Vec<f32> = Vec::new();
    let mut tangents: Vec<f32> = Vec::new();
    let mut keys: Vec<[u8; 8]> = Vec::new();
    // Hermite keys (KH2's own kind, the only one its motions use). KH2's tangents are value per
    // key-time unit (checked: A300's equal its keys' slopes). A key's in and out tangent are one
    // entry; equal values and tangents are stored once (the tables are indexed by 16 bits).
    let mut value_at: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    let mut tangent_at: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    let mut fcurve_bytes = |list: &[Curve], keys: &mut Vec<[u8; 8]>, values: &mut Vec<f32>, tangents: &mut Vec<f32>| -> Result<Vec<u8>> {
        let mut out = Vec::new();
        for c in list {
            if c.keys.len() > 255 {
                return Err(Error::Format("a channel has more than 255 keys".into()));
            }
            let start = keys.len();
            for &(t, v, tan) in &c.keys {
                let vi = *value_at.entry(v.to_bits()).or_insert_with(|| {
                    values.push(v);
                    values.len() - 1
                });
                let li = *tangent_at.entry(tan.to_bits()).or_insert_with(|| {
                    tangents.push(tan);
                    tangents.len() - 1
                });
                let mut k = [0u8; 8];
                let tt = ((t as u16) << 2) | 2;
                k[0..2].copy_from_slice(&tt.to_le_bytes());
                k[2..4].copy_from_slice(&(vi as i16).to_le_bytes());
                k[4..6].copy_from_slice(&(li as i16).to_le_bytes());
                k[6..8].copy_from_slice(&(li as i16).to_le_bytes());
                keys.push(k);
            }
            out.extend_from_slice(&c.joint.to_le_bytes());
            out.push(c.channel & 0xF);
            out.push(c.keys.len() as u8);
            out.extend_from_slice(&(start as i16).to_le_bytes());
        }
        Ok(out)
    };
    let fwd_bytes = fcurve_bytes(&forward, &mut keys, &mut values, &mut tangents)?;
    let inv_bytes = fcurve_bytes(&inverse, &mut keys, &mut values, &mut tangents)?;
    if values.len() > i16::MAX as usize || keys.len() > i16::MAX as usize || tangents.len() > i16::MAX as usize {
        return Err(Error::Format(format!("{} values / {} keys: more than KH2's 16-bit indices hold", values.len(), keys.len())));
    }

    // The template's pieces we keep.
    let helper_count = if own { 2 } else { total.saturating_sub(bone_count) };
    let ik = BASE + hdr(0x08)? as usize;
    let ik_bytes = if own {
        // Two helpers of our own, past the new skeleton's bones: a root (no parent) and the
        // travel helper under it, which the travel curves set (`OWN_TRAVEL_JOINT`). KH2's own
        // helpers (11 to 53 a motion, KH2 Sora's IK targets) aren't read: no IK, no constraints.
        let mut b = Vec::new();
        for (i, parent) in [(0i32, -1i32), (1, kh2.len() as i32)] {
            for x in [kh2.len() as i32 + i, parent, 0, 0] {
                b.extend_from_slice(&x.to_le_bytes());
            }
            for x in [1.0f32, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] {
                b.extend_from_slice(&x.to_le_bytes());
            }
        }
        b
    } else {
        template.get(ik..ik + helper_count * 0x40).ok_or_else(|| Error::Format("template IK helpers".into()))?.to_vec()
    };
    let jt = BASE + hdr(0x0C)? as usize;
    let mut joint_bytes = if own {
        (0..(kh2.len() + helper_count) as i16).flat_map(|i| [i.to_le_bytes(), [0, 0]].concat()).collect()
    } else {
        template.get(jt..jt + total * 4).ok_or_else(|| Error::Format("template joints".into()))?.to_vec()
    };
    // No IK chains, constraint flags or external effectors: our motion has no constraints or IK
    // targets, so every joint is placed by its own curves (KH2 would otherwise pull the limbs to
    // the template's IK helpers at rest, and read constraints we don't have).
    for j in joint_bytes.chunks_mut(4) {
        j[2] = 0;
    }
    let ext_bytes: Vec<u8> = Vec::new();
    let ro = hdr(0x1C)?;
    if ro <= 0 {
        return Err(Error::Format("template has no root block".into()));
    }
    let ro = BASE + ro as usize;
    let mut root = template.get(ro..ro + 0x54).ok_or_else(|| Error::Format("template root block".into()))?.to_vec();
    // The root block (OpenKH `RootPosition`: scale, a word, rotation xyzw at 0x10, translation
    // xyzw at 0x20, then nine curve ids): no rotation and no translation of our own, the poses
    // carry everything. KH2's guard (173) has a root turn of -3.165 on y and a curve for it
    // (Checked (files)): kept, it turned the KH1 guard round by 180° (the owner, t42 / t44).
    root[0x10..0x1C].fill(0);
    root[0x20..0x2C].fill(0);
    for (i, id) in [-1i32, -1, -1, turn_ids[0], turn_ids[1], turn_ids[2], travel_ids[0], travel_ids[1], travel_ids[2]].iter().enumerate() {
        let id = if *id >= 0 { *id } else { -1 };
        root[0x30 + i * 4..0x34 + i * 4].copy_from_slice(&id.to_le_bytes());
    }

    // Lay it out (offsets from BASE), 16-byte aligned blocks.
    let mut body: Vec<u8> = template[..BASE + 0x10].to_vec();
    let mut header = template[h..h + 0xA0].to_vec();
    body.resize(BASE + 0xB0, 0);
    let place = |bytes: &[u8], body: &mut Vec<u8>| -> i32 {
        while body.len() % 16 != 0 {
            body.push(0);
        }
        let at = (body.len() - BASE) as i32;
        body.extend_from_slice(bytes);
        at
    };
    let pose_bytes: Vec<u8> = initial.iter().flat_map(|(j, c, v)| [j.to_le_bytes().to_vec(), c.to_le_bytes().to_vec(), v.to_le_bytes().to_vec()].concat()).collect();
    let o_pose = place(&pose_bytes, &mut body);
    let o_fwd = place(&fwd_bytes, &mut body);
    let o_inv = place(&inv_bytes, &mut body);
    let o_keys = place(&keys.concat(), &mut body);
    let o_times = place(&times.iter().flat_map(|t| t.to_le_bytes()).collect::<Vec<_>>(), &mut body);
    let o_vals = place(&values.iter().flat_map(|t| t.to_le_bytes()).collect::<Vec<_>>(), &mut body);
    let o_tans = place(&tangents.iter().flat_map(|t| t.to_le_bytes()).collect::<Vec<_>>(), &mut body);
    let o_ik = place(&ik_bytes, &mut body);
    let o_joints = place(&joint_bytes, &mut body);
    let o_root = place(&root, &mut body);
    let o_end = place(&[0u8; 16], &mut body);
    let o_ext = place(&ext_bytes, &mut body);
    while body.len() % 16 != 0 {
        body.push(0);
    }
    let set = |hd: &mut Vec<u8>, o: usize, v: i32| hd[o..o + 4].copy_from_slice(&v.to_le_bytes());
    if own {
        set(&mut header, 0x00, (kh2.len() as i32) | (((kh2.len() + helper_count) as i32) << 16));
    }
    set(&mut header, 0x04, frames as i32);
    set(&mut header, 0x08, o_ik);
    set(&mut header, 0x0C, o_joints);
    // The key-time table's length (OpenKH: KeyFrameCount). KH2 searches the times by it; A300's
    // own 27 left most of our key times past its end.
    set(&mut header, 0x10, times.len() as i32);
    set(&mut header, 0x14, o_pose);
    set(&mut header, 0x18, initial.len() as i32);
    set(&mut header, 0x1C, o_root);
    set(&mut header, 0x20, o_fwd);
    set(&mut header, 0x24, forward.len() as i32);
    set(&mut header, 0x28, o_inv);
    set(&mut header, 0x2C, inverse.len() as i32);
    set(&mut header, 0x30, o_keys);
    set(&mut header, 0x34, o_times);
    set(&mut header, 0x38, o_vals);
    set(&mut header, 0x3C, o_tans);
    for (o, v) in [(0x40, o_end), (0x44, 0), (0x48, o_end), (0x4C, o_end), (0x50, o_end), (0x54, 0), (0x58, o_end), (0x5C, 0)] {
        set(&mut header, o, v);
    }
    let fps = 30.0f32;
    let end = (frames.saturating_sub(1)) as f32 / 2.0;
    // The play range (OpenKH: start, end, fps, return, in 30 Hz frames). KH2 jumps back to
    // `return` at the end: a looping motion's is before its end (idle 0 of 150, walk 20 of
    // 20..56, fall 6 of 16; Checked (files)), a one-shot's is its end. Ours loops where the
    // template does, from the same share of the way through; with the return at the end a long
    // idle, walk or run froze on its last frame (the owner, t44).
    let tf = |o: usize| f32::from_le_bytes(template[h + o..h + o + 4].try_into().unwrap());
    let (t_start, t_end, t_ret) = (tf(0x80), tf(0x84), tf(0x8C));
    let ret = if t_ret < t_end - 1e-3 && t_end > t_start { ((t_ret - t_start).max(0.0) / (t_end - t_start) * end).clamp(0.0, end) } else { end };
    header[0x80..0x84].copy_from_slice(&0f32.to_le_bytes());
    header[0x84..0x88].copy_from_slice(&end.to_le_bytes());
    header[0x88..0x8C].copy_from_slice(&fps.to_le_bytes());
    header[0x8C..0x90].copy_from_slice(&ret.to_le_bytes());
    set(&mut header, 0x90, o_ext);
    body[h..h + 0xA0].copy_from_slice(&header);
    Ok(body)
}

/// Where a written motion goes back to at its end (`crate::return_at`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Return {
    /// As the template loops (`write_motion`'s own).
    Template,
    /// Held on its last frame (a one-shot).
    End,
    /// Back to this 60 Hz frame (a loop of the rest).
    Frame(usize),
}

/// Sets a written motion's return point (header `+0x8C`, 30 Hz frames, at most its end).
pub fn set_return_for(motion: &mut [u8], r: Return) {
    let h = BASE + 0x10;
    let Some(end) = motion.get(h + 0x84..h + 0x88).map(|b| f32::from_le_bytes(b.try_into().unwrap())) else { return };
    let ret = match r {
        Return::Template => return,
        Return::End => end,
        Return::Frame(f) => (f as f32 / 2.0).min(end),
    };
    motion[h + 0x8C..h + 0x90].copy_from_slice(&ret.to_le_bytes());
}

/// `original` (a BAR file) with `extra` entries added at the end. The original's data is kept
/// byte for byte, only moved down by the longer entry table, so entries that share data (KH2 marks
/// them `duplicate` and relocates the data once) still share it. An extra entry with `None` data
/// is a `DUMM` slot pointing at `dummy_at` (an original entry's data, as KH2's own `DUMM`s do).
pub fn extend_bar(original: &[u8], extra: &[(u16, i16, [u8; 4], Option<Vec<u8>>)]) -> Result<Vec<u8>> {
    let n = i32_at(original, 4)?.max(0) as usize;
    let table_end = 16 + n * 16;
    if original.get(0..4) != Some(&b"BAR\x01"[..]) || original.len() < table_end {
        return Err(Error::Format("not a BAR file".into()));
    }
    let shift = extra.len() * 16;
    let mut out = original[..16].to_vec();
    out[4..8].copy_from_slice(&((n + extra.len()) as i32).to_le_bytes());
    let mut dumm_at = None;
    for i in 0..n {
        let e = &original[16 + i * 16..32 + i * 16];
        let mut row = e.to_vec();
        let off = i32_at(e, 8)? as usize + shift;
        row[8..12].copy_from_slice(&(off as i32).to_le_bytes());
        if &e[4..8] == b"DUMM" && dumm_at.is_none() {
            dumm_at = Some((u16::from_le_bytes([e[0], e[1]]), off));
        }
        out.extend_from_slice(&row);
    }
    out.resize(table_end + shift, 0);
    out.extend_from_slice(&original[table_end..]);
    let (dumm_kind, dumm_off) = dumm_at.ok_or_else(|| Error::Format("the BAR has no DUMM slot to copy".into()))?;
    for (i, (kind, dup, name, data)) in extra.iter().enumerate() {
        let e = 16 + (n + i) * 16;
        let (kind, dup, off, size) = match data {
            None => (dumm_kind, 1i16, dumm_off, 0usize),
            Some(d) => {
                while out.len() % 16 != 0 {
                    out.push(0);
                }
                let at = out.len();
                out.extend_from_slice(d);
                (*kind, *dup, at, d.len())
            }
        };
        out[e..e + 2].copy_from_slice(&kind.to_le_bytes());
        out[e + 2..e + 4].copy_from_slice(&dup.to_le_bytes());
        out[e + 4..e + 8].copy_from_slice(name);
        out[e + 8..e + 12].copy_from_slice(&(off as i32).to_le_bytes());
        out[e + 12..e + 16].copy_from_slice(&(size as i32).to_le_bytes());
    }
    while out.len() % 16 != 0 {
        out.push(0);
    }
    Ok(out)
}

/// A BAR file from entries (`kind`, `duplicate`, name, data).
pub fn write_bar(motionset: i32, entries: &[(u16, i16, [u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0x0152_4142u32.to_le_bytes());
    out.extend_from_slice(&(entries.len() as i32).to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes());
    out.extend_from_slice(&motionset.to_le_bytes());
    let table = out.len();
    out.resize(table + entries.len() * 16, 0);
    for (i, (kind, dup, name, data)) in entries.iter().enumerate() {
        while out.len() % 16 != 0 {
            out.push(0);
        }
        let at = out.len();
        out.extend_from_slice(data);
        let e = table + i * 16;
        out[e..e + 2].copy_from_slice(&kind.to_le_bytes());
        out[e + 2..e + 4].copy_from_slice(&dup.to_le_bytes());
        out[e + 4..e + 8].copy_from_slice(name);
        let (off, size) = if data.is_empty() { (at as i32, 0) } else { (at as i32, data.len() as i32) };
        out[e + 8..e + 12].copy_from_slice(&off.to_le_bytes());
        out[e + 12..e + 16].copy_from_slice(&size.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hermite_keys_stay_within_tolerance() {
        let v: Vec<f32> = (0..90).map(|f| (f as f32 * 0.11).sin() * 2.0 + if f > 60 { 0.5 } else { 0.0 }).collect();
        let k = fit_hermite(&v, 0.002);
        assert!(k.len() < 45, "{} keys", k.len());
        for f in 0..v.len() {
            let i = k.partition_point(|x| x.0 <= f) - 1;
            let got = if k[i].0 == f { k[i].1 } else {
                let (a, b) = (k[i], k[i + 1]);
                let dt = (b.0 - a.0) as f32;
                let n = (f - a.0) as f32 / dt;
                (2.0 * n * n * n - 3.0 * n * n + 1.0) * a.1 + (-2.0 * n * n * n + 3.0 * n * n) * b.1 + (n * n * n - 2.0 * n * n + n) * a.2 * dt + (n * n * n - n * n) * b.2 * dt
            };
            assert!((got - v[f]).abs() <= 0.002 + 1e-6, "frame {f}");
        }
    }

    #[test]
    fn euler_round_trip() {
        let q = kh2_anim::euler_quat(Vec3::new(0.3, -1.1, 2.0));
        let e = euler_near(q, None);
        let back = kh2_anim::euler_quat(Vec3::from(e));
        assert!(back.angle_between(q) < 1e-4);
        let e2 = euler_near(q, Some([0.3 + 6.283185, -1.1, 2.0]));
        assert!((e2[0] - (0.3 + 6.283185)).abs() < 1e-3);
    }

    #[test]
    fn bar_round_trip() {
        let b = write_bar(0, &[(17, 0, *b"A300", vec![1, 2, 3]), (17, 0, *b"DUMM", vec![])]);
        let p = kh2_formats::bar::Bar::parse(&b).unwrap();
        assert_eq!(p.entries.len(), 2);
        assert_eq!(p.entries[0].data, &[1, 2, 3]);
        assert!(p.entries[1].data.is_empty());
    }
}

/// `original` (a BAR file) with each entry's data passed through `f`, entries that shared data
/// (the same offset: KH2's `duplicate` slots and `DUMM`s) still sharing the converted data.
pub fn map_bar(original: &[u8], f: impl FnMut(usize, u16, &[u8]) -> Result<Vec<u8>>) -> Result<Vec<u8>> {
    map_bar_ex(original, &[], false, f)
}

/// As `map_bar`, and each empty row of `fills` (row, template row) gets an entry of its own: `f`
/// run on the template row's data for that row, with the template's kind and name, not a
/// duplicate (KH2 skips entries marked duplicate, Stage 14).
pub fn map_bar_filled(original: &[u8], fills: &[(usize, usize)], f: impl FnMut(usize, u16, &[u8]) -> Result<Vec<u8>>) -> Result<Vec<u8>> {
    map_bar_ex(original, fills, false, f)
}

/// As `map_bar_filled`, but each filled row shares its template row's (new) data, marked a
/// duplicate as KH2's own shared entries are (its DUMMs all share one block): no bytes added.
pub fn map_bar_shared(original: &[u8], fills: &[(usize, usize)], f: impl FnMut(usize, u16, &[u8]) -> Result<Vec<u8>>) -> Result<Vec<u8>> {
    map_bar_ex(original, fills, true, f)
}

fn map_bar_ex(original: &[u8], fills: &[(usize, usize)], share: bool, mut f: impl FnMut(usize, u16, &[u8]) -> Result<Vec<u8>>) -> Result<Vec<u8>> {
    let n = i32_at(original, 4)?.max(0) as usize;
    let table_end = 16 + n * 16;
    if original.get(0..4) != Some(&b"BAR\x01"[..]) || original.len() < table_end {
        return Err(Error::Format("not a BAR file".into()));
    }
    let rows: Vec<&[u8]> = (0..n).map(|i| &original[16 + i * 16..32 + i * 16]).collect();
    let mut out = original[..table_end].to_vec();
    let mut placed: std::collections::HashMap<usize, (usize, usize)> = std::collections::HashMap::new();
    for (i, r) in rows.iter().enumerate() {
        let (off, size) = (i32_at(r, 8)? as usize, i32_at(r, 12)?.max(0) as usize);
        if size == 0 || placed.contains_key(&off) {
            continue;
        }
        let data = original.get(off..off + size).ok_or_else(|| Error::Format(format!("BAR entry {i} out of bounds")))?;
        let new = f(i, u16::from_le_bytes([r[0], r[1]]), data)?;
        while out.len() % 16 != 0 {
            out.push(0);
        }
        placed.insert(off, (out.len(), new.len()));
        out.extend_from_slice(&new);
    }
    let mut filled: std::collections::HashMap<usize, (usize, usize, usize)> = std::collections::HashMap::new();
    for &(row, t) in fills {
        let (Some(r), Some(tr)) = (rows.get(row), rows.get(t)) else { continue };
        if i32_at(r, 12)? > 0 {
            continue;
        }
        let (off, size) = (i32_at(tr, 8)? as usize, i32_at(tr, 12)?.max(0) as usize);
        if size == 0 {
            continue;
        }
        if share {
            if let Some(&(at, len)) = placed.get(&off) {
                filled.insert(row, (at, len, t));
            }
            continue;
        }
        let data = original.get(off..off + size).ok_or_else(|| Error::Format(format!("BAR entry {t} out of bounds")))?;
        let new = f(row, u16::from_le_bytes([tr[0], tr[1]]), data)?;
        while out.len() % 16 != 0 {
            out.push(0);
        }
        filled.insert(row, (out.len(), new.len(), t));
        out.extend_from_slice(&new);
    }
    let first = placed.values().map(|p| p.0).min().unwrap_or(table_end);
    for (i, r) in rows.iter().enumerate() {
        let e = 16 + i * 16;
        if let Some(&(at, len, t)) = filled.get(&i) {
            let tr = 16 + t * 16;
            let (kind, name) = (original[tr..tr + 2].to_vec(), original[tr + 4..tr + 8].to_vec());
            out[e..e + 2].copy_from_slice(&kind);
            out[e + 2..e + 4].copy_from_slice(&(share as u16).to_le_bytes());
            out[e + 4..e + 8].copy_from_slice(&name);
            out[e + 8..e + 12].copy_from_slice(&(at as i32).to_le_bytes());
            out[e + 12..e + 16].copy_from_slice(&(len as i32).to_le_bytes());
            continue;
        }
        let (off, size) = (i32_at(r, 8)? as usize, i32_at(r, 12)?.max(0) as usize);
        let (at, len) = match placed.get(&off) {
            Some(&(at, len)) => (at, if size == 0 { 0 } else { len }),
            None => (first, 0),
        };
        out[e + 8..e + 12].copy_from_slice(&(at as i32).to_le_bytes());
        out[e + 12..e + 16].copy_from_slice(&(len as i32).to_le_bytes());
    }
    while out.len() % 16 != 0 {
        out.push(0);
    }
    Ok(out)
}
