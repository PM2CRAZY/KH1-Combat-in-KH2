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
pub mod swing_triggers;

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
/// KH2's Keyblade motion set (every Keyblade's; Checked (files): no other `W_EX010_*.mset`).
pub const KH2_KEYBLADE_MSET: &str = "obj/W_EX010.mset";
/// KH2's Kingdom Key: its skeleton (12 bones in one chain) stands for every Keyblade's, which
/// all play `W_EX010.mset` (Checked (files): `W_EX010_10.mdlx` has the same chain).
pub const KH2_KEYBLADE_MDLX: &str = "obj/W_EX010.mdlx";
/// The keyblade's motion set: slot for slot beside Sora's (839 entries, the same motion ids;
/// Checked (files)). KH2 plays the keyblade's motion for Sora's motion id, so every motion we add
/// to Sora needs one here (without it the keyblade vanished during KH1's swings, and switching
/// keyblades crashed).

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

/// KH1 Sora's combo motions: the ground combo 98..101 (three swings, the finisher: travels 100 /
/// 117 / 135 / 186, Checked (files)) and air combo 102..104 (the sister project's notes name 98 /
/// 101 / 102 / 104 the plain swing, finisher, air swing and air finisher, Checked (decomp) there).
pub const KH1_MOTIONS: [usize; 7] = [98, 99, 100, 101, 102, 103, 104];

/// The KH2 swing slots KH1's motions are written into (KH2 motion id, KH1 motion), in place of
/// KH2's swing: each slot keeps its own motion triggers, and KH2's combo picker (`ptya`) and the
/// Keyblade's own motion of that id (`W_EX010.mset`, unchanged) play with it. Ground: ids 152
/// (row 4) → 99, 151 (row 5, the fallback) → 98, 153 (row 6) → 100, finishers 154 / 155 (rows 2
/// / 3) → 101; air: 181 (row 9, the fallback) → 102, 182 (row 10) → 103, finishers 184 / 183
/// (rows 7 / 8) → 104 (the t12 row mapping, `docs/combos.md`).
///
/// Until t32 the KH1 motions were added after KH2's (ids 210..216), the `ptya` rows pointed at them
/// and the Keyblade's motion set got copies of the swings' Keyblade motions. Switching Keyblades
/// then crashed (`0x1403B68E0`: an object whose collision list had been overwritten, Checked
/// (log, t31/t32)); with KH2's own Keyblade motion set (`combos = kh2`) it did not. Writing the
/// KH1 motions into the swings' own slots leaves the Keyblade's file and the `ptya` table as KH2's.
pub const SWINGS: [(usize, usize); 9] = [(151, 98), (152, 99), (153, 100), (154, 101), (155, 101), (181, 102), (182, 103), (184, 104), (183, 104)];

/// KH1's guard written into KH2's guard slot (`defence = kh1`, Phase 3): KH2 motion 173 (`A322`,
/// combo row 11, its Keyblade guard window ATKP 161, Checked (files)) ← KH1 motion 110 (request
/// `0xD4`, attack row `0x3D`: KH1's guard is an attack object of priority 100 live in the motion's
/// hit window, frames 10–36; the sister project's `docs/kh1-roll-guard-native.md` §3.3, the events
/// Checked (files)). The Keyblade's guard window moves to KH1's frames like a swing's hit window.
pub const GUARD: [(usize, usize); 1] = [(173, 110)];

/// KH1's Dodge Roll in KH2's (`defence = kh1`): KH2's roll is motion 209 (`0x1403A81F0` requests
/// `0xD1` for growth kind 4, Checked (decomp)), whose motion is row 838 (209 × 4 + 2, `A180`; rows
/// 836 / 837 / 839 are DUMMs, Checked (files)) ← KH1 motion 117 (request `0xDC`, slot 117, kind 2,
/// travel by its root: the sister project's `kh1-roll-guard-native.md` §5, Checked (files) there;
/// here: 38 frames, 350 units forward by frame 30, "may be left" (event kind 0) at 30, Checked
/// (files)). KH2's own triggers move with it (`swing_triggers::sora`: KH2's blend-to-idle, range
/// 2 at 36, onto KH1's 30). Rows, not ids.
pub const ROLL: [(usize, usize); 1] = [(838, 117)];

