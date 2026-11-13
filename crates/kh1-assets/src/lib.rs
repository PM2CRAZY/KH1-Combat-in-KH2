//! # kh1-assets
//!
//! The build step (`docs/asset-build.md`): KH1's assets made once from the player's own KH1 and
//! KH2 files into the mod folder, for KH2 to load as its own. Plain Rust, run by the DLL on the
//! first launch and by the `kh1-assets` tool for checks.
//!
//! Both games' packages are in the collection's `Image\<lang>\` (`kh1_first..fifth`,
//! `kh2_first..sixth`), the same HD format and cipher (`kh2_formats::egs`), each game's key table
//! in its own executable. Files are found by the MD5 of their name.
//!
//! This first part (t10) only reads: KH1 Sora's motion set and skeleton, KH2 Sora's motion set and
//! skeleton, and reports what they hold.

pub mod convert;
pub mod player_model;

use kh1_data::{Kh1Install, Source};
use kh2_formats::bar::Bar;
use kh2_formats::egs::{Archive, KeyTable};
use kh2_formats::{Error, Result};
use std::path::{Path, PathBuf};

/// KH1 Sora (`xa_ex_0010`): motions and model. Checked (files): the player's extraction.
pub const KH1_SORA_MSET: &str = "xa_ex_0010.mset";
pub const KH1_SORA_MDLS: &str = "xa_ex_0010.mdls";
/// KH2 Sora: motions and model.
pub const KH2_SORA_MSET: &str = "obj/P_EX100.mset";
pub const KH2_SORA_MDLX: &str = "obj/P_EX100.mdlx";
/// The keyblade's motion set: slot for slot beside Sora's (839 entries, the same motion ids;
/// Checked (files)). KH2 plays the keyblade's motion for Sora's motion id, so every motion we add
/// to Sora needs one here (without it the keyblade vanished during KH1's swings, and switching
/// keyblades crashed).
pub const KH2_KEYBLADE_MSET: &str = "obj/W_EX010.mset";

pub const KH2_ARCHIVES: [&str; 6] = ["kh2_first", "kh2_second", "kh2_third", "kh2_fourth", "kh2_fifth", "kh2_sixth"];
pub const KH2_EXECUTABLE: &str = "KINGDOM HEARTS II FINAL MIX.exe";

/// KH2's packages, beside KH1's in the same collection folder.
pub struct Kh2Install {
    pub root: PathBuf,
    pub image_dir: PathBuf,
}

impl Kh2Install {
    pub fn locate(root: impl AsRef<Path>) -> Result<Kh2Install> {
        let root = root.as_ref().to_path_buf();
        if !root.join(KH2_EXECUTABLE).is_file() {
            return Err(Error::Format(format!("{KH2_EXECUTABLE} not found in {}", root.display())));
        }
        let image_dir = ["en", "dt", "jp"]
            .iter()
            .map(|l| root.join("Image").join(l))
            .find(|d| d.join("kh2_first.hed").is_file())
            .ok_or_else(|| Error::Format(format!("kh2_first.hed not found under {}", root.join("Image").display())))?;
        Ok(Kh2Install { root, image_dir })
    }

    pub fn key_table(&self) -> Result<KeyTable> {
        KeyTable::from_executable_cached(self.root.join(KH2_EXECUTABLE))
    }

    /// A file's remastered (HD) assets (names like `-0.dds`, with their bytes), from the
    /// unmodded package first.
    pub fn read_remastered_all(&self, name: &str) -> Result<Vec<kh2_formats::egs::HdAsset>> {
        let keys = self.key_table()?;
        for part in KH2_ARCHIVES {
            let backup = self.root.join("BackupImage").join(format!("{part}.hed"));
            let live = self.image_dir.join(format!("{part}.hed"));
            for hed in [backup, live] {
                if !hed.is_file() {
                    continue;
                }
                let archive = kh2_formats::egs::Archive::open_cached(&hed)?;
                if archive.find(name).is_some() {
                    return archive.read_remastered_all(&keys, name);
                }
            }
        }
        Err(Error::NotFound(name.to_string()))
    }

