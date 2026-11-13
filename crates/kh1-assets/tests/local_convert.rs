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
