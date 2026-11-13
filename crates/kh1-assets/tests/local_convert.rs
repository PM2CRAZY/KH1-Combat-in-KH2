//! A KH1 motion converted to a KH2 motion and played back by KH2's evaluator: the KH2 pose the
//! motion gives must match the retargeted pose it was written from. Needs the player's own files
//! in `KH1_KH2_FILES` (xa_ex_0010.mset / .mdls, P_EX100.mset / .mdlx); skipped without them.

use glam::Vec3;
use kh2_formats::bar::Bar;

fn dir() -> Option<std::path::PathBuf> {
    let d = std::path::PathBuf::from(std::env::var("KH1_KH2_FILES").ok()?);
    d.join("xa_ex_0010.mset").is_file().then_some(d)
}

#[test]
fn kh1_combo_swing_plays_back_on_kh2() {
    let Some(d) = dir() else { return };
    let rd = |n: &str| std::fs::read(d.join(n)).unwrap();
    let kh1 = kh1_data::mdls::parse_skeleton(&rd("xa_ex_0010.mdls")).unwrap();
    let kh2 = kh2_formats::anim::parse_skeleton(&rd("P_EX100.mdlx")).unwrap();
    let motions = kh1_data::mset::parse(&rd("xa_ex_0010.mset")).unwrap();
    let mset = rd("P_EX100.mset");
    let bar = Bar::parse(&mset).unwrap();
    let a300 = Bar::parse(bar.entries[151 * 4].data).unwrap();
    let template = a300.entries.iter().find(|e| e.kind == 9).unwrap().data;
    for idx in [98usize, 101] {
        let m = motions[idx].clone().unwrap();
        let (poses, travel) = kh1_assets::convert::kh2_poses(&m, &kh1, &kh2);
        let data = kh1_assets::convert::write_motion(template, &kh2, &poses, &travel).unwrap();
        let back = kh2_formats::anim::Interpolated::parse(&data).unwrap().unwrap();
        assert_eq!(back.frame_count as usize, poses.len());
        let info = kh2_formats::motion::MotionInfo::parse(&data).unwrap();
        let mut worst = 0.0f32;
        for f in [0usize, poses.len() / 3, poses.len() / 2, poses.len() - 1] {
            let p = kh2_anim::evaluate(&kh2, &back, f as f32, kh2_anim::Options { constraints: false });
            for b in 0..kh2.len() {
                let e = (p.bones[b].w_axis.truncate() - poses[f][b].w_axis.truncate()).length();
                worst = worst.max(e);
            }
            let r = info.root_at(f as f32);
            let t = travel[f];
            assert!((Vec3::from(r) - t).length() < 1.0 || f == 0, "travel {r:?} vs {t:?} at {f}");
        }
        if let Ok(out) = std::env::var("KH1_KH2_DUMP") {
            let parents: Vec<String> = kh2.iter().map(|b| b.parent.to_string()).collect();
            let frames: Vec<String> = (0..poses.len()).step_by(4).map(|f| {
                let p = kh2_anim::evaluate(&kh2, &back, f as f32, kh2_anim::Options { constraints: false });
                let r = info.root_at(f as f32);
                let pts: Vec<String> = p.bones.iter().map(|m| { let t = m.w_axis; format!("[{:.1},{:.1},{:.1}]", t.x + r[0], t.y + r[1], t.z + r[2]) }).collect();
                format!("[{}]", pts.join(","))
            }).collect();
            std::fs::write(format!("{out}/kh2-{idx}.json"), format!("{{\"parents\":[{}],\"frames\":[{}]}}", parents.join(","), frames.join(","))).unwrap();
        }
        eprintln!("motion {idx}: {} frames, {} bytes, worst joint error {worst:.3}", poses.len(), data.len());
        assert!(worst < 1.5, "worst joint error {worst}");
    }
}