    /// Reads a file by its original name; the unmodded package OpenKH keeps in `BackupImage\`
    /// first, so a mod's edits don't leak into what we build.
    pub fn read(&self, name: &str) -> Result<Vec<u8>> {
        let keys = self.key_table()?;
        for part in KH2_ARCHIVES {
            let backup = self.root.join("BackupImage").join(format!("{part}.hed"));
            let live = self.image_dir.join(format!("{part}.hed"));
            for hed in [backup, live] {
                if !hed.is_file() {
                    continue;
                }
                let archive = Archive::open_cached(&hed)?;
                if archive.find(name).is_some() {
                    return archive.read(&keys, name);
                }
            }
        }
        Err(Error::NotFound(name.to_string()))
    }
}

/// What the two Soras' files hold.
#[derive(Debug, Default)]
pub struct Report {
    pub kh1_motions: usize,
    pub kh1_motion_slots: usize,
    pub kh1_bones: usize,
    pub kh1_frames_total: u64,
    pub kh2_mset_bytes: usize,
    pub kh2_mset_entries: usize,
    pub kh2_bones: usize,
}

/// Reads both Soras' motion sets and skeletons from the collection at `root` (the folder holding
/// both games' executables).
pub fn scan(root: impl AsRef<Path>) -> Result<Report> {
    let root = root.as_ref();
    let kh1 = Kh1Install::locate(root)?;
    let kh2 = Kh2Install::locate(root)?;
    let mut r = Report::default();
    let mset = kh1.read(Source::Original, KH1_SORA_MSET)?;
    let motions = kh1_data::mset::parse(&mset)?;
    r.kh1_motion_slots = motions.len();
    for m in motions.iter().flatten() {
        r.kh1_motions += 1;
        r.kh1_frames_total += m.frames as u64;
    }
    let mdls = kh1.read(Source::Original, KH1_SORA_MDLS)?;
    r.kh1_bones = kh1_data::mdls::parse_skeleton(&mdls)?.len();
    let mset2 = kh2.read(KH2_SORA_MSET)?;
    r.kh2_mset_bytes = mset2.len();
    r.kh2_mset_entries = Bar::parse(&mset2)?.entries.len();
    let mdlx = kh2.read(KH2_SORA_MDLX)?;
    r.kh2_bones = kh2_formats::anim::parse_skeleton(&mdlx)?.len();
    Ok(r)
}

/// KH1 Sora's pose for motion `index`, every frame (KH1's whole skeleton pass, `kh1_data::rig`):
/// a JSON text with the parents, then per frame each bone's model-space position, for checks.
pub fn kh1_pose_json(root: impl AsRef<Path>, index: usize) -> Result<String> {
    let kh1 = Kh1Install::locate(root.as_ref())?;
    let mset = kh1.read(Source::Original, KH1_SORA_MSET)?;
    let motions = kh1_data::mset::parse(&mset)?;
    let m = motions.get(index).cloned().flatten().ok_or_else(|| Error::NotFound(format!("KH1 motion {index}")))?;
    let mdls = kh1.read(Source::Original, KH1_SORA_MDLS)?;
    let bones = kh1_data::mdls::parse_skeleton(&mdls)?;
    let mut out = String::new();
    out.push_str(&format!("{{\"motion\":{index},\"frames\":{},\"fps\":{},\"helpers\":{},\"constants\":{},\"parents\":[", m.frames, m.fps, m.helper_table.len(), m.constants.len()));
    out.push_str(&bones.iter().map(|b| b.parent.to_string()).collect::<Vec<_>>().join(","));
    out.push_str("],\"words\":[");
    out.push_str(&m.joint_words.iter().map(|w| w.to_string()).collect::<Vec<_>>().join(","));
    out.push_str("],\"poses\":[");
    let mut first = true;
    for f in 0..m.frames.max(1) {
        let p = kh1_data::rig::pose(&m, &bones, f as f32);
        if !first { out.push(','); }
        first = false;
        let pts: Vec<String> = p.bones.iter().map(|b| { let t = b.w_axis; format!("[{:.2},{:.2},{:.2}]", t.x, t.y, t.z) }).collect();
        let hs: Vec<String> = p.helpers.iter().map(|b| { let t = b.w_axis; format!("[{:.2},{:.2},{:.2}]", t.x, t.y, t.z) }).collect();
        out.push_str(&format!("{{\"b\":[{}],\"h\":[{}]}}", pts.join(","), hs.join(",")));
    }
    out.push_str("]}");
    Ok(out)
}