/// KH1's hit reactions in KH2's (`defence = kh1`), rows ← KH1 motions. KH2's reaction routine
/// `0x1403D2FE0` plays motion 10 / 12 for a ground flinch from the front / behind (11 / 13 the
/// other hit part) and 14 / 15 in the air (the sister project's `kh1-hit-reactions-native.md` §5
/// and §9, Checked (decomp) there); their motions are rows 42 / 46 / 50 / 54 / 58 / 62 (id × 4 + 2,
/// the rest DUMMs, Checked (files)). KH1's flinch requests `0x48` front, `0x49` the other part,
/// `0x4A` behind, `0x4B`, its air flinch `0x4E` / `0x4F` are slots 61..64, 67, 68 (§9, Checked
/// (files) there; here: 70 frames each, "may be left" at 46; the air ones 30 frames). KH1 picks the
/// front one by the same test as KH2 (`dot(facing, push) < 0`). KH2's free state (range 2 at 50)
/// moves onto KH1's 46 (`swing_triggers::free_anchor`). KH2's launches (16 / 18, landing 17 / 19)
/// stay KH2's: KH1 knocks Sora down in place there (slots 65 / 66), a different reaction.
pub const REACTIONS: [(usize, usize); 6] = [(42, 61), (46, 62), (50, 63), (54, 64), (58, 67), (62, 68)];

/// KH1's knock-down in place (the owner, 2026-11-13: big hits knock Sora down where he stands, as
/// KH1): KH1's ground knock-back requests `0x4C` front / `0x4D` behind are slots 65 / 66 (72
/// frames, in place, "may be left" at 58; Checked (files)). KH2 has no such reaction, so they get
/// motions of their own, KH2 Sora's free ids 20 / 21 (every row of both DUMMs in `P_EX100.mset` and
/// `W_EX010.mset`, Checked (files); inside KH2's damage motions 10..39): rows 82 / 86, each made on
/// KH2's front flinch (row 42, `A200`) as its template, its triggers moved onto KH1's (free state
/// 50 → 58). `kh2-hook`'s `reactions` plays them. (row, template row, KH1 motion).
pub const KNOCKDOWN: [(usize, usize, usize); 2] = [(82, 42, 65), (86, 42, 66)];

/// The motion id KH2 plays for KH1's knock-down from the front / from behind (`KNOCKDOWN`).
pub const KNOCKDOWN_IDS: [u16; 2] = [20, 21];

/// Rows whose KH1 motion ends in KH1's 8-frame blend into the motion KH2 plays after it
/// (`convert::append_blend`; the owner, t54: the air hit doesn't flow into the fall, the landing
/// into the idle). (row, the next KH1 motion). KH2's landing has one row (22, its field one); a
/// battle row of ours (20, `LANDING_BATTLE`) ends in the battle idle instead.
/// The air flinches blend into the fall row's first pose, KH1's top (`PRELUDE`, t56).
pub const TAILS: [(usize, usize); 4] = [(22, 0), (20, 1), (58, 9), (62, 9)];
/// KH1's blend length (`0x14029FA30(.., 8.0)`).
pub const KH1_BLEND: usize = 8;

/// KH1's landing on KH2's landing's battle variant (row 20 = 5 × 4 + 0; KH2 plays .0 first in
/// battle with the Keyblade out, `MOVES`' notes), made on the field row 22 as its template.
pub const LANDING_BATTLE: (usize, usize, usize) = (20, 22, 11);

/// KH1's jump as KH1 plays it (the owner, t55: "when jumping without high jump equipped, sora does
/// the jump animation a second time mid air"). KH2's jump (row 14) and High Jump (806) loop (their
/// returns 12 of 8..21, 21 of 4..36; Checked (files)) while he rises, so ours went back into the
/// jump. KH1 plays its jump once and, at the top, its top (request 5, slot 9: 14 frames) and then
/// its fall (request 6, slot 10) queued after it (KH1 `0x1402C7AB0`: `0x14029F7C0(.., 5, 0)`,
/// `0x14029FA30(.., 6, 0)`; the state-2 step changes the slot at the top, the sister project's
/// `animations.md`; Checked (decomp) for the pair). KH2 starts its fall (motion 4, row 18) at
/// the top, so: the rises hold their last frame, and the fall row is KH1's top then its fall, the
/// fall looping on its own. Not 1:1: a fall off a ledge plays the top first too (KH1 starts it at
/// its fall); the High Jump's own top (slot 13) isn't played (KH2's one fall row).
pub const PRELUDE: [(usize, usize); 1] = [(18, 9)];

