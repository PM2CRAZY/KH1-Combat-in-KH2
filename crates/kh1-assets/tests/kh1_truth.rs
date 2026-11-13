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
    locals: Vec<Mat4>,
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
            let lo = i + 88 + n * 64;
            let locals = (0..n).map(|k| mat(&d[lo + k * 64..lo + 64 + k * 64])).collect();
            i += 24 + 64 + n * 128;
            frames.push(Frame { tag, motion: mp, t, world, bones, locals });
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

#[test]
fn kh1_locals_against_kh1() {
    let (Ok(files), Ok(dump)) = (std::env::var("KH1_KH2_FILES"), std::env::var("KH1_POSEDUMP")) else { return };
    let Ok(want) = std::env::var("KH1_LOCALS") else { return };
    let mut it = want.split(':');
    let idx: usize = it.next().unwrap().parse().unwrap();
    let bones: Vec<usize> = it.next().unwrap().split(',').map(|x| x.parse().unwrap()).collect();
    let rd = |n: &str| std::fs::read(std::path::Path::new(&files).join(n)).unwrap();
    let kh1 = kh1_data::mdls::parse_skeleton(&rd("xa_ex_0010.mdls")).unwrap();
    let mset = kh1_data::mset::parse(&rd("xa_ex_0010.mset")).unwrap();
    let m = mset[idx].as_ref().unwrap();
    let (motions, frames) = read_dump(&std::fs::read(dump).unwrap());
    let mp = motions.iter().find(|(_, w)| w == &m.joint_words).unwrap().0;
    let mut fs: Vec<&Frame> = frames.iter().filter(|f| f.motion == mp && f.tag == 2).collect();
    fs.sort_by(|a, b| a.t.total_cmp(&b.t));
    for f in fs.iter().step_by(4) {
        // The sample time whose turns fit KH1's locals best (over the asked bones).
        let fit = |tau: f32| -> f32 { let s = m.apply(&kh1, tau); bones.iter().map(|&b| { let (_, q, _) = f.locals[b].to_scale_rotation_translation(); q.angle_between(kh2_anim::euler_quat(glam::Vec3::from(s[b].rotation))) }).sum() };
        let best = (0..=800).map(|k| k as f32 * 0.1).min_by(|a, b| fit(*a).total_cmp(&fit(*b))).unwrap();
        eprintln!("recorded t {:5.1}: best sample time {:5.1} (error {:.2} deg there, {:.2} at t)", f.t, best, fit(best).to_degrees(), fit(f.t).to_degrees());
        let s = m.apply(&kh1, f.t);
        for &b in &bones {
            let (_, q, t) = f.locals[b].to_scale_rotation_translation();
            let ours = kh2_anim::euler_quat(glam::Vec3::from(s[b].rotation));
            eprintln!("t {:5.1} bone {b}: KH1 local turn vs our sampled {:6.2} deg; KH1 t {:?} ours t {:?}; ours rot {:?}", f.t, q.angle_between(ours).to_degrees(), t, s[b].translation, s[b].rotation);
        }
    }
}

fn sample_variant(c: &kh1_data::mset::Channel, t: f32, v: u8) -> Option<f32> {
    let (first, last) = (c.keys.first()?, c.keys.last()?);
    if c.keys.len() == 1 { return Some(first.value); }
    if t <= first.time { return Some(first.value); }
    if t >= last.time { return Some(last.value); }
    let i = c.keys.partition_point(|k| k.time <= t).saturating_sub(1).min(c.keys.len() - 2);
    let (a, b) = (c.keys[i], c.keys[i + 1]);
    let gap = b.time - a.time;
    if gap <= 0.0 || a.interpolation == 0 { return Some(a.value); }
    let x = t - a.time;
    if a.interpolation == 1 || v == 4 { return Some(a.value + (b.value - a.value) * x / gap); }
    let s = x / gap;
    let (s2, s3) = (s * s, s * s * s);
    let (h00, h01, h10, h11) = (2.0 * s3 - 3.0 * s2 + 1.0, 3.0 * s2 - 2.0 * s3, s3 - 2.0 * s2 + s, s3 - s2);
    let base = h00 * a.value + h01 * b.value;
    Some(match v {
        0 => base + gap * (h10 * a.tangent_in + h11 * b.tangent_out),
        1 => base + gap * (h10 * a.tangent_out + h11 * b.tangent_in),
        2 => base + (h10 * a.tangent_in + h11 * b.tangent_out),
        3 => base + (h10 * a.tangent_out + h11 * b.tangent_in),
        _ => base,
    })
}

#[test]
fn kh1_sampling_variants() {
    let (Ok(files), Ok(dump)) = (std::env::var("KH1_KH2_FILES"), std::env::var("KH1_POSEDUMP")) else { return };
    if std::env::var("KH1_VARIANTS").is_err() { return }
    let rd = |n: &str| std::fs::read(std::path::Path::new(&files).join(n)).unwrap();
    let kh1 = kh1_data::mdls::parse_skeleton(&rd("xa_ex_0010.mdls")).unwrap();
    let mset = kh1_data::mset::parse(&rd("xa_ex_0010.mset")).unwrap();
    let (motions, frames) = read_dump(&std::fs::read(dump).unwrap());
    for idx in [98usize, 99, 101] {
        let m = mset[idx].as_ref().unwrap();
        let mp = motions.iter().find(|(_, w)| w == &m.joint_words).unwrap().0;
        let fs: Vec<&Frame> = frames.iter().filter(|f| f.motion == mp && f.tag == 2).collect();
        // Plain FK bones (word 0): their local turn is the sampled one.
        let plain: Vec<usize> = (0..kh1.len()).filter(|&b| m.joint_words[b] == 0).collect();
        for v in 0..6u8 {
            let mut sum = 0.0f32;
            let mut n = 0;
            for f in &fs {
                for &b in &plain {
                    let mut r = kh1[b].rotation;
                    for &(bone, kind, val) in &m.constants { if bone as usize == b && (4..=6).contains(&kind) { r[(kind - 4) as usize] = val; } }
                    for c in m.channels.iter().filter(|c| c.bone as usize == b && (4..=6).contains(&c.kind)) {
                        if let Some(x) = sample_variant(c, f.t, v) { r[(c.kind - 4) as usize] = x; }
                    }
                    let (_, q, _) = f.locals[b].to_scale_rotation_translation();
                    sum += q.angle_between(kh2_anim::euler_quat(glam::Vec3::from(r))).to_degrees();
                    n += 1;
                }
            }
            eprintln!("motion {idx} variant {v}: mean local error {:.3} deg over {} bone frames", sum / n.max(1) as f32, n);
        }
    }
}