/// KH1 Sora's combo motions built into KH2's motion set, and the KH2 motion id each gets.
/// KH1's ground combo 98..101 (three swings, the finisher: travels 100 / 117 / 135 / 186, Checked
/// (files)) and air combo 102..104 (the sister project's notes name 98 / 101 / 102 / 104 the
/// plain swing, finisher, air swing and air finisher, Checked (decomp) there). KH2's
/// `P_EX100.mset` holds 839 slots (motion × 4 + relative): ours start at motion 210.
pub const KH1_MOTIONS: [usize; 7] = [98, 99, 100, 101, 102, 103, 104];
pub const FIRST_KH2_MOTION: usize = 210;
/// KH2's first ground swing (motion 151, `A300`): the template motion and, until KH1's own windows
/// are built (Phase 2 step 4), the triggers.
pub const TEMPLATE_MOTION: usize = 151;
/// The KH2 swing each KH1 motion takes the place of (`combos.rs`'s rows): the keyblade plays
/// that swing's own keyblade motion during the KH1 one.
pub const REPLACES: [(usize, usize); 7] = [(98, 151), (99, 152), (100, 153), (101, 154), (102, 181), (103, 182), (104, 184)];
/// Test build (t17, off from t19): KH1 motion 98's slot holds KH2's own first swing (A300) passed through the
/// converter, so the owner can see in game whether the converter itself keeps a KH2 swing whole.
pub const ROUND_TRIP_FIRST_SWING: bool = false;
/// Test build (t22): KH1 motion 98's slot holds KH2's own rest pose (the `.mdlx` bind), held
/// still, so the owner can see whether KH2's mesh sits right on the rest's bone frames (the
/// transfer turns every bone from them; KH2's own motions use other frames for 175 bones).
pub const REST_POSE_FIRST_SWING: bool = false;

/// The KH2 motion id KH1 motion `kh1` is built as.
pub fn kh2_motion_of(kh1: usize) -> Option<usize> {
    KH1_MOTIONS.iter().position(|&m| m == kh1).map(|i| FIRST_KH2_MOTION + i)
}

/// What a build made.
#[derive(Debug, Default)]
pub struct Built {
    pub motions: Vec<(usize, usize, usize)>,
    pub bytes: usize,
}

/// KH2 Sora's motion set with KH1's combo motions added, from the collection at `root`.
pub fn build_mset(root: impl AsRef<Path>) -> Result<(Vec<u8>, Built)> {
    let root = root.as_ref();
    let kh1 = Kh1Install::locate(root)?;
    let kh2 = Kh2Install::locate(root)?;
    let mset1 = kh1.read(Source::Original, KH1_SORA_MSET)?;
    let motions = kh1_data::mset::parse(&mset1)?;
    let kh1_bones = kh1_data::mdls::parse_skeleton(&kh1.read(Source::Original, KH1_SORA_MDLS)?)?;
    let kh2_bones = kh2_formats::anim::parse_skeleton(&kh2.read(KH2_SORA_MDLX)?)?;
    let mset2 = kh2.read(KH2_SORA_MSET)?;
    let bar = Bar::parse(&mset2)?;
    let tslot = bar.entries.get(TEMPLATE_MOTION * 4).ok_or_else(|| Error::Format("KH2's template motion is missing".into()))?;
    let tanb = Bar::parse(tslot.data)?;
    let tmotion = tanb.entries.iter().find(|e| e.kind == kh2_formats::motion::BAR_TYPE_MOTION).ok_or_else(|| Error::Format("template has no motion".into()))?;
    let ttrig = tanb.entries.iter().find(|e| e.kind == kh2_formats::motion::BAR_TYPE_MOTION_TRIGGERS).map(|e| e.data.to_vec()).unwrap_or_default();
    let mut extra: Vec<(u16, i16, [u8; 4], Option<Vec<u8>>)> = Vec::new();
    let dumm = (0u16, 1i16, *b"DUMM", None);
    while bar.entries.len() + extra.len() < FIRST_KH2_MOTION * 4 {
        extra.push(dumm.clone());
    }
    let mut built = Built::default();
    for &k in &KH1_MOTIONS {
        let m = motions.get(k).cloned().flatten().ok_or_else(|| Error::NotFound(format!("KH1 motion {k}")))?;
        let (poses, travel) = if REST_POSE_FIRST_SWING && k == KH1_MOTIONS[0] {
            let rest = kh2_anim::Pose::bind(&kh2_bones).bones;
            (vec![rest; 48], vec![glam::Vec3::ZERO; 48])
        } else if ROUND_TRIP_FIRST_SWING && k == KH1_MOTIONS[0] {
            convert::kh2_round_trip(tmotion.data, &kh2_bones)?
        } else {
            convert::kh2_poses(&m, &kh1_bones, &kh2_bones)
        };
        let data = convert::write_motion(tmotion.data, &kh2_bones, &poses, &travel)?;
        let name = *format!("K{k:03}").as_bytes().first_chunk::<4>().unwrap();
        let anb = convert::write_bar(0, &[(kh2_formats::motion::BAR_TYPE_MOTION, 0, name, data), (kh2_formats::motion::BAR_TYPE_MOTION_TRIGGERS, 0, name, ttrig.clone())]);
        let id = kh2_motion_of(k).unwrap();
        built.motions.push((k, id, poses.len()));
        extra.push((kh2_formats::motion::BAR_TYPE_ANB, 0, name, Some(anb)));
        for _ in 0..3 {
            extra.push(dumm.clone());
        }
    }
    let out = convert::extend_bar(&mset2, &extra)?;
    built.bytes = out.len();
    Ok((out, built))
}