#[test]
fn kh2_bone_lengths_hold() {
    let Some(d) = dir() else { return };
    let rd = |n: &str| std::fs::read(d.join(n)).unwrap();
    let kh1 = kh1_data::mdls::parse_skeleton(&rd("xa_ex_0010.mdls")).unwrap();
    let kh2 = kh2_formats::anim::parse_skeleton(&rd("P_EX100.mdlx")).unwrap();
    let motions = kh1_data::mset::parse(&rd("xa_ex_0010.mset")).unwrap();
    let m = motions[98].clone().unwrap();
    let (poses, _) = kh1_assets::convert::kh2_poses(&m, &kh1, &kh2);
    let mut bad = 0;
    for (b, bone) in kh2.iter().enumerate() {
        let Some(p) = usize::try_from(bone.parent).ok() else { continue };
        let bind = Vec3::from(bone.translation).length();
        let mut worst = 0.0f32;
        for pose in &poses {
            let l = (pose[b].w_axis - pose[p].w_axis).truncate().length();
            worst = worst.max((l - bind).abs());
        }
        if worst > 0.5 {
            bad += 1;
            eprintln!("bone {b} parent {p}: bind {bind:.2}, off by up to {worst:.2}");
        }
    }
    assert_eq!(bad, 0, "bones stretched away from KH2's own lengths");
}

#[test]
fn extended_mset_keeps_kh2s_entries_and_sharing() {
    let Some(d) = dir() else { return };
    let orig = std::fs::read(d.join("P_EX100.mset")).unwrap();
    let extra = vec![(0u16, 1i16, *b"DUMM", None), (17u16, 0i16, *b"K098", Some(vec![7u8; 40]))];
    let out = kh1_assets::convert::extend_bar(&orig, &extra).unwrap();
    let (a, b) = (Bar::parse(&orig).unwrap(), Bar::parse(&out).unwrap());
    assert_eq!(b.entries.len(), a.entries.len() + 2);
    for (x, y) in a.entries.iter().zip(&b.entries) {
        assert_eq!((x.kind, x.duplicate, &x.name, x.data.len()), (y.kind, y.duplicate, &y.name, y.data.len()));
        assert!(x.data == y.data, "{} data differs", x.name);
    }
    let n = a.entries.len();
    assert_eq!((b.entries[n].name.as_str(), b.entries[n].duplicate, b.entries[n].data.len()), ("DUMM", 1, 0));
    assert_eq!(b.entries[n + 1].data, &[7u8; 40][..]);
    assert_eq!(out.len(), orig.len() + 32 + 48);
    eprintln!("{} -> {} bytes", orig.len(), out.len());
}

#[test]
fn rotations_play_back() {
    let Some(d) = dir() else { return };
    let rd = |n: &str| std::fs::read(d.join(n)).unwrap();
    let kh1 = kh1_data::mdls::parse_skeleton(&rd("xa_ex_0010.mdls")).unwrap();
    let kh2 = kh2_formats::anim::parse_skeleton(&rd("P_EX100.mdlx")).unwrap();
    let motions = kh1_data::mset::parse(&rd("xa_ex_0010.mset")).unwrap();
    let mset = rd("P_EX100.mset");
    let bar = Bar::parse(&mset).unwrap();
    let a300 = Bar::parse(bar.entries[151 * 4].data).unwrap();
    let template = a300.entries.iter().find(|e| e.kind == 9).unwrap().data;
    let m = motions[98].clone().unwrap();
    let (poses, travel) = kh1_assets::convert::kh2_poses(&m, &kh1, &kh2);
    let data = kh1_assets::convert::write_motion(template, &kh2, &poses, &travel).unwrap();
    let back = kh2_formats::anim::Interpolated::parse(&data).unwrap().unwrap();
    let mut worst = (0.0f32, 0, 0);
    for f in 0..poses.len() {
        let p = kh2_anim::evaluate(&kh2, &back, f as f32, kh2_anim::Options { constraints: false });
        for b in 0..kh2.len() {
            let (_, qa, _) = p.bones[b].to_scale_rotation_translation();
            let (_, qb, _) = poses[f][b].to_scale_rotation_translation();
            let a = qa.angle_between(qb);
            if a > worst.0 { worst = (a, f, b); }
        }
    }
    eprintln!("worst rotation error {:.3} rad at frame {} bone {}", worst.0, worst.1, worst.2);
}