#[test]
fn kh1_error_by_frame() {
    let (Ok(files), Ok(dump)) = (std::env::var("KH1_KH2_FILES"), std::env::var("KH1_POSEDUMP")) else { return };
    let Ok(want) = std::env::var("KH1_BYFRAME") else { return };
    let idx: usize = want.parse().unwrap();
    let rd = |n: &str| std::fs::read(std::path::Path::new(&files).join(n)).unwrap();
    let kh1 = kh1_data::mdls::parse_skeleton(&rd("xa_ex_0010.mdls")).unwrap();
    let mset = kh1_data::mset::parse(&rd("xa_ex_0010.mset")).unwrap();
    let (motions, frames) = read_dump(&std::fs::read(dump).unwrap());
    let m = mset[idx].as_ref().unwrap();
    let mp = motions.iter().find(|(_, w)| w == &m.joint_words).unwrap().0;
    let plain: Vec<usize> = (0..kh1.len()).filter(|&b| m.joint_words[b] == 0).collect();
    let mut prev_motion = 0u64;
    for f in frames.iter().filter(|f| f.tag == 2) {
        if f.motion != mp { prev_motion = f.motion; continue; }
        let s = m.apply(&kh1, f.t);
        let e: f32 = plain.iter().map(|&b| { let (_, q, _) = f.locals[b].to_scale_rotation_translation(); q.angle_between(kh2_anim::euler_quat(glam::Vec3::from(s[b].rotation))).to_degrees() }).sum::<f32>() / plain.len() as f32;
        eprintln!("t {:5.1} (after motion {:#x}): mean plain-bone error {:.2} deg", f.t, prev_motion, e);
    }
}

#[test]
fn kh1_channel_trace() {
    let (Ok(files), Ok(dump)) = (std::env::var("KH1_KH2_FILES"), std::env::var("KH1_POSEDUMP")) else { return };
    let Ok(want) = std::env::var("KH1_CHANNEL") else { return };
    let v: Vec<usize> = want.split(':').map(|x| x.parse().unwrap()).collect();
    let (idx, bone) = (v[0], v[1]);
    let rd = |n: &str| std::fs::read(std::path::Path::new(&files).join(n)).unwrap();
    let kh1 = kh1_data::mdls::parse_skeleton(&rd("xa_ex_0010.mdls")).unwrap();
    let mset = kh1_data::mset::parse(&rd("xa_ex_0010.mset")).unwrap();
    let (motions, frames) = read_dump(&std::fs::read(dump).unwrap());
    let m = mset[idx].as_ref().unwrap();
    for c in m.channels.iter().filter(|c| c.bone as usize == bone) {
        eprintln!("channel kind {} before {} after {}: {:?}", c.kind, c.before, c.after, c.keys.iter().map(|k| (k.time, (k.value * 1000.0).round() / 1000.0, k.interpolation, k.tangent_in * 256.0, k.tangent_out * 256.0)).collect::<Vec<_>>());
    }
    let mp = motions.iter().find(|(_, w)| w == &m.joint_words).unwrap().0;
    let mut seen = std::collections::BTreeMap::new();
    for f in frames.iter().filter(|f| f.tag == 2 && f.motion == mp) {
        let (_, q, _) = f.locals[bone].to_scale_rotation_translation();
        let (z, y, x) = q.to_euler(glam::EulerRot::ZYX);
        let s = m.apply(&kh1, f.t)[bone].rotation;
        seen.entry((f.t * 10.0) as i32).or_insert(format!("t {:5.1}: KH1 ({:.3}, {:.3}, {:.3}) ours ({:.3}, {:.3}, {:.3})", f.t, x, y, z, s[0], s[1], s[2]));
    }
    for l in seen.values() { eprintln!("{l}"); }
}

#[test]
fn kh1_world_tilt() {
    let Ok(dump) = std::env::var("KH1_POSEDUMP") else { return };
    if std::env::var("KH1_TILT").is_err() { return }
    let (motions, frames) = read_dump(&std::fs::read(dump).unwrap());
    let _ = motions;
    let mut last = 0u64;
    for f in frames.iter().filter(|f| f.tag == 2) {
        let w = f.world;
        let (x, y, z) = (w.x_axis.truncate(), w.y_axis.truncate(), w.z_axis.truncate());
        let tilt = y.normalize().dot(glam::Vec3::Y).abs().acos().to_degrees();
        if f.motion != last || (f.t as i32) % 8 == 0 {
            eprintln!("motion {:#x} t {:5.1}: world y-axis {:.0} deg off vertical; scale {:.2} {:.2} {:.2}", f.motion, f.t, tilt, x.length(), y.length(), z.length());
            last = f.motion;
        }
    }
}