/// KH1's motion played before `row`'s own in it (`PRELUDE`).
pub fn prelude_at(row: usize) -> Option<usize> {
    PRELUDE.iter().find(|p| p.0 == row).map(|p| p.1)
}

/// How `row`'s KH1 motion ends (`PRELUDE`'s notes): the rises held, the fall looping after the top
/// (`pre` frames, the prelude's length).
pub fn return_at(row: usize, pre: usize) -> convert::Return {
    match row {
        14 | 806 => convert::Return::End,
        18 if pre > 0 => convert::Return::Frame(pre),
        _ => convert::Return::Template,
    }
}

/// The next motion's tail for `row`, if any.
pub fn tail_at(row: usize) -> Option<usize> {
    TAILS.iter().find(|t| t.0 == row).map(|t| t.1)
}

/// The empty rows the build fills (row, template row): `KNOCKDOWN`'s, when chosen.
pub fn fills() -> Vec<(usize, usize)> {
    KNOCKDOWN.iter().chain(std::iter::once(&LANDING_BATTLE)).filter(|k| kh1_motion_at(k.0).is_some()).map(|k| (k.0, k.1)).collect()
}

/// KH1's own idle, walk, run, jump, fall and landing in KH2's (`movement = kh1`). Motion set rows
/// (KH2 id × 4 + variant), KH1 motion.
///
/// KH2's variants (OpenKH `MotionSet.GetMotionSetIndex`, Assumption from it; the t48 video agrees:
/// row 2 plays in the field): KH2 tries, in order, .0 .1 .3 .2 in battle with the weapon out,
/// .1 .0 .2 .3 in battle without, .2 .3 .1 .0 in the field without, .3 .2 .0 .1 in the field with
/// it, and plays the first that isn't a DUMM. KH2 Sora's (Checked (files)): idle 0 (rows 0, 2),
/// walk 1 (4, 6, 7), run 2 (8, 10, 11), jump 3 (14), fall 4 (18), land 5 (22): .0 his battle
/// motion, .2 his field one, .3 the field with the Keyblade out.
///
/// KH1's (Checked (decomp): KH1's request resolver `0x1402A01E0`): the idle (request 0, kind 4)
/// is slot 0, slot 1 in battle (the actor's `+0x374` bit 18); walk and run (kind 3) are slot +1
/// out of battle and +2 in it, +0 only for an actor with a handle at `+0x14C` and without flag
/// `0x4000` (Assumption: never the player in normal play). So: 0 the field idle (120 frames,
/// standing), 1 the battle idle (170, the Keyblade stance), 3 / 6 the field walk and run, 4 / 7
/// the battle ones (Checked (files): 4 and 7 crouch like the battle idle, `kh1_cycle_stances`).
/// KH1's battle set goes on KH2's battle rows, its field set on the field rows (t50 kept KH2's run
/// on row 10; the owner, t54: "it should be the KH1 out of battle run"). 8 the jump (26),
/// 10 the fall (40, held), 11 the landing (28), one each in KH1, on KH2's one row each. The owner
/// (t42): 12 is KH1's High Jump: it goes on KH2's High Jump, motion 201, whose motion is its row
/// 806 (201 × 4 + 2; rows 804 / 805 / 807 are DUMMs, Checked (files)). KH1's 9 / 13 (the jumps'
/// tops) have no KH2 row.
pub const MOVES: [(usize, usize); 12] = [(0, 1), (2, 0), (4, 4), (6, 3), (7, 3), (8, 7), (10, 6), (11, 6), (14, 8), (18, 10), (22, 11), (806, 12)];

static SLOTS: std::sync::RwLock<Option<Vec<(usize, usize)>>> = std::sync::RwLock::new(None);

thread_local! {
    /// The return the KH2-skeleton build picked for the motion it is writing.
    static RETURN: std::cell::Cell<convert::Return> = const { std::cell::Cell::new(convert::Return::Template) };
}

