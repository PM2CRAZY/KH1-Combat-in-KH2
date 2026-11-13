//! KH1 Sora's model as a KH2 model file (`obj/P_EX100.mdlx`): KH1's own skeleton (293 bones),
//! mesh and textures, written in KH2's format so KH2 draws, animates and collides him with its
//! own model code. Everything else in the file (the collision list's shapes) stays KH2's; only
//! the bone numbers KH2 reads from the file are moved onto the matching KH1 bones.
//!
//! The HD textures are KH1's own (`remastered/xa_ex_0010.mdls/-<offset>.dds`), handed out by
//! the caller as `remastered/obj/P_EX100.mdlx/-<n>.dds` in texture order.

use kh2_formats::{Error, Result};
use kh1_data::mdls;
use kh2_formats::anim::Bone;
use kh2_formats::bar::Bar;
use kh2_formats::mdlx::{Group, Vertex};
use kh2_formats::mdlx_write::{write_model, write_textures, IndexedTexture};

const BAR_MODEL: u16 = 4;
const BAR_TEXTURES: u16 = 7;
const BAR_COLLISION: u16 = 23;
/// A collision entry's "no bone".
const NO_BONE: i16 = 16384;

/// KH2 Sora's bones the file names (the skeleton data's body parts and the collision list),
/// each with the KH1 Sora bone at the same place. KH1 per side: hips h, h+1; knee h+2; ankle
/// h+3..h+5; heel h+6; toes h+7, h+8 (h = 42 / 16); collarbone c, c+1; shoulder c+2..c+4; elbow
/// c+5; wrist c+6..c+8 (c = 194 / 89). Checked: `bone_map_lands_on_kh1s_same_places`.
pub const BONE_MAP: [(i16, i16); 23] = [
    (140, 149), // head
    (137, 144), // neck
    (67, 77),   // chest
    (2, 2),     // hips
    (75, 195),  // collarbone +x
    (167, 90),
    (78, 197), // upper arm
    (170, 92),
    (79, 199), // forearm (elbow)
    (171, 94),
    (85, 202), // hand
    (177, 97),
    (11, 42), // thigh
    (40, 16),
    (12, 44), // shin (knee)
    (41, 18),
    (15, 46), // foot
    (44, 20),
    (18, 50), // toes
    (47, 24),
    (0, 0),
    (1, 1),
    (-1, -1),
];

/// KH1's right and left palms: KH2 hangs a weapon on the bones of its objentry's `sklt` row (KH2
/// Sora's row 1: 178 / 86, his palms, Checked (files)). With KH1's model, P_EX100 gets a row of
/// ours with these (`kh2-hook` `weapon_hold`), so the Keyblade hangs on KH1's palm, where KH1's
/// own grip bone (280, the Keyblade's root) hangs (Checked (files): 280's parent is 101; 206 the
/// left palm, the parent of the left fingers).
pub const PALMS: [i16; 2] = [101, 206];

/// KH1's skeleton as the model's.
pub fn model_bones(kh1: &[Bone], _kh2: &[Bone]) -> Vec<Bone> {
    kh1.to_vec()
}

pub fn kh1_bone(kh2: i16) -> Option<i16> {
    BONE_MAP.iter().find(|m| m.0 == kh2).map(|m| m.1)
}

/// KH1's mesh as KH2 groups (one per KH1 mesh, its texture kept), triangles in `winding`.
pub fn groups(meshes: &[mdls::Mesh], flip: bool) -> Vec<Group> {
    meshes
        .iter()
        .map(|m| Group {
            attributes: 0,
            texture: m.texture as usize,
            triangles: m
                .triangles
                .iter()
                .map(|t| {
                    let v = |i: u32| {
                        let v = m.vertices[i as usize];
                        Vertex { position: v.position, bone: v.joint, uv: v.uv }
                    };
                    if flip { [v(t[0]), v(t[2]), v(t[1])] } else { [v(t[0]), v(t[1]), v(t[2])] }
                })
                .collect(),
        })
        .collect()
}