#[test]
fn dump_root_values() {
    let Some(d) = dir() else { return };
    if std::env::var("KH1_KH2_ROOTS").is_err() { return }
    let rd = |n: &str| std::fs::read(d.join(n)).unwrap();
    let kh1 = kh1_data::mdls::parse_skeleton(&rd("xa_ex_0010.mdls")).unwrap();
    let kh2 = kh2_formats::anim::parse_skeleton(&rd("P_EX100.mdlx")).unwrap();
    for b in 0..4 { eprintln!("bind {b}: parent {} rot {:?} t {:?}", kh2[b].parent, kh2[b].rotation, kh2[b].translation); }
    let mset = rd("P_EX100.mset");
    let bar = Bar::parse(&mset).unwrap();
    let a300 = Bar::parse(bar.entries[151 * 4].data).unwrap();
    let template = a300.entries.iter().find(|e| e.kind == 9).unwrap().data;
    let motions = kh1_data::mset::parse(&rd("xa_ex_0010.mset")).unwrap();
    let m = motions[98].clone().unwrap();
    let (poses, travel) = kh1_assets::convert::kh2_poses(&m, &kh1, &kh2);
    let data = kh1_assets::convert::write_motion(template, &kh2, &poses, &travel).unwrap();
    let back = kh2_formats::anim::Interpolated::parse(&data).unwrap().unwrap();
    for (j, c, v) in back.initial_pose.iter().filter(|x| x.0 < 4) { eprintln!("pose {j} {c} {v:.3}"); }
    for c in back.forward.iter().filter(|c| c.joint < 4) { eprintln!("curve {} {} first {:.3} last {:.3}", c.joint, c.channel, c.keys[0].value, c.keys.last().unwrap().value); }
}

#[test]
fn dump_a300_upright() {
    let Some(d) = dir() else { return };
    if std::env::var("KH1_KH2_ROOTS").is_err() { return }
    let rd = |n: &str| std::fs::read(d.join(n)).unwrap();
    let kh2 = kh2_formats::anim::parse_skeleton(&rd("P_EX100.mdlx")).unwrap();
    let mset = rd("P_EX100.mset");
    let bar = Bar::parse(&mset).unwrap();
    for slot in [151usize, 0] {
        let anb = Bar::parse(bar.entries[slot * 4].data).unwrap();
        let data = anb.entries.iter().find(|e| e.kind == 9).unwrap().data;
        let m = kh2_formats::anim::Interpolated::parse(data).unwrap().unwrap();
        for cons in [false, true] {
            let p = kh2_anim::evaluate(&kh2, &m, 0.0, kh2_anim::Options { constraints: cons });
            let ys: Vec<f32> = p.bones.iter().map(|b| b.w_axis.y).collect();
            let top = ys.iter().cloned().fold(f32::MIN, f32::max);
            let bot = ys.iter().cloned().fold(f32::MAX, f32::min);
            eprintln!("motion {slot} cons {cons}: root y {:.1}, highest {top:.1}, lowest {bot:.1}, bone 3 {:?}", ys[0], p.bones[3].w_axis);
        }
    }
    let p = kh2_anim::Pose::bind(&kh2);
    let ys: Vec<f32> = p.bones.iter().map(|b| b.w_axis.y).collect();
    eprintln!("bind: highest {:.1} lowest {:.1}", ys.iter().cloned().fold(f32::MIN, f32::max), ys.iter().cloned().fold(f32::MAX, f32::min));
}