/// The motion set rows this build writes KH1 motions into (row = KH2 id × 4 + variant, KH1
/// motion): the swings with `combos = kh1`, the guard with `defence = kh1`, the moves with
/// `movement = kh1`. Until chosen, the swings.
pub fn kh1_slots() -> Vec<(usize, usize)> {
    SLOTS.read().ok().and_then(|s| s.clone()).unwrap_or_else(|| SWINGS.iter().map(|&(id, k)| (id * 4, k)).collect())
}

/// Chooses `kh1_slots` for a build.
pub fn select_kh1_slots(swings: bool, guard: bool, moves: bool) {
    let mut v: Vec<(usize, usize)> = Vec::new();
    if swings {
        v.extend(SWINGS.iter().map(|&(id, k)| (id * 4, k)));
    }
    if guard {
        v.extend(GUARD.iter().map(|&(id, k)| (id * 4, k)));
        v.extend_from_slice(&ROLL);
        v.extend_from_slice(&REACTIONS);
        v.extend(KNOCKDOWN.iter().map(|k| (k.0, k.2)));
    }
    if moves {
        v.extend_from_slice(&MOVES);
        v.push((LANDING_BATTLE.0, LANDING_BATTLE.2));
    }
    if let Ok(mut s) = SLOTS.write() {
        *s = Some(v);
    }
}

/// The KH1 motion written into motion set row `row`, if any.
pub fn kh1_motion_at(row: usize) -> Option<usize> {
    kh1_slots().iter().find(|s| s.0 == row).map(|s| s.1)
}

/// Test build (t17, off from t19): KH1 motion 98's slot holds KH2's own first swing (A300) passed
/// through the converter, so the owner can see in game whether the converter keeps a KH2 swing
/// whole.
pub const ROUND_TRIP_FIRST_SWING: bool = false;
/// Test build (t22): KH1 motion 98's slot holds KH2's own rest pose (the `.mdlx` bind), held
/// still, so the owner can see whether KH2's mesh sits right on the rest's bone frames.
pub const REST_POSE_FIRST_SWING: bool = false;

/// What a build made: (KH1 motion, KH2 motion id, frames) for each swing written.
#[derive(Debug, Default)]
pub struct Built {
    pub motions: Vec<(usize, usize, usize)>,
    pub bytes: usize,
}