fn remap_skeleton_data(kh2: &[u8]) -> Result<[u8; 0x120]> {
    let mut s: [u8; 0x120] = kh2.try_into().map_err(|_| Error::Format("skeleton data is not 0x120 bytes".into()))?;
    for k in 0..56 {
        let at = 0x30 + k * 4;
        let b = i32::from_le_bytes(s[at..at + 4].try_into().unwrap()) as i16;
        let n = kh1_bone(b).ok_or_else(|| Error::Format(format!("KH2 body-part bone {b} has no KH1 bone")))?;
        s[at..at + 4].copy_from_slice(&(n as i32).to_le_bytes());
    }
    Ok(s)
}

fn remap_collision(kh2: &[u8]) -> Result<Vec<u8>> {
    let mut c = kh2.to_vec();
    let n = u32::from_le_bytes(c[0..4].try_into().unwrap()) as usize;
    for k in 0..n {
        let at = 0x40 + k * 0x14 + 6;
        let b = i16::from_le_bytes(c[at..at + 2].try_into().unwrap());
        if b != NO_BONE {
            let m = kh1_bone(b).ok_or_else(|| Error::Format(format!("KH2 collision bone {b} has no KH1 bone")))?;
            c[at..at + 2].copy_from_slice(&m.to_le_bytes());
        }
    }
    Ok(c)
}