#[test]
fn dump_a300_round_trip() {
    let Some(d) = dir() else { return };
    if std::env::var("KH1_KH2_ROOTS").is_err() { return }
    let rd = |n: &str| std::fs::read(d.join(n)).unwrap();
    let kh2 = kh2_formats::anim::parse_skeleton(&rd("P_EX100.mdlx")).unwrap();
    let mset = rd("P_EX100.mset");
    let bar = Bar::parse(&mset).unwrap();
    let anb = Bar::parse(bar.entries[151 * 4].data).unwrap();
    let data = anb.entries.iter().find(|e| e.kind == 9).unwrap().data;
    let m = kh2_formats::anim::Interpolated::parse(data).unwrap().unwrap();
    let info = kh2_formats::motion::MotionInfo::parse(data).unwrap();
    let n = m.frame_count as usize;
    let poses: Vec<Vec<glam::Mat4>> = (0..n).map(|f| kh2_anim::evaluate(&kh2, &m, f as f32, kh2_anim::Options { constraints: true }).bones).collect();
    let travel: Vec<Vec3> = (0..n).map(|f| Vec3::from(info.root_at(f as f32))).collect();
    let out = kh1_assets::convert::write_motion(data, &kh2, &poses, &travel).unwrap();
    let back = kh2_formats::anim::Interpolated::parse(&out).unwrap().unwrap();
    let val = |mm: &kh2_formats::anim::Interpolated, j: i16, c: u8| -> f32 {
        mm.forward.iter().find(|x| x.joint == j && x.channel == c).map(|x| x.eval(0.0))
            .or_else(|| mm.initial_pose.iter().find(|x| x.0 == j && x.1 == c as i16).map(|x| x.2))
            .unwrap_or(f32::NAN)
    };
    let mut diffs = 0;
    for j in 0..228i16 {
        for c in 3..9u8 {
            let (a, b) = (val(&m, j, c), val(&back, j, c));
            if a.is_finite() && (a - b).abs() > 0.05 {
                diffs += 1;
                if diffs < 25 { eprintln!("bone {j} ch {c}: KH2 {a:.3} ours {b:.3}"); }
            }
        }
    }
    eprintln!("{diffs} channels differ at frame 0; root y in our pose {:.1}", poses[0][0].w_axis.y);
}

#[test]
fn dump_kh1_motion_range() {
    let Some(d) = dir() else { return };
    let Ok(out) = std::env::var("KH1_RANGE_OUT") else { return };
    let rd = |n: &str| std::fs::read(d.join(n)).unwrap();
    let kh1 = kh1_data::mdls::parse_skeleton(&rd("xa_ex_0010.mdls")).unwrap();
    let motions = kh1_data::mset::parse(&rd("xa_ex_0010.mset")).unwrap();
    let mut s = String::from("{\"parents\":[");
    s += &kh1.iter().map(|b| b.parent.to_string()).collect::<Vec<_>>().join(",");
    s += "],\"motions\":{";
    let mut first = true;
    for (i, m) in motions.iter().enumerate() {
        let Some(m) = m else { continue };
        if !first { s += ","; }
        first = false;
        let n = m.frames.max(1) as usize;
        let fr: Vec<String> = [0, n / 4, n / 2, 3 * n / 4].iter().map(|&f| {
            let p = kh1_data::rig::pose(m, &kh1, f as f32);
            format!("[{}]", p.bones.iter().map(|b| format!("[{:.1},{:.1},{:.1}]", b.w_axis.x, b.w_axis.y, b.w_axis.z)).collect::<Vec<_>>().join(","))
        }).collect();
        s += &format!("\"{i}\":{{\"frames\":{n},\"p\":[{}]}}", fr.join(","));
    }
    s += "}}";
    std::fs::write(out, s).unwrap();
}

#[test]
fn dump_kh1_root_channels() {
    let Some(d) = dir() else { return };
    if std::env::var("KH1_KH2_ROOTS").is_err() { return }
    let rd = |n: &str| std::fs::read(d.join(n)).unwrap();
    let motions = kh1_data::mset::parse(&rd("xa_ex_0010.mset")).unwrap();
    for idx in [0usize, 98] {
        let m = motions[idx].clone().unwrap();
        eprintln!("motion {idx}: frames {} fps {} bones {} helpers {} words {}", m.frames, m.fps, m.bones, m.helpers, m.joint_words.len());
        for c in m.channels.iter().filter(|c| c.bone < 3) {
            let v: Vec<String> = [0.0f32, 10.0, 20.0, 30.0].iter().map(|&f| format!("{:.2}", c.sample(f).unwrap_or(f32::NAN))).collect();
            eprintln!("  bone {} kind {} {:?}", c.bone, c.kind, v);
        }
        eprintln!("  words {:?}", &m.joint_words[..m.joint_words.len().min(12)]);
    }
}