/// KH2 Sora's motion set with KH1's combo motions written into KH2's swing slots (`SWINGS`), on
/// KH2's skeleton, from the collection at `root`.
pub fn build_mset(root: impl AsRef<Path>) -> Result<(Vec<u8>, Built)> {
    use kh2_formats::motion::{BAR_TYPE_ANB, BAR_TYPE_MOTION};
    let root = root.as_ref();
    let kh1 = Kh1Install::locate(root)?;
    let kh2 = Kh2Install::locate(root)?;
    let motions = kh1_data::mset::parse(&kh1.read(Source::Original, KH1_SORA_MSET)?)?;
    let kh1_bones = kh1_data::mdls::parse_skeleton(&kh1.read(Source::Original, KH1_SORA_MDLS)?)?;
    let kh2_bones = kh2_formats::anim::parse_skeleton(&kh2.read(KH2_SORA_MDLX)?)?;
    let mset2 = kh2.read(KH2_SORA_MSET)?;
    let spans = swing_triggers::keyblade_spans(&kh2.read(KH2_KEYBLADE_MSET)?)?;
    let mut built = Built::default();
    let out = convert::map_bar_filled(&mset2, &fills(), |row, kind, data| {
        let Some(k) = kh1_motion_at(row).filter(|_| kind == BAR_TYPE_ANB && !data.is_empty()) else { return Ok(data.to_vec()) };
        let m = motions.get(k).cloned().flatten().ok_or_else(|| Error::NotFound(format!("KH1 motion {k}")))?;
        let len = swing_triggers::anb_frames(data);
        convert::map_bar(data, |_, kind, own| {
            if kind == kh2_formats::motion::BAR_TYPE_MOTION_TRIGGERS && !own.is_empty() {
                return Ok(swing_triggers::sora(&kh2_formats::motion::MotionTriggers::parse(own)?, spans.get(&row).copied(), len, &m).write());
            }
            if kind != BAR_TYPE_MOTION {
                return Ok(own.to_vec());
            }
            let (poses, travel) = if REST_POSE_FIRST_SWING && k == KH1_MOTIONS[0] {
                let rest = kh2_anim::Pose::bind(&kh2_bones).bones;
                (vec![rest; 48], vec![glam::Vec3::ZERO; 48])
            } else if ROUND_TRIP_FIRST_SWING && k == KH1_MOTIONS[0] {
                convert::kh2_round_trip(own, &kh2_bones)?
            } else {
                let (mut poses, mut travel) = convert::kh2_poses(&m, &kh1_bones, &kh2_bones);
                // KH1's motion before this one in the row (`PRELUDE`), its travel carried on.
                let pre_frames = if let Some(pm) = prelude_at(row).and_then(|n| motions.get(n).and_then(|x| x.as_ref())) {
                    let (mut pp, mut pt) = convert::kh2_poses(pm, &kh1_bones, &kh2_bones);
                    let base = pt.last().copied().unwrap_or(glam::Vec3::ZERO);
                    let on = glam::Mat4::from_translation(base);
                    let n = pp.len();
                    pp.extend(poses.iter().map(|p| p.iter().map(|m| on * *m).collect::<Vec<_>>()));
                    pt.extend(travel.iter().map(|t| base + *t));
                    poses = pp;
                    travel = pt;
                    n
                } else {
                    0
                };
                RETURN.with(|r| r.set(return_at(row, pre_frames)));
                // KH1's blend into the next motion, where KH2 doesn't blend (`TAILS`).
                if let Some(next) = tail_at(row).and_then(|n| motions.get(n).and_then(|x| x.as_ref())) {
                    let (np, nt) = convert::kh2_poses(next, &kh1_bones, &kh2_bones);
                    if let (Some(p0), Some(&t0)) = (np.first(), nt.first()) {
                        convert::append_blend(&mut poses, &mut travel, p0, t0, &kh2_bones, KH1_BLEND);
                    }
                }
                (poses, travel)
            };
            built.motions.push((k, row / 4, poses.len()));
            let mut out = convert::write_motion(own, &kh2_bones, &poses, &travel)?;
            convert::set_return_for(&mut out, RETURN.with(|r| r.replace(convert::Return::Template)));
            Ok(out)
        })
    })?;
    if built.motions.len() != kh1_slots().len() {
        return Err(Error::Format(format!("KH2's motion set had {} of the {} KH1 slots", built.motions.len(), kh1_slots().len())));
    }
    built.bytes = out.len();
    Ok((out, built))
}

/// The build's stamp: what it was made from (the mod's version, both games' entries and which
/// player model), so a later launch builds again only when one changed.
pub fn stamp(root: impl AsRef<Path>, version: &str, kh1_model: bool, kh1_swings: bool) -> String {
    let root = root.as_ref();
    let k1 = Kh1Install::locate(root).map(|k| format!("{} / {}", k.stamp(Source::Original, KH1_SORA_MSET), k.stamp(Source::Original, KH1_SORA_MDLS))).unwrap_or_default();
    format!("kh2-kh1 {version}; KH1 {k1}; KH1 slots {:?} (KH1 windows); model {}", { let _ = kh1_swings; kh1_slots() }, if kh1_model { "kh1" } else { "kh2" })
}

