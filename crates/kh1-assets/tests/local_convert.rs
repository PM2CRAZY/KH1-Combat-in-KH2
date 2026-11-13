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