#[test]
fn dump_kh1_helpers() {
    let Some(d) = dir() else { return };
    if std::env::var("KH1_KH2_ROOTS").is_err() { return }
    let rd = |n: &str| std::fs::read(d.join(n)).unwrap();
    let kh1 = kh1_data::mdls::parse_skeleton(&rd("xa_ex_0010.mdls")).unwrap();
    let motions = kh1_data::mset::parse(&rd("xa_ex_0010.mset")).unwrap();
    for idx in [0usize, 98] {
        let m = motions[idx].clone().unwrap();
        for (k, h) in m.helper_table.iter().enumerate() { eprintln!("m{idx} helper {k}: parent {} t {:?} r {:?}", h.parent, h.translation, h.rotation); }
        let mut seen = std::collections::BTreeSet::new();
        for c in &m.helper_chans { seen.insert((c.bone, c.kind)); }
        eprintln!("m{idx} helper channels {:?}", seen);
        for f in [0.0f32, 10.0, 20.0, 30.0, 40.0] {
            let p = kh1_data::rig::pose(&m, &kh1, f);
            let hs: Vec<String> = p.helpers.iter().map(|h| format!("{:.0},{:.0},{:.0}", h.w_axis.x, h.w_axis.y, h.w_axis.z)).collect();
            eprintln!("m{idx} f{f}: root {:.1} | {}", p.bones[0].w_axis.y, hs.join(" | "));
        }
    }
}

#[test]
fn dump_kh1_key_times() {
    let Some(d) = dir() else { return };
    if std::env::var("KH1_KH2_ROOTS").is_err() { return }
    let rd = |n: &str| std::fs::read(d.join(n)).unwrap();
    let motions = kh1_data::mset::parse(&rd("xa_ex_0010.mset")).unwrap();
    for idx in [0usize, 98, 101] {
        let m = motions[idx].clone().unwrap();
        let mx = m.channels.iter().chain(&m.helper_chans).flat_map(|c| c.keys.iter().map(|k| k.time)).fold(0.0f32, f32::max);
        let h3: Vec<String> = m.helper_chans.iter().filter(|c| c.bone == 3 && c.kind == 8).flat_map(|c| c.keys.iter().map(|k| format!("{:.1}@{:.1}", k.value, k.time))).collect();
        eprintln!("m{idx}: frames {} fps {} max key time {mx}; helper3 ty keys {:?}", m.frames, m.fps, h3);
    }
}

#[test]
fn dump_bind_vs_idle() {
    let Some(d) = dir() else { return };
    if std::env::var("KH1_KH2_ROOTS").is_err() { return }
    let rd = |n: &str| std::fs::read(d.join(n)).unwrap();
    let kh2 = kh2_formats::anim::parse_skeleton(&rd("P_EX100.mdlx")).unwrap();
    let mset = rd("P_EX100.mset");
    let bar = Bar::parse(&mset).unwrap();
    let anb = Bar::parse(bar.entries[0].data).unwrap();
    let data = anb.entries.iter().find(|e| e.kind == 9).unwrap().data;
    let m = kh2_formats::anim::Interpolated::parse(data).unwrap().unwrap();
    let idle = kh2_anim::evaluate(&kh2, &m, 0.0, kh2_anim::Options { constraints: true });
    let bind = kh2_anim::Pose::bind(&kh2);
    let mut big = Vec::new();
    for b in 0..kh2.len() {
        let (_, qi, _) = idle.bones[b].to_scale_rotation_translation();
        let (_, qb, _) = bind.bones[b].to_scale_rotation_translation();
        let a = qi.angle_between(qb).to_degrees();
        if a > 60.0 { big.push((b, a as i32, kh2[b].parent)); }
    }
    eprintln!("{} bones turned over 60 deg between KH2's bind and its idle: {:?}", big.len(), big);
}

#[test]
fn dump_template_constraints() {
    let Some(d) = dir() else { return };
    if std::env::var("KH1_KH2_ROOTS").is_err() { return }
    let rd = |n: &str| std::fs::read(d.join(n)).unwrap();
    let mset = rd("P_EX100.mset");
    let bar = Bar::parse(&mset).unwrap();
    for slot in [151usize, 0] {
        let anb = Bar::parse(bar.entries[slot * 4].data).unwrap();
        let data = anb.entries.iter().find(|e| e.kind == 9).unwrap().data;
        let m = kh2_formats::anim::Interpolated::parse(data).unwrap().unwrap();
        eprintln!("motion {slot}: {} constraints, {} activations, {} limiters, {} expressions, {} nodes", m.constraints.len(), m.activations.len(), m.limiters.len(), m.expressions.len(), m.nodes.len());
        for c in &m.constraints { eprintln!("  kind {} constrained {} source {} limiter {} act {}+{} temp {}", c.kind, c.constrained, c.source, c.limiter, c.activation_start, c.activation_count, c.temporary_active); }
        for e in &m.expressions { eprintln!("  expr {:?}", e); }
    }
}