/// KH1 Sora as KH2's player: his model (`obj/P_EX100.mdlx`, `player_model::build`), KH2's whole
/// motion set moved onto his skeleton with KH1's combo motions added (`player_model::build_mset`),
/// and his HD textures, named for the new model (`-0.dds`, ... in texture order).
pub fn build_kh1_model(root: impl AsRef<Path>, kh1_swings: bool) -> Result<(Vec<u8>, Vec<u8>, Vec<(String, Vec<u8>)>, Built)> {
    let root = root.as_ref();
    let kh1 = Kh1Install::locate(root)?;
    let kh2 = Kh2Install::locate(root)?;
    let mdls = kh1.read(Source::Original, KH1_SORA_MDLS)?;
    let mdlx = kh2.read(KH2_SORA_MDLX)?;
    let model = player_model::build(&mdls, &mdlx)?;
    let kh1_bones = kh1_data::mdls::parse_skeleton(&mdls)?;
    let kh2_bones = kh2_formats::anim::parse_skeleton(&mdlx)?;
    let motions = kh1_data::mset::parse(&kh1.read(Source::Original, KH1_SORA_MSET)?)?;
    let spans = swing_triggers::keyblade_spans(&kh2.read(KH2_KEYBLADE_MSET)?)?;
    let (mset, swings) = player_model::build_mset(&kh2.read(KH2_SORA_MSET)?, &kh2_bones, &kh1_bones, &motions, kh1_swings, &spans)?;
    // KH1's HD textures sit at the PS2 textures' offsets in the .mdls (-<offset>.dds): sorted by
    // offset, they are the textures in order. Missing ones leave KH2's PS2 upscale to the game.
    let mut hd: Vec<_> = kh1.read_remastered_all(Source::Original, KH1_SORA_MDLS).unwrap_or_default().into_iter().filter(|a| a.name.ends_with(".dds") && u32::from_str_radix(a.name.trim_start_matches('-').trim_end_matches(".dds"), 16).is_ok()).collect();
    hd.sort_by_key(|a| u32::from_str_radix(a.name.trim_start_matches('-').trim_end_matches(".dds"), 16).unwrap_or(0));
    // (The HD mesh's own textures, -1c*/-2c*, have small names; they are not these.)
    hd.retain(|a| u32::from_str_radix(a.name.trim_start_matches('-').trim_end_matches(".dds"), 16).unwrap_or(0) >= 0x1000);
    let count = kh1_data::mdls::parse_mesh(&mdls)?.1.len();
    let textures = if hd.len() == count { hd.into_iter().enumerate().map(|(i, a)| (format!("-{i}.dds"), a.data)).collect() } else { Vec::new() };
    let built = Built { motions: swings, bytes: mset.len() + model.len() };
    Ok((model, mset, textures, built))
}

/// Builds into `mod_dir` (OpenKH's `mod\kh2`): `obj/P_EX100.mset`, `obj/W_EX010.mset` and the
/// stamp, and with `kh1_model` KH1 Sora's `obj/P_EX100.mdlx` and HD textures
/// (`remastered/obj/P_EX100.mdlx/`), unless the stamp says the files there are already this
/// build's. Returns what was built (`None`: up to date).
pub fn build_into(root: impl AsRef<Path>, mod_dir: impl AsRef<Path>, version: &str, kh1_model: bool, kh1_swings: bool, kh1_guard: bool, kh1_moves: bool) -> Result<Option<Built>> {
    select_kh1_slots(kh1_swings, kh1_guard, kh1_moves);
    let kh1_swings = !kh1_slots().is_empty();
    let (root, mod_dir) = (root.as_ref(), mod_dir.as_ref());
    let obj = mod_dir.join("obj");
    let out = obj.join("P_EX100.mset");
    let mdlx = obj.join("P_EX100.mdlx");
    let hd_dir = mod_dir.join("remastered").join("obj").join("P_EX100.mdlx");
    let stamp_file = mod_dir.join("kh2-kh1-build.txt");
    let want = stamp(root, version, kh1_model, kh1_swings);
    let blade = obj.join("W_EX010.mset");
    if !kh1_swings {
        let _ = std::fs::remove_file(&blade);
    }
    if out.is_file() && mdlx.is_file() == kh1_model && blade.is_file() == kh1_swings && std::fs::read_to_string(&stamp_file).ok().as_deref() == Some(want.as_str()) {
        return Ok(None);
    }
    let io = |e: std::io::Error| Error::Format(e.to_string());
    let mut files: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
    if kh1_swings {
        // The Keyblade's hit windows and trail at KH1's times (`swing_triggers`), same motions.
        let kh1 = Kh1Install::locate(root)?;
        let kh2 = Kh2Install::locate(root)?;
        let motions = kh1_data::mset::parse(&kh1.read(Source::Original, KH1_SORA_MSET)?)?;
        // With KH1's model, the Keyblade's motions too: KH1's own hold, in KH1's palm.
        let hold = if kh1_model { Some((kh1_data::mdls::parse_skeleton(&kh1.read(Source::Original, KH1_SORA_MDLS)?)?, kh2_formats::anim::parse_skeleton(&kh2.read(KH2_KEYBLADE_MDLX)?)?)) } else { None };
        files.push((blade.clone(), swing_triggers::build_keyblade(&kh2.read(KH2_KEYBLADE_MSET)?, &motions, hold.as_ref().map(|(a, b)| (a.as_slice(), b.as_slice())))?));
    }
    let built = if kh1_model {
        let (model, mset, textures, built) = build_kh1_model(root, kh1_swings)?;
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
