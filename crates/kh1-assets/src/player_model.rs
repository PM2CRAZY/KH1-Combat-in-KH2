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
    let bones: Vec<Bone> = mdls::parse_skeleton(kh1_mdls)?;
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
    let out = map_bar(kh2_mset, |row, kind, data| {
        if kind != BAR_TYPE_ANB || data.is_empty() {
            return Ok(data.to_vec());
        }
        let swing = crate::kh1_motion_at(row).filter(|_| kh1_swings);
        let len = crate::swing_triggers::anb_frames(data);
        map_bar(data, |_, k, m| {
            if let (Some(s), kh2_formats::motion::BAR_TYPE_MOTION_TRIGGERS) = (swing, k) {
                if let (Some(motion), false) = (kh1_motions.get(s).and_then(|x| x.as_ref()), m.is_empty()) {
                    return Ok(crate::swing_triggers::sora(&kh2_formats::motion::MotionTriggers::parse(m)?, spans.get(&(row / 4)).copied(), len, motion).write());
                }
            }
            if k != BAR_TYPE_MOTION || !matches!(kh2_formats::anim::Interpolated::parse(m), Ok(Some(_))) {
                return Ok(m.to_vec());
            }
            if let Some(k) = swing {
                let motion = kh1_motions.get(k).cloned().flatten().ok_or_else(|| Error::NotFound(format!("KH1 motion {k}")))?;
                let frames = motion.frames.max(1) as usize;
                let (mut poses, mut travel) = (Vec::with_capacity(frames), Vec::with_capacity(frames));
                for f in 0..frames {
                    let p = kh1_data::rig::pose(&motion, kh1, f as f32);
                    let t = p.helpers.last().map(|x| x.w_axis.truncate()).unwrap_or(glam::Vec3::ZERO);
                    let back = glam::Mat4::from_translation(-t);
                    poses.push(p.bones.iter().map(|x| back * *x).collect::<Vec<_>>());
                    travel.push(t);
                }
                swings.push((k, row / 4, frames));
                return write_motion(m, kh1, &poses, &travel);
            }
            let (poses, travel) = kh2_round_trip(m, kh2)?;
            // A fresh retarget a motion: its cloth and turn memory start from rest, not from
            // wherever the motion before it left them.
            let r = kh1_data::retarget::Retarget::new(kh1, kh2);
            let kh1_poses: Vec<_> = poses.iter().map(|p| r.pose(p)).collect();
            write_motion(m, kh1, &kh1_poses, &travel)
        })
    })?;
    if kh1_swings && swings.len() != crate::kh1_slots().len() {
        return Err(Error::Format(format!("KH2's motion set had {} of the {} KH1 slots", swings.len(), crate::kh1_slots().len())));
    }
    Ok((out, swings))
}