#[test]
fn dump_mesh_poses() {
    let Some(d) = dir() else { return };
    let Ok(out) = std::env::var("KH1_MESH_OUT") else { return };
    let rd = |n: &str| std::fs::read(d.join(n)).unwrap();
    let mdlx = rd("P_EX100.mdlx");
    let model = kh2_formats::mdlx::parse(&mdlx).unwrap();
    let kh1 = kh1_data::mdls::parse_skeleton(&rd("xa_ex_0010.mdls")).unwrap();
    let kh2 = kh2_formats::anim::parse_skeleton(&mdlx).unwrap();
    let motions = kh1_data::mset::parse(&rd("xa_ex_0010.mset")).unwrap();
    let mset = rd("P_EX100.mset");
    let bar = Bar::parse(&mset).unwrap();
    let a300 = Bar::parse(bar.entries[151 * 4].data).unwrap();
    let template = a300.entries.iter().find(|e| e.kind == 9).unwrap().data;
    let tm = kh2_formats::anim::Interpolated::parse(template).unwrap().unwrap();
    let m = motions[98].clone().unwrap();
    let (poses, travel) = kh1_assets::convert::kh2_poses(&m, &kh1, &kh2);
    let data = kh1_assets::convert::write_motion(template, &kh2, &poses, &travel).unwrap();
    let back = kh2_formats::anim::Interpolated::parse(&data).unwrap().unwrap();
    let mut sets: Vec<(String, Vec<glam::Mat4>)> = vec![("bind".into(), kh2_anim::Pose::bind(&kh2).bones)];
    sets.push(("a300_f20".into(), kh2_anim::evaluate(&kh2, &tm, 20.0, kh2_anim::Options { constraints: true }).bones));
    for f in [0usize, 10, 20, 30, 40] {
        sets.push((format!("ours_f{f}"), kh2_anim::evaluate(&kh2, &back, f as f32, kh2_anim::Options { constraints: false }).bones));
    }
    for (name, bones) in sets {
        let mut s = String::from("[");
        let mut first = true;
        for g in &model.groups {
            for t in &g.triangles {
                if !first { s.push(','); }
                first = false;
                let p: Vec<String> = t.iter().map(|v| { let w = bones[v.bone as usize].transform_point3(glam::Vec3::from(v.position)); format!("[{:.1},{:.1},{:.1},{}]", w.x, w.y, w.z, v.bone) }).collect();
                s.push_str(&format!("[{}]", p.join(",")));
            }
        }
        s.push(']');
        std::fs::write(format!("{out}/mesh-{name}.json"), s).unwrap();
    }
}

#[test]
fn dump_locals_bind_vs_kh2() {
    let Some(d) = dir() else { return };
    if std::env::var("KH1_KH2_ROOTS").is_err() { return }
    let rd = |n: &str| std::fs::read(d.join(n)).unwrap();
    let kh2 = kh2_formats::anim::parse_skeleton(&rd("P_EX100.mdlx")).unwrap();
    let mset = rd("P_EX100.mset");
    let bar = Bar::parse(&mset).unwrap();
    let ms: Vec<kh2_formats::anim::Interpolated> = [151usize, 0].iter().map(|&s| { let anb = Bar::parse(bar.entries[s * 4].data).unwrap(); kh2_formats::anim::Interpolated::parse(anb.entries.iter().find(|e| e.kind == 9).unwrap().data).unwrap().unwrap() }).collect();
    let val = |m: &kh2_formats::anim::Interpolated, j: usize, c: u8| -> String {
        if let Some(x) = m.forward.iter().find(|x| x.joint as usize == j && x.channel == c) { return format!("curve{:.2}", x.eval(0.0)); }
        if let Some(x) = m.initial_pose.iter().find(|x| x.0 as usize == j && x.1 == c as i16) { return format!("{:.2}", x.2); }
        "-".into()
    };
    for b in [1usize, 2, 3, 10, 11, 12, 22, 23, 24, 32, 34, 38, 39, 40, 41, 52, 53, 63] {
        let bone = &kh2[b];
        let r: Vec<String> = (3..9u8).map(|c| val(&ms[0], b, c)).collect();
        let i: Vec<String> = (3..9u8).map(|c| val(&ms[1], b, c)).collect();
        eprintln!("bone {b} (parent {}): bind r {:?} t {:?} | A300 {:?} | idle {:?}", bone.parent, bone.rotation.map(|x| (x * 100.0).round() / 100.0), bone.translation.map(|x| (x * 10.0).round() / 10.0), r, i);
    }
}

