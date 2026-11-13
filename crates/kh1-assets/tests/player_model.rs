//! KH1 Sora as a KH2 model. Needs `KH1_KH2_FILES` (xa_ex_0010.mdls, P_EX100.mdlx).

use glam::Vec3;
use kh1_assets::player_model;

fn files() -> Option<(Vec<u8>, Vec<u8>)> {
    let dir = std::path::PathBuf::from(std::env::var("KH1_KH2_FILES").ok()?);
    Some((std::fs::read(dir.join("xa_ex_0010.mdls")).ok()?, std::fs::read(dir.join("P_EX100.mdlx")).ok()?))
}

fn pos(m: &[glam::Mat4]) -> Vec<Vec3> {
    m.iter().map(|m| m.w_axis.truncate()).collect()
}

#[test]
fn bone_map_lands_on_kh1s_same_places() {
    let Some((mdls, mdlx)) = files() else { return };
    let kh1 = kh1_data::mdls::parse_skeleton(&mdls).unwrap();
    let kh2 = kh2_formats::anim::parse_skeleton(&mdlx).unwrap();
    let p1 = pos(&kh2_anim::Pose::bind(&kh1).bones);
    let p2 = pos(&kh2_anim::Pose::bind(&kh2).bones);
    let h1 = p1.iter().map(|p| p.y).fold(0.0, f32::max);
    let h2 = p2.iter().map(|p| p.y).fold(0.0, f32::max);
    eprintln!("heights KH1 {h1:.1} KH2 {h2:.1}");
    for (a, b) in player_model::BONE_MAP {
        if a < 0 {
            continue;
        }
        let (x, y) = (p2[a as usize] * (h1 / h2), p1[b as usize]);
        eprintln!("KH2 {a:3} -> KH1 {b:3}: KH2 {:6.1?} KH1 {:6.1?} off {:.1}", x.to_array(), y.to_array(), (x - y).length());
        assert!((x - y).length() < 0.12 * h1, "KH2 bone {a} and KH1 bone {b} are far apart");
    }
}

#[test]
fn kh1_mesh_winding_and_uvs() {
    let Some((mdls, mdlx)) = files() else { return };
    let (meshes, textures) = kh1_data::mdls::parse_mesh(&mdls).unwrap();
    let (mut umin, mut umax) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    let (mut agree, mut total) = (0, 0);
    for m in &meshes {
        for t in &m.triangles {
            let v: Vec<_> = t.iter().map(|&i| m.vertices[i as usize]).collect();
            for x in &v {
                let u = Vec3::new(x.uv[0], x.uv[1], 0.0);
                umin = umin.min(u);
                umax = umax.max(u);
            }
            if v.iter().all(|x| x.joint == v[0].joint) {
                let p: Vec<Vec3> = v.iter().map(|x| Vec3::from(x.position)).collect();
                let n = (p[1] - p[0]).cross(p[2] - p[0]);
                let vn: Vec3 = v.iter().map(|x| Vec3::from(x.normal)).sum();
                total += 1;
                if n.dot(vn) > 0.0 {
                    agree += 1;
                }
            }
        }
    }
    eprintln!("KH1 uv {:?}..{:?}; textures {:?}; mesh textures {:?}", umin.truncate(), umax.truncate(), textures.iter().map(|t| (t.width, t.height)).collect::<Vec<_>>(), meshes.iter().map(|m| m.texture).collect::<Vec<_>>());
    eprintln!("KH1 triangles whose (0,1,2) normal follows the vertex normals: {agree} of {total}");
    // KH2's winding: (0,1,2) against outward from its bone's vertices' middle.
    let m = kh2_formats::mdlx::parse(&mdlx).unwrap();
    let out_frac = |tris: &mut dyn Iterator<Item = [(u16, Vec3); 3]>| {
        let tris: Vec<_> = tris.collect();
        let mut mid = std::collections::HashMap::<u16, (Vec3, f32)>::new();
        for t in &tris {
            for (b, p) in t {
                let e = mid.entry(*b).or_insert((Vec3::ZERO, 0.0));
                e.0 += *p;
                e.1 += 1.0;
            }
        }
        let (mut out, mut n) = (0, 0);
        for t in &tris {
            if t.iter().all(|x| x.0 == t[0].0) {
                let c = mid[&t[0].0].0 / mid[&t[0].0].1;
                let f = (t[1].1 - t[0].1).cross(t[2].1 - t[0].1);
                n += 1;
                if f.dot((t[0].1 + t[1].1 + t[2].1) / 3.0 - c) > 0.0 {
                    out += 1;
                }
            }
        }
        (out, n)
    };
    let kh2 = out_frac(&mut m.groups.iter().flat_map(|g| g.triangles.iter().map(|t| t.map(|v| (v.bone, Vec3::from(v.position))))));
    let kh1 = out_frac(&mut meshes.iter().flat_map(|m| m.triangles.iter().map(|t| t.map(|i| (m.vertices[i as usize].joint, Vec3::from(m.vertices[i as usize].position))))));
    eprintln!("(0,1,2) faces outward: KH2 {kh2:?}, KH1 {kh1:?}");
}