/// The BAR with KH2's alignment for model files (0x80 between entries, as `P_EX100.mdlx`).
fn write_bar(motionset: i32, entries: &[(u16, i16, [u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    for x in [0x0152_4142u32, entries.len() as u32, 0, motionset as u32] {
        out.extend_from_slice(&x.to_le_bytes());
    }
    let table = out.len();
    out.resize(table + entries.len() * 16, 0);
    for (i, (kind, dup, name, data)) in entries.iter().enumerate() {
        if i > 0 {
            out.resize(out.len().next_multiple_of(0x80), 0);
        }
        let at = out.len();
        out.extend_from_slice(data);
        let e = table + i * 16;
        out[e..e + 2].copy_from_slice(&kind.to_le_bytes());
        out[e + 2..e + 4].copy_from_slice(&dup.to_le_bytes());
        out[e + 4..e + 8].copy_from_slice(name);
        out[e + 8..e + 12].copy_from_slice(&(at as u32).to_le_bytes());
        out[e + 12..e + 16].copy_from_slice(&(data.len() as u32).to_le_bytes());
    }
    out
}

/// KH1 Sora (`xa_ex_0010.mdls`) as `P_EX100.mdlx`, from KH2's own `P_EX100.mdlx`.
pub fn build(kh1_mdls: &[u8], kh2_mdlx: &[u8]) -> Result<Vec<u8>> {
    let bones: Vec<Bone> = model_bones(&mdls::parse_skeleton(kh1_mdls)?, &kh2_formats::anim::parse_skeleton(kh2_mdlx)?);
    let (meshes, textures) = mdls::parse_mesh(kh1_mdls)?;
    let bar = Bar::parse(kh2_mdlx)?;
    let mut entries = Vec::new();
    for e in &bar.entries {
        let mut name = [0u8; 4];
        name[..e.name.len().min(4)].copy_from_slice(&e.name.as_bytes()[..e.name.len().min(4)]);
        let data = match e.kind {
            BAR_MODEL => {
                let skel_at = 0x90 + u32::from_le_bytes(e.data[0x90 + 0x18..0x90 + 0x1C].try_into().unwrap()) as usize;
                let skel = remap_skeleton_data(&e.data[skel_at..skel_at + 0x120])?;
                write_model(&bones, &groups(&meshes, false), textures.len() as u16, &skel)
            }
            BAR_TEXTURES => {
                let t: Vec<IndexedTexture> = textures.iter().map(|t| IndexedTexture { width: t.width, height: t.height, pixels: t.pixels.clone(), palette: t.palette.clone() }).collect();
                write_textures(&t, false)?
            }
            BAR_COLLISION => remap_collision(e.data)?,
            _ => e.data.to_vec(),
        };
        entries.push((e.kind, e.duplicate, name, data));
    }
    Ok(write_bar(bar.motionset, &entries))
}

/// KH2 Sora's motion set on KH1 Sora's skeleton: every KH2 motion posed by KH2's own evaluator,
/// moved onto KH1's bones (`kh1_data::retarget`), and written for the 293-bone skeleton; in the
/// swing slots (`crate::SWINGS`) KH1's combo motions instead, posed by KH1's own bone pass
/// (`kh1_data::rig`), as KH1 plays them. Returns the set and the swings written (KH1 motion, KH2
/// id, frames). With `kh1_swings` false (`combos = kh2`) the swings are KH2's, converted too.
pub fn build_mset(kh2_mset: &[u8], kh2: &[Bone], kh1: &[Bone], kh1_motions: &[Option<kh1_data::mset::Motion>], kh1_swings: bool, spans: &std::collections::HashMap<usize, (i16, i16)>) -> Result<(Vec<u8>, Vec<(usize, usize, usize)>)> {
    use crate::convert::{kh2_round_trip, map_bar, write_motion};
    use kh2_formats::motion::{BAR_TYPE_ANB, BAR_TYPE_MOTION};
    let mut swings = Vec::new();
    let mb = model_bones(kh1, kh2);
    // Every motion's first Euler forms from KH1's battle idle (motion 1, frame 0).
    if let Some(idle) = kh1_motions.get(1).and_then(|m| m.as_ref()) {
        crate::convert::set_own_seed(&mb, &kh1_data::rig::pose(idle, kh1, 0.0).bones);
    }
    let out = crate::convert::map_bar_filled(kh2_mset, &crate::fills(), |row, kind, data| {
        if kind != BAR_TYPE_ANB || data.is_empty() {
            return Ok(data.to_vec());
        }
        let swing = crate::kh1_motion_at(row).filter(|_| kh1_swings);
        let len = crate::swing_triggers::anb_frames(data);
        map_bar(data, |_, k, m| {
            if let (Some(s), kh2_formats::motion::BAR_TYPE_MOTION_TRIGGERS) = (swing, k) {
                if let (Some(motion), false) = (kh1_motions.get(s).and_then(|x| x.as_ref()), m.is_empty()) {
                    return Ok(crate::swing_triggers::sora(&kh2_formats::motion::MotionTriggers::parse(m)?, spans.get(&row).copied(), len, motion).write());
                }
            }
            if k != BAR_TYPE_MOTION || !matches!(kh2_formats::anim::Interpolated::parse(m), Ok(Some(_))) {
                return Ok(m.to_vec());
            }
            if let Some(k) = swing {
                let motion = kh1_motions.get(k).cloned().flatten().ok_or_else(|| Error::NotFound(format!("KH1 motion {k}")))?;
                let frames = motion.frames.max(1) as usize;
                let (mut poses, mut travel) = (Vec::with_capacity(frames), Vec::with_capacity(frames));
                // KH1's motion before this one in the same row (`crate::PRELUDE`), then this one,
                // its travel on from where the first left him.
                let pre = crate::prelude_at(row).and_then(|n| kh1_motions.get(n).and_then(|m| m.as_ref()));
                let mut order: Vec<&kh1_data::mset::Motion> = pre.into_iter().collect();
                order.push(&motion);
                let mut base = glam::Vec3::ZERO;
                // The travel stays in the bones, as in KH2's own motions (`convert::kh2_poses`).
                for mo in order {
                    let n = mo.frames.max(1) as usize;
                    let mut last = glam::Vec3::ZERO;
                    for f in 0..n {
                        let p = kh1_data::rig::pose(mo, kh1, f as f32);
                        let t = p.helpers.last().map(|x| x.w_axis.truncate()).unwrap_or(glam::Vec3::ZERO);
                        let on = glam::Mat4::from_translation(base);
                        poses.push(p.bones.iter().map(|m| on * *m).collect());
                        travel.push(base + t);
                        last = t;
                    }
                    base += last;
                }
                let pre_frames = pre.map(|m| m.frames.max(1) as usize).unwrap_or(0);
                // KH1's blend into the next motion, where KH2 doesn't blend (`crate::TAILS`).
                if let Some(next) = crate::tail_at(row).and_then(|n| kh1_motions.get(n).and_then(|m| m.as_ref())) {
                    let p = kh1_data::rig::pose(next, kh1, 0.0);
                    let t0 = p.helpers.last().map(|x| x.w_axis.truncate()).unwrap_or(glam::Vec3::ZERO);
                    crate::convert::append_blend(&mut poses, &mut travel, &p.bones, t0, &mb, crate::KH1_BLEND);
                }
                let mut ret = crate::return_at(row, pre_frames);
                // The looping motion after it (`crate::POSTLUDE`), from where he is.
                if let Some(pm) = crate::postlude_at(row).and_then(|n| kh1_motions.get(n).and_then(|m| m.as_ref())) {
                    let base = travel.last().copied().unwrap_or(glam::Vec3::ZERO);
                    ret = crate::convert::Return::Frame(poses.len());
                    for f in 0..pm.frames.max(1) as usize {
                        let p = kh1_data::rig::pose(pm, kh1, f as f32);
                        let t = p.helpers.last().map(|x| x.w_axis.truncate()).unwrap_or(glam::Vec3::ZERO);
                        let on = glam::Mat4::from_translation(base);
                        poses.push(p.bones.iter().map(|m| on * *m).collect());
                        travel.push(base + t);
                    }
                }
                swings.push((k, row / 4, frames));
                let mut out = write_motion(m, &mb, &poses, &travel)?;
                crate::convert::set_return_for(&mut out, ret);
                return Ok(out);
            }
            let (poses, travel) = kh2_round_trip(m, kh2)?;
            // A fresh retarget a motion: its cloth and turn memory start from rest, not from
            // wherever the motion before it left them.
            let r = kh1_data::retarget::Retarget::new(kh1, kh2);
            // KH2's bones carry the travel; it comes off for the transfer (which scales the hips'
            // place by the two Soras' heights) and goes back on after.
            let kh1_poses: Vec<_> = poses
                .iter()
                .zip(&travel)
                .map(|(p, t)| {
                    let (off, on) = (glam::Mat4::from_translation(-*t), glam::Mat4::from_translation(*t));
                    let body: Vec<_> = p.iter().map(|m| off * *m).collect();
                    r.pose(&body).into_iter().map(|m| on * m).collect::<Vec<_>>()
                })
                .collect();
            crate::convert::write_motion_keep_turn(m, &mb, &kh1_poses, &travel)
        })
    })?;
    if kh1_swings && swings.len() != crate::kh1_slots().len() {
        return Err(Error::Format(format!("KH2's motion set had {} of the {} KH1 slots", swings.len(), crate::kh1_slots().len())));
    }
    Ok((out, swings))
}

/// The stances both sources give, to pick the hidden bones they turn differently: (KH1 motion,
/// KH2 motion set row) at their first and middle frames: battle idle, field idle, run, fall.
const CANON_PAIRS: [(usize, usize); 4] = [(1, 0), (0, 2), (7, 8), (10, 18)];

/// How far apart (degrees, against the parent) a hidden bone may be turned by KH1's rig and by
/// our transfer before it gets the one rule.
pub const CANON_DEG: f32 = 30.0;

/// KH1's model: which bones keep their turn and the reference pose (KH1's battle idle) for
/// `convert::set_canon`. Kept: the mesh's bones, the palms (the Keyblade's), and every hidden bone KH1's rig and the transfer (`kh1_data::retarget`) turn alike within
/// `CANON_DEG` in the `CANON_PAIRS` stances.
pub fn canon_for_kh1_model(mdls: &[u8], kh1: &[Bone], kh2: &[Bone], kh1_motions: &[Option<kh1_data::mset::Motion>], kh2_mset: &[u8]) -> Option<(Vec<bool>, Vec<glam::Mat4>)> {
    let mut keep = vec![false; kh1.len()];
    for m in kh1_data::mdls::parse_mesh(mdls).ok()?.0 {
        for v in m.vertices {
            if let Some(k) = keep.get_mut(v.joint as usize) {
                *k = true;
            }
        }
    }
    // (The bones KH2's skeleton data and collision list point at, `BONE_MAP`, keep their places
    // either way; their offsets were KH2's bones' and are no truer on one KH1 orientation than another.)
    for b in PALMS {
        if let Some(k) = keep.get_mut(b as usize) {
            *k = true;
        }
    }
    let r = kh1_data::retarget::Retarget::new(kh1, kh2);
    let mut pairs: Vec<(Vec<glam::Mat4>, Vec<glam::Mat4>)> = Vec::new();
    for (k1, row) in CANON_PAIRS {
        let (Some(m1), Some(m2)) = (kh1_motions.get(k1).and_then(|m| m.as_ref()), crate::convert::row_motion(kh2_mset, row)) else { continue };
        let Ok((p2, t2)) = crate::convert::kh2_round_trip(&m2, kh2) else { continue };
        for (f1, f2) in [(0, 0), (m1.frames.max(1) as usize / 2, p2.len() / 2)] {
            let a = kh1_data::rig::pose(m1, kh1, f1 as f32).bones;
            let off = glam::Mat4::from_translation(-t2[f2]);
            let b = r.pose(&p2[f2].iter().map(|m| off * *m).collect::<Vec<_>>());
            pairs.push((a, b));
        }
    }
    let refs: Vec<(&[glam::Mat4], &[glam::Mat4])> = pairs.iter().map(|(a, b)| (a.as_slice(), b.as_slice())).collect();
    crate::convert::keep_alike(kh1, &mut keep, &refs, CANON_DEG);
    let reference = kh1_data::rig::pose(kh1_motions.get(1)?.as_ref()?, kh1, 0.0).bones;
    Some((keep, reference))
}

/// KH2's model: the same for our KH1 motions on KH2's skeleton (`convert::kh2_poses`) against KH2's
/// own: kept the mesh's bones (`kh2_formats::mdlx::bones_used`), the weapon's palms (178 / 86) and
/// the hidden bones turned alike; the reference KH2's battle idle (row 0).
pub fn canon_for_kh2_model(mdlx: &[u8], kh1: &[Bone], kh2: &[Bone], kh1_motions: &[Option<kh1_data::mset::Motion>], kh2_mset: &[u8]) -> Option<(Vec<bool>, Vec<glam::Mat4>)> {
    let mut keep = kh2_formats::mdlx::bones_used(mdlx).ok()?;
    for b in [178usize, 86] {
        if let Some(k) = keep.get_mut(b) {
            *k = true;
        }
    }
    let mut pairs: Vec<(Vec<glam::Mat4>, Vec<glam::Mat4>)> = Vec::new();
    let mut reference = None;
    for (k1, row) in CANON_PAIRS {
        let (Some(m1), Some(m2)) = (kh1_motions.get(k1).and_then(|m| m.as_ref()), crate::convert::row_motion(kh2_mset, row)) else { continue };
        let Ok((p2, t2)) = crate::convert::kh2_round_trip(&m2, kh2) else { continue };
        let (p1, t1) = crate::convert::kh2_poses(m1, kh1, kh2);
        if row == 0 {
            reference = p2.first().cloned();
        }
        for (f1, f2) in [(0, 0), (p1.len() / 2, p2.len() / 2)] {
            let (o1, o2) = (glam::Mat4::from_translation(-t1[f1]), glam::Mat4::from_translation(-t2[f2]));
            pairs.push((p2[f2].iter().map(|m| o2 * *m).collect(), p1[f1].iter().map(|m| o1 * *m).collect()));
        }
    }
    let refs: Vec<(&[glam::Mat4], &[glam::Mat4])> = pairs.iter().map(|(a, b)| (a.as_slice(), b.as_slice())).collect();
    crate::convert::keep_alike(kh2, &mut keep, &refs, CANON_DEG);
    Some((keep, reference?))
}