#[test]
fn dump_idle_vs_kh2_idle() {
    let Some(d) = dir() else { return };
    if std::env::var("KH1_KH2_ROOTS").is_err() { return }
    let rd = |n: &str| std::fs::read(d.join(n)).unwrap();
    let kh1 = kh1_data::mdls::parse_skeleton(&rd("xa_ex_0010.mdls")).unwrap();
    let kh2 = kh2_formats::anim::parse_skeleton(&rd("P_EX100.mdlx")).unwrap();
    let motions = kh1_data::mset::parse(&rd("xa_ex_0010.mset")).unwrap();
    let mset = rd("P_EX100.mset");
    let bar = Bar::parse(&mset).unwrap();
    let anb = Bar::parse(bar.entries[0].data).unwrap();
    let idle2 = kh2_formats::anim::Interpolated::parse(anb.entries.iter().find(|e| e.kind == 9).unwrap().data).unwrap().unwrap();
    let k2 = kh2_anim::evaluate(&kh2, &idle2, 0.0, kh2_anim::Options { constraints: true }).bones;
    let (ours, _) = kh1_assets::convert::kh2_poses(&motions[0].clone().unwrap(), &kh1, &kh2);
    let bind = kh2_anim::Pose::bind(&kh2).bones;
    let ang = |a: &glam::Mat4, b: &glam::Mat4| { let (_, qa, _) = a.to_scale_rotation_translation(); let (_, qb, _) = b.to_scale_rotation_translation(); qa.angle_between(qb).to_degrees() as i32 };
    for b in [0usize, 1, 2, 3, 4, 10, 11, 12, 13, 23, 24, 39, 40, 41, 52, 53, 68, 69, 74, 76, 79, 140, 141] {
        eprintln!("bone {b}: ours vs KH2 idle {} deg, ours vs bind {} deg, KH2 idle vs bind {} deg; pos ours {:?} kh2 {:?}", ang(&ours[0][b], &k2[b]), ang(&ours[0][b], &bind[b]), ang(&k2[b], &bind[b]), ours[0][b].w_axis.truncate().round(), k2[b].w_axis.truncate().round());
    }
}

#[test]
fn dump_torso_refs() {
    let Some(d) = dir() else { return };
    if std::env::var("KH1_KH2_ROOTS").is_err() { return }
    let rd = |n: &str| std::fs::read(d.join(n)).unwrap();
    let kh1 = kh1_data::mdls::parse_skeleton(&rd("xa_ex_0010.mdls")).unwrap();
    let kh2 = kh2_formats::anim::parse_skeleton(&rd("P_EX100.mdlx")).unwrap();
    let motions = kh1_data::mset::parse(&rd("xa_ex_0010.mset")).unwrap();
    let r = kh1_data::retarget::Retarget::new(&kh1, &kh2);
    let inv = kh1_data::inverse::Inverse::new(&kh1, &kh2, &r, |i| i < 280 && r.follows_one_part(i));
    let m0 = motions[0].clone().unwrap();
    for b in inv.torso_bones() {
        let w = m0.joint_words[b];
        let p0 = kh1_data::rig::pose(&m0, &kh1, 0.0).bones[b];
        let bind = kh2_anim::Pose::bind(&kh1).bones[b];
        let (_, q0, _) = p0.to_scale_rotation_translation();
        let (_, qb, _) = bind.to_scale_rotation_translation();
        eprintln!("torso KH1 bone {b} parent {} word {w:#010x} (mode {} lo {} A {}), idle vs bind {:.0} deg", kh1[b].parent, (w >> 21) & 3, w & 3, ((w >> 16 & 0x800) | (w & 0x7FC)) >> 2, q0.angle_between(qb).to_degrees());
    }
}