#[test]
fn kh1_sora_as_kh2_model() {
    let Some((mdls, mdlx)) = files() else { return };
    let out = player_model::build(&mdls, &mdlx).unwrap();
    let m = kh2_formats::mdlx::parse(&out).unwrap();
    let (meshes, textures) = kh1_data::mdls::parse_mesh(&mdls).unwrap();
    assert_eq!(m.bones.len(), 293);
    assert_eq!(m.textures.len(), textures.len());
    assert_eq!(m.triangle_count(), meshes.iter().map(|m| m.triangles.len()).sum::<usize>());
    if let Ok(dir) = std::env::var("KH1_MESH_OUT") {
        std::fs::write(std::path::Path::new(&dir).join("P_EX100.mdlx"), &out).unwrap();
    }
    eprintln!("KH1 Sora model: {} bytes", out.len());
}

#[test]
fn kh2_mset_survey() {
    let Some((_, mdlx)) = files() else { return };
    let dir = std::path::PathBuf::from(std::env::var("KH1_KH2_FILES").unwrap());
    let mset = std::fs::read(dir.join("P_EX100.mset")).unwrap();
    let bar = kh2_formats::bar::Bar::parse(&mset).unwrap();
    let mut kinds = std::collections::BTreeMap::new();
    let (mut interp, mut other, mut dup) = (0, 0, 0);
    for e in &bar.entries {
        *kinds.entry((e.kind, e.name.clone() == "DUMM")).or_insert(0) += 1;
        if e.duplicate != 0 { dup += 1; }
        if e.kind == kh2_formats::motion::BAR_TYPE_ANB && !e.data.is_empty() {
            let anb = kh2_formats::bar::Bar::parse(e.data).unwrap();
            for x in &anb.entries {
                if x.kind == kh2_formats::motion::BAR_TYPE_MOTION {
                    match kh2_formats::anim::Interpolated::parse(x.data) { Ok(Some(_)) => interp += 1, _ => other += 1 }
                }
            }
        }
    }
    eprintln!("{} entries, kinds {kinds:?}, duplicates {dup}, interpolated {interp}, other {other}", bar.entries.len());
    let kh2 = kh2_formats::anim::parse_skeleton(&mdlx).unwrap();
    let t = std::time::Instant::now();
    let e = &bar.entries[151 * 4];
    let anb = kh2_formats::bar::Bar::parse(e.data).unwrap();
    let m = anb.entries.iter().find(|x| x.kind == kh2_formats::motion::BAR_TYPE_MOTION).unwrap();
    let (poses, _) = kh1_assets::convert::kh2_round_trip(m.data, &kh2).unwrap();
    eprintln!("one motion: {} frames in {:?}", poses.len(), t.elapsed());
}

#[test]
fn kh2_motions_on_kh1_skeleton() {
    let Some((mdls, mdlx)) = files() else { return };
    let dir = std::path::PathBuf::from(std::env::var("KH1_KH2_FILES").unwrap());
    let mset2 = std::fs::read(dir.join("P_EX100.mset")).unwrap();
    let mset1 = kh1_data::mset::parse(&std::fs::read(dir.join("xa_ex_0010.mset")).unwrap()).unwrap();
    let kh1 = kh1_data::mdls::parse_skeleton(&mdls).unwrap();
    let kh2 = kh2_formats::anim::parse_skeleton(&mdlx).unwrap();
    let t = std::time::Instant::now();
    let (out, n) = player_model::build_mset(&mset2, &kh2, &kh1, &mset1, true).unwrap();
    eprintln!("swings {n:?}; converted in {:?}, {} -> {} bytes", t.elapsed(), mset2.len(), out.len());
    // Every motion reads back on KH1's skeleton, and plays the pose it was written from.
    let (a, b) = (kh2_formats::bar::Bar::parse(&mset2).unwrap(), kh2_formats::bar::Bar::parse(&out).unwrap());
    let mut worst = 0.0f32;
    assert_eq!(a.entries.len(), b.entries.len());
    for (row, (ea, eb)) in a.entries.iter().zip(&b.entries).enumerate() {
        assert_eq!((ea.kind, ea.duplicate, &ea.name), (eb.kind, eb.duplicate, &eb.name));
        if ea.kind != 17 || ea.data.is_empty() || ea.name == "DUMM" || kh1_assets::kh1_motion_at(row).is_some() { continue; }
        let (xa, xb) = (kh2_formats::bar::Bar::parse(ea.data).unwrap(), kh2_formats::bar::Bar::parse(eb.data).unwrap());
        let ma = xa.entries.iter().find(|e| e.kind == 9).unwrap();
        let mb = xb.entries.iter().find(|e| e.kind == 9).unwrap();
        let (pa, _) = kh1_assets::convert::kh2_round_trip(ma.data, &kh2).unwrap();
        let (pb, _) = kh1_assets::convert::kh2_round_trip(mb.data, &kh1).unwrap();
        let r = kh1_data::retarget::Retarget::new(&kh1, &kh2);
        let all: Vec<_> = pa.iter().map(|p| r.pose(p)).collect();
        for f in [0, pa.len() / 2, pa.len() - 1] {
            let want = &all[f];
            for (j, (x, y)) in want.iter().zip(&pb[f]).enumerate() {
                let d = (x.w_axis - y.w_axis).length();
                worst = worst.max(d);
            }
        }
    }
    eprintln!("worst joint off after the round trip: {worst:.2}");
    assert!(worst < 1.0);
    if let Ok(o) = std::env::var("KH1_MESH_OUT") {
        std::fs::write(std::path::Path::new(&o).join("P_EX100.mset"), &out).unwrap();
    }
}