/// KH2's keyblade motion set with an entry for each motion we add to Sora: a copy of the
/// keyblade motion of the KH2 swing it takes the place of.
pub fn build_keyblade(root: impl AsRef<Path>) -> Result<Vec<u8>> {
    let kh2 = Kh2Install::locate(root.as_ref())?;
    let orig = kh2.read(KH2_KEYBLADE_MSET)?;
    let bar = Bar::parse(&orig)?;
    let mut extra: Vec<(u16, i16, [u8; 4], Option<Vec<u8>>)> = Vec::new();
    let dumm = (0u16, 1i16, *b"DUMM", None);
    while bar.entries.len() + extra.len() < FIRST_KH2_MOTION * 4 {
        extra.push(dumm.clone());
    }
    for &k in &KH1_MOTIONS {
        let kh2_id = REPLACES.iter().find(|r| r.0 == k).map(|r| r.1).ok_or_else(|| Error::NotFound(format!("the KH2 swing KH1 motion {k} replaces")))?;
        for rel in 0..4 {
            match bar.entries.get(kh2_id * 4 + rel) {
                Some(e) if e.name != "DUMM" && !e.data.is_empty() => {
                    let mut n = [0u8; 4];
                    for (i, c) in e.name.bytes().take(4).enumerate() {
                        n[i] = c;
                    }
                    extra.push((e.kind, 0, n, Some(e.data.to_vec())));
                }
                _ => extra.push(dumm.clone()),
            }
        }
    }
    convert::extend_bar(&orig, &extra)
}

/// The build's stamp: what it was made from (the mod's version, both games' entries and which
/// player model), so a later launch builds again only when one changed.
pub fn stamp(root: impl AsRef<Path>, version: &str, kh1_model: bool) -> String {
    let root = root.as_ref();
    let k1 = Kh1Install::locate(root).map(|k| format!("{} / {}", k.stamp(Source::Original, KH1_SORA_MSET), k.stamp(Source::Original, KH1_SORA_MDLS))).unwrap_or_default();
    format!("kh2-kh1 {version}; KH1 {k1}; motions {KH1_MOTIONS:?} from {FIRST_KH2_MOTION}; model {}", if kh1_model { "kh1" } else { "kh2" })
}

