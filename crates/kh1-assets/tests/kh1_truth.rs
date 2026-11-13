//! KH1's own poses (a `kh1-posedump` recording, `docs/kh1-posedump.md`) against the offline KH1
//! reader `kh1_data::rig`. Needs `KH1_KH2_FILES` (xa_ex_0010.mset / .mdls) and `KH1_POSEDUMP`
//! (the recording); skipped without them. Prints, per recorded motion, its index in the mset and
//! how far the reader's joints are from KH1's.

use glam::{Mat4, Vec3};

struct Frame {
    tag: u32,
    motion: u64,
    t: f32,
    world: Mat4,
    bones: Vec<Mat4>,
}

fn mat(b: &[u8]) -> Mat4 {
    let mut a = [0f32; 16];
    for (i, v) in a.iter_mut().enumerate() {
        *v = f32::from_le_bytes(b[i * 4..i * 4 + 4].try_into().unwrap());
    }
    Mat4::from_cols_array(&a)
}

fn read_dump(d: &[u8]) -> (Vec<(u64, Vec<u32>)>, Vec<Frame>) {
    assert_eq!(&d[..8], b"KH1POSE1");
    let (mut i, mut motions, mut frames) = (8usize, Vec::new(), Vec::new());
    let u32_at = |i: usize| u32::from_le_bytes(d[i..i + 4].try_into().unwrap());
    let u64_at = |i: usize| u64::from_le_bytes(d[i..i + 8].try_into().unwrap());
    while i < d.len() {
        let tag = u32_at(i);
        i += 4;
        if tag == 1 {
            let mp = u64_at(i);
            let n = u32_at(i + 0x48) as usize;
            let words = (0..n).map(|k| u32_at(i + 0x4C + k * 4)).collect();
            i += 0x4C + n * 4;
            motions.push((mp, words));
        } else {
            let mp = u64_at(i + 8);
            let t = f32::from_le_bytes(d[i + 16..i + 20].try_into().unwrap());
            let n = u32_at(i + 20) as usize;
            let world = mat(&d[i + 24..i + 88]);
            let bones = (0..n).map(|k| mat(&d[i + 88 + k * 64..i + 152 + k * 64])).collect();
            i += 24 + 64 + n * 128;
            frames.push(Frame { tag, motion: mp, t, world, bones });
        }
    }
    (motions, frames)
}

/// KH1's world placement as the pass makes it: rows y and z of the world matrix negated, axes
/// normalised, translation kept apart (`0x140391C70`, Checked (decomp)).
fn model_points(f: &Frame) -> Vec<Vec3> {
    let w = f.world;
    let x = w.x_axis.truncate().normalize();
    let y = -w.y_axis.truncate().normalize();
    let z = -w.z_axis.truncate().normalize();
    let rot = Mat4::from_cols(x.extend(0.0), y.extend(0.0), z.extend(0.0), glam::Vec4::W);
    let inv = rot.inverse();
    let t = w.w_axis.truncate();
    f.bones.iter().map(|b| inv.transform_point3(b.w_axis.truncate() - t)).collect()
}

#[test]
fn kh1_reader_against_kh1() {
    let (Ok(files), Ok(dump)) = (std::env::var("KH1_KH2_FILES"), std::env::var("KH1_POSEDUMP")) else { return };
    let rd = |n: &str| std::fs::read(std::path::Path::new(&files).join(n)).unwrap();
    let kh1 = kh1_data::mdls::parse_skeleton(&rd("xa_ex_0010.mdls")).unwrap();
    let mset = kh1_data::mset::parse(&rd("xa_ex_0010.mset")).unwrap();
    let (motions, frames) = read_dump(&std::fs::read(dump).unwrap());
    let out = std::env::var("KH1_TRUTH_OUT").ok();
    for (mp, words) in &motions {
        let found: Vec<usize> = mset.iter().enumerate().filter(|(_, m)| m.as_ref().is_some_and(|m| &m.joint_words == words)).map(|(i, _)| i).collect();
        let fs: Vec<&Frame> = frames.iter().filter(|f| f.motion == *mp && f.tag == 2).collect();
        let Some(&idx) = found.first() else {
            eprintln!("motion {mp:#x}: no mset match ({} frames)", fs.len());
            continue;
        };
        let m = mset[idx].as_ref().unwrap();
        let mut worst = (0.0f32, 0usize, 0.0f32);
        let mut sum = 0.0f32;
        let mut count = 0;
        for f in &fs {
            let kp = model_points(f);
            let ours = kh1_data::rig::pose(m, &kh1, f.t);
            // Both relative to the root's position (the actor's travel differs).
            let (k0, o0) = (kp[0], ours.bones[0].w_axis.truncate());
            for b in 0..kp.len().min(ours.bones.len()) {
                let e = ((kp[b] - k0) - (ours.bones[b].w_axis.truncate() - o0)).length();
                sum += e;
                count += 1;
                if e > worst.0 {
                    worst = (e, b, f.t);
                }
            }
        }
        let f0 = fs.first().map(|f| model_points(f)[0]).unwrap_or_default();
        eprintln!(
            "motion {mp:#x} = mset {found:?} ({} frames, mset frames {}, t {:.1}..{:.1}): mean joint error {:.2}, worst {:.1} (bone {} at t {:.1}); KH1 root {:?}",
            fs.len(), m.frames, fs.iter().map(|f| f.t).fold(f32::MAX, f32::min), fs.iter().map(|f| f.t).fold(0.0, f32::max),
            sum / count.max(1) as f32, worst.0, worst.1, worst.2, f0
        );
        if let Some(dir) = &out {
            let parents: Vec<String> = kh1.iter().map(|b| b.parent.to_string()).collect();
            let fr: Vec<String> = fs.iter().step_by(3).map(|f| {
                let kp = model_points(f);
                let ours = kh1_data::rig::pose(m, &kh1, f.t);
                let a: Vec<String> = kp.iter().map(|p| format!("[{:.1},{:.1},{:.1}]", p.x, p.y, p.z)).collect();
                let b: Vec<String> = ours.bones.iter().map(|p| { let p = p.w_axis; format!("[{:.1},{:.1},{:.1}]", p.x, p.y, p.z) }).collect();
                format!("{{\"t\":{:.2},\"kh1\":[{}],\"ours\":[{}]}}", f.t, a.join(","), b.join(","))
            }).collect();
            std::fs::write(format!("{dir}/truth-{idx}.json"), format!("{{\"parents\":[{}],\"frames\":[{}]}}", parents.join(","), fr.join(","))).unwrap();
        }
    }
}