#[test]
fn kh2_bone_places() {
    let Some((_, mdlx)) = files() else { return };
    let kh2 = kh2_formats::anim::parse_skeleton(&mdlx).unwrap();
    let p = pos(&kh2_anim::Pose::bind(&kh2).bones);
    for b in [25usize, 85, 86, 87, 88, 177, 178] {
        eprintln!("KH2 bone {b} parent {} at {:?}", kh2[b].parent, p[b]);
    }
}

#[test]
fn converted_size_by_channel() {
    let Some((mdls, mdlx)) = files() else { return };
    let dir = std::path::PathBuf::from(std::env::var("KH1_KH2_FILES").unwrap());
    let mset2 = std::fs::read(dir.join("P_EX100.mset")).unwrap();
    let mset1 = kh1_data::mset::parse(&std::fs::read(dir.join("xa_ex_0010.mset")).unwrap()).unwrap();
    let kh1 = kh1_data::mdls::parse_skeleton(&mdls).unwrap();
    let kh2 = kh2_formats::anim::parse_skeleton(&mdlx).unwrap();
    let (out, _) = player_model::build_mset(&mset2, &kh2, &kh1, &mset1, true).unwrap();
    let bar = kh2_formats::bar::Bar::parse(&out).unwrap();
    let mut keys = [0usize; 9];
    let mut curves = [0usize; 9];
    let mut by_bone = std::collections::BTreeMap::<i16, usize>::new();
    let mut seen = std::collections::HashSet::new();
    for e in &bar.entries {
        if e.kind != 17 || e.data.is_empty() || !seen.insert(e.data.as_ptr() as usize) { continue; }
        let anb = kh2_formats::bar::Bar::parse(e.data).unwrap();
        let m = anb.entries.iter().find(|x| x.kind == 9).unwrap();
        let Ok(Some(i)) = kh2_formats::anim::Interpolated::parse(m.data) else { continue };
        for c in &i.forward {
            keys[c.channel as usize] += c.keys.len();
            curves[c.channel as usize] += 1;
            *by_bone.entry(c.joint).or_default() += c.keys.len();
        }
    }
    eprintln!("keys by channel {keys:?}\ncurves by channel {curves:?}");
    let mut v: Vec<_> = by_bone.into_iter().collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.1));
    eprintln!("top bones {:?}", &v[..30]);
}

#[test]
fn swing_triggers() {
    let Some(_) = files() else { return };
    let dir = std::path::PathBuf::from(std::env::var("KH1_KH2_FILES").unwrap());
    let mset2 = std::fs::read(dir.join("P_EX100.mset")).unwrap();
    let mset1 = kh1_data::mset::parse(&std::fs::read(dir.join("xa_ex_0010.mset")).unwrap()).unwrap();
    let bar = kh2_formats::bar::Bar::parse(&mset2).unwrap();
    for id in [151usize, 152, 153, 154, 155, 181, 182, 183, 184] {
        let e = &bar.entries[id * 4];
        let anb = kh2_formats::motion::Anb::parse(e.data).unwrap();
        let fr = anb.pose.as_ref().map(|p| p.frame_count).unwrap_or(0);
        eprintln!("KH2 {id} {} frames {fr}: ranges {:?} frames {:?}", e.name, anb.triggers.ranges.iter().map(|r| (r.start, r.end, r.id, r.params.clone())).collect::<Vec<_>>(), anb.triggers.frames.iter().map(|f| (f.frame, f.id)).collect::<Vec<_>>());
    }
    for k in kh1_assets::KH1_MOTIONS { eprintln!("KH1 {k}: {} frames", mset1[k].as_ref().unwrap().frames); }
    if let Ok(w) = std::fs::read(dir.join("W_EX010.mset")) {
        let b = kh2_formats::bar::Bar::parse(&w).unwrap();
        for id in [151usize, 152, 153, 154, 155, 181, 182, 183, 184] {
            let Some(e) = b.entries.get(id * 4) else { continue };
            let Ok(anb) = kh2_formats::motion::Anb::parse(e.data) else { continue };
            eprintln!("blade {id} {}: ranges {:?}", e.name, anb.triggers.ranges.iter().map(|r| (r.start, r.end, r.id, r.params.clone())).collect::<Vec<_>>());
        }
    }
}