/// KH1 Sora as KH2's player: his model (`obj/P_EX100.mdlx`, `player_model::build`), KH2's whole
/// motion set moved onto his skeleton with KH1's combo motions added (`player_model::build_mset`),
/// and his HD textures, named for the new model (`-0.dds`, ... in texture order).
pub fn build_kh1_model(root: impl AsRef<Path>) -> Result<(Vec<u8>, Vec<u8>, Vec<(String, Vec<u8>)>, Built)> {
    let root = root.as_ref();
    let kh1 = Kh1Install::locate(root)?;
    let kh2 = Kh2Install::locate(root)?;
    let mdls = kh1.read(Source::Original, KH1_SORA_MDLS)?;
    let mdlx = kh2.read(KH2_SORA_MDLX)?;
    let model = player_model::build(&mdls, &mdlx)?;
    let kh1_bones = kh1_data::mdls::parse_skeleton(&mdls)?;
    let kh2_bones = kh2_formats::anim::parse_skeleton(&mdlx)?;
    let motions = kh1_data::mset::parse(&kh1.read(Source::Original, KH1_SORA_MSET)?)?;
    let (mset, n) = player_model::build_mset(&kh2.read(KH2_SORA_MSET)?, &kh2_bones, &kh1_bones, &motions)?;
    // KH1's HD textures sit at the PS2 textures' offsets in the .mdls (-<offset>.dds): sorted by
    // offset, they are the textures in order. Missing ones leave KH2's PS2 upscale to the game.
    let mut hd: Vec<_> = kh1.read_remastered_all(Source::Original, KH1_SORA_MDLS).unwrap_or_default().into_iter().filter(|a| a.name.ends_with(".dds") && u32::from_str_radix(a.name.trim_start_matches('-').trim_end_matches(".dds"), 16).is_ok()).collect();
    hd.sort_by_key(|a| u32::from_str_radix(a.name.trim_start_matches('-').trim_end_matches(".dds"), 16).unwrap_or(0));
    // (The HD mesh's own textures, -1c*/-2c*, have small names; they are not these.)
    hd.retain(|a| u32::from_str_radix(a.name.trim_start_matches('-').trim_end_matches(".dds"), 16).unwrap_or(0) >= 0x1000);
    let count = kh1_data::mdls::parse_mesh(&mdls)?.1.len();
    let textures = if hd.len() == count { hd.into_iter().enumerate().map(|(i, a)| (format!("-{i}.dds"), a.data)).collect() } else { Vec::new() };
    let built = Built { motions: KH1_MOTIONS.iter().map(|&k| (k, kh2_motion_of(k).unwrap(), n)).collect(), bytes: mset.len() + model.len() };
    Ok((model, mset, textures, built))
}

/// Builds into `mod_dir` (OpenKH's `mod\kh2`): `obj/P_EX100.mset`, `obj/W_EX010.mset` and the
/// stamp, and with `kh1_model` KH1 Sora's `obj/P_EX100.mdlx` and HD textures
/// (`remastered/obj/P_EX100.mdlx/`), unless the stamp says the files there are already this
/// build's. Returns what was built (`None`: up to date).
pub fn build_into(root: impl AsRef<Path>, mod_dir: impl AsRef<Path>, version: &str, kh1_model: bool) -> Result<Option<Built>> {
    let (root, mod_dir) = (root.as_ref(), mod_dir.as_ref());
    let obj = mod_dir.join("obj");
    let out = obj.join("P_EX100.mset");
    let mdlx = obj.join("P_EX100.mdlx");
    let hd_dir = mod_dir.join("remastered").join("obj").join("P_EX100.mdlx");
    let stamp_file = mod_dir.join("kh2-kh1-build.txt");
    let want = stamp(root, version, kh1_model);
    if out.is_file() && obj.join("W_EX010.mset").is_file() && mdlx.is_file() == kh1_model && std::fs::read_to_string(&stamp_file).ok().as_deref() == Some(want.as_str()) {
        return Ok(None);
    }
    let io = |e: std::io::Error| Error::Format(e.to_string());
    let blade = build_keyblade(root)?;
    let mut files: Vec<(std::path::PathBuf, Vec<u8>)> = vec![(obj.join("W_EX010.mset"), blade)];
    let built = if kh1_model {
        let (model, mset, textures, built) = build_kh1_model(root)?;
        files.push((out.clone(), mset));
        files.push((mdlx.clone(), model));
        for (name, data) in textures {
            files.push((hd_dir.join(name), data));
        }
        built
    } else {
        let (bytes, built) = build_mset(root)?;
        files.push((out.clone(), bytes));
        let _ = std::fs::remove_file(&mdlx);
        let _ = std::fs::remove_dir_all(&hd_dir);
        built
    };
    for (path, data) in &files {
        std::fs::create_dir_all(path.parent().unwrap()).map_err(io)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, data).map_err(io)?;
        std::fs::rename(&tmp, path).map_err(io)?;
    }
    std::fs::write(&stamp_file, want).map_err(io)?;
    Ok(Some(built))
}
