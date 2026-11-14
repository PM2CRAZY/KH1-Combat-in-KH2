//! KH1's spells as KH2 spell files (`magic = kh1`, `docs/mp-magic.md`): KH2 loads
//! `magic/FIRE_1.mag` (a BAR: a PAX of effects and a BDX script) and runs it with its own script
//! machine and effect engine. Ours hold KH1's own effect set and a script of ours that does what
//! KH1's spell script does, through KH2's own traps.
//!
//! - **Effects.** KH1 keeps a spell's effects as a DPX set in its `.mag` (section 1); KH2's spell
//!   PAX holds a DPX of the same layout (set header `0x82`, DPD `0x96` with the same five lists),
//!   and KH2's effect op table is KH1's 235 ops in the same order plus seven (Checked (files):
//!   both `.mag`s parse with KH2's DPD reader; Checked (exe): the op names). KH1's set goes in as
//!   it is, but for two things KH2 does elsewhere: KH1's sound ids on its programs (`+0xFC`, KH1's
//!   sound bank) are cleared, KH2 plays a spell's sounds from its PAX elements; KH1's hit masks on
//!   the set entries (`+0x10..`, KH1's lookups) are cleared, KH2 finds hit effects by the element
//!   tags.
//! - **Elements** (KH2's PAX, OpenKH `Pax.cs`; the tags and bones as KH2's own spells use them,
//!   Checked (files) `FIRE_1.mag` / `BLIZZARD_1.mag`): which KH1 effect plays for which script id.
//! - **Script** (`fire_script`): KH2's spell protocol (the sister's `magic-model.md`: trigger 20
//!   at the cast with the caster, resources, sub and `magi`; trigger 21 at the cast event), KH1's
//!   Fire (KH1-in-ReCom's `recom-m7-magic.md`, decoded from `xm_fire01.mag`, Checked (files)
//!   there): the ball from the Keyblade, 40 a frame, 30 frames, turning at most 3° a frame toward
//!   the target, radius 50, gone on its first hit; its trap calls are the ones KH2's Blizzard shard
//!   makes (Checked (files): `BLIZZARD_1.mag`'s script, read with `kh2_bdx::disasm`).

use kh2_bdx::asm::{Asm, Mode};

fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    b.get(o..o + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap()))
}

/// A KH1 `.mag`'s effect set (section 1: the file's second offset to its end), made ready for a
/// KH2 PAX: program sounds and set-entry masks cleared.
pub fn kh1_effect_set(kh1_mag: &[u8]) -> Option<Vec<u8>> {
    if u32_at(kh1_mag, 0)? < 2 {
        return None;
    }
    let start = u32_at(kh1_mag, 8)? as usize;
    let mut set = kh1_mag.get(start..)?.to_vec();
    if u32_at(&set, 0)? != 0x82 {
        return None;
    }
    let n = u32_at(&set, 12)? as usize;
    let mut dpds = Vec::new();
    for i in 0..n.min(256) {
        let e = 16 + i * 32;
        dpds.push(u32_at(&set, e)? as usize);
        set.get_mut(e + 16..e + 32)?.fill(0);
    }
    dpds.sort_unstable();
    dpds.dedup();
    for d in dpds {
        if u32_at(&set, d)? != 0x96 {
            return None;
        }
        let count = u32_at(&set, d + 4)? as usize;
        for k in 0..count.min(256) {
            let pd = d + u32_at(&set, d + 8 + 4 * k)? as usize;
            // The particle header's sound id and volume (`+0xFC`, `+0xFE`).
            set.get_mut(pd + 0xFC..pd + 0x100)?.fill(0);
        }
    }
    Some(set)
}

/// One PAX element: script id `id` plays the set's effect `effect` on `bone` (a bone index or one
/// of KH2's anchors: 0x4000 the object, 0x4003 / 0x4004 the weapon tips, ...), with `flags` (1 bind,
/// 2 bind at start, 4 position only, 0x20 gone when the motion changes, 0x80 gone at the loop's
/// end, 0x200 gone when the object leaves), `tag` (KH2's hit lookups: low half the kinds, high
/// half the groups) and `sound` (KH2's sound id, -1 none).
#[derive(Clone, Copy, Debug)]
pub struct Element {
    pub id: u16,
    pub effect: u16,
    pub bone: u16,
    pub flags: u32,
    pub tag: u64,
    pub sound: i32,
    pub scale: f32,
}

/// A PAX: "PAX_", no debug data, the elements, then the DPX.
pub fn pax(elements: &[Element], dpx: &[u8]) -> Vec<u8> {
    let at = 16 + elements.len() * 80;
    let dpx_at = (at + 15) & !15;
    let mut out = Vec::with_capacity(dpx_at + dpx.len());
    out.extend_from_slice(b"PAX_");
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(elements.len() as u32).to_le_bytes());
    out.extend_from_slice(&(dpx_at as u32).to_le_bytes());
    for e in elements {
        out.extend_from_slice(&e.effect.to_le_bytes());
        out.extend_from_slice(&e.id.to_le_bytes());
        out.extend_from_slice(&[0, 0]); // group, fade-out frames
        out.extend_from_slice(&e.bone.to_le_bytes());
        out.extend_from_slice(&e.tag.to_le_bytes());
        out.extend_from_slice(&e.flags.to_le_bytes());
        out.extend_from_slice(&0f32.to_le_bytes()); // start wait
        out.extend_from_slice(&e.sound.to_le_bytes());
        for v in [0.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, e.scale, e.scale, e.scale] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        // 16 bytes KH2's own elements leave zero.
        out.extend_from_slice(&[0u8; 16]);
    }
    out.resize(dpx_at, 0);
    out.extend_from_slice(dpx);
    out
}

/// A spell file: a BAR of the PAX (type 18) and the BDX (type 3) twice, as KH2's own (the second
/// marked a duplicate of the first, Checked (files) `FIRE_1.mag`).
pub fn mag(name: &[u8; 4], pax: &[u8], bdx: &[u8]) -> Vec<u8> {
    let head = 16 + 3 * 16;
    let pax_at = head;
    let bdx_at = (pax_at + pax.len() + 15) & !15;
    let mut out = Vec::with_capacity(bdx_at + bdx.len() + 16);
    out.extend_from_slice(b"BAR\x01");
    out.extend_from_slice(&3u32.to_le_bytes());
    out.extend_from_slice(&[0u8; 8]);
    for (kind, dup, at, len) in [(18u16, 0u16, pax_at, pax.len()), (3, 0, bdx_at, bdx.len()), (3, 1, bdx_at, bdx.len())] {
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&dup.to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(&(at as u32).to_le_bytes());
        out.extend_from_slice(&(len as u32).to_le_bytes());
    }
    out.extend_from_slice(pax);
    out.resize(bdx_at, 0);
    out.extend_from_slice(bdx);
    while out.len() % 16 != 0 {
        out.push(0);
    }
    out
}

/// The KH1 spells built into KH2's spell files: (KH1's file, KH2's file, spell, level 0..2).
/// KH2's `magic/<SPELL>_n.mag` is level n (its MAGC rows; the Drive Forms cast the same files,
/// the sister's `drive-model.md`); KH1's `xm_<spell>0n.mag` likewise (Checked (files): the scripts
/// differ only in the level's numbers, Blizzard's shard speed 20 / 30 / 40).
pub const SPELLS: [(&str, &str, Kind, usize); 21] = [
    ("xm_fire01.mag", "magic/FIRE_1.mag", Kind::Fire, 0),
    ("xm_fire02.mag", "magic/FIRE_2.mag", Kind::Fire, 1),
    ("xm_fire03.mag", "magic/FIRE_3.mag", Kind::Fire, 2),
    ("xm_blizzard01.mag", "magic/BLIZZARD_1.mag", Kind::Blizzard, 0),
    ("xm_blizzard02.mag", "magic/BLIZZARD_2.mag", Kind::Blizzard, 1),
    ("xm_blizzard03.mag", "magic/BLIZZARD_3.mag", Kind::Blizzard, 2),
    ("xm_thunder01.mag", "magic/THUNDER_1.mag", Kind::Thunder, 0),
    ("xm_thunder02.mag", "magic/THUNDER_2.mag", Kind::Thunder, 1),
    ("xm_thunder03.mag", "magic/THUNDER_3.mag", Kind::Thunder, 2),
    ("xm_cure01.mag", "magic/CURE_1.mag", Kind::Cure, 0),
    ("xm_cure02.mag", "magic/CURE_2.mag", Kind::Cure, 1),
    ("xm_cure03.mag", "magic/CURE_3.mag", Kind::Cure, 2),
    ("xm_gravity01.mag", "magic/GRAVITY_1.mag", Kind::Gravity, 0),
    ("xm_gravity02.mag", "magic/GRAVITY_2.mag", Kind::Gravity, 1),
    ("xm_gravity03.mag", "magic/GRAVITY_3.mag", Kind::Gravity, 2),
    ("xm_stop01.mag", "magic/STOP_1.mag", Kind::Stop, 0),
    ("xm_stop02.mag", "magic/STOP_2.mag", Kind::Stop, 1),
    ("xm_stop03.mag", "magic/STOP_3.mag", Kind::Stop, 2),
    // KH1 has one Aero file for the three levels (the level is the command's).
    ("xm_air01.mag", "magic/AERO_1.mag", Kind::Aero, 0),
    ("xm_air01.mag", "magic/AERO_2.mag", Kind::Aero, 1),
    ("xm_air01.mag", "magic/AERO_3.mag", Kind::Aero, 2),
];

/// The KH2 file a spell's file is built on: its own, or for a spell KH2 doesn't have, the KH2
/// spell cast on the same motions (Gravity: Blizzard's, KH1's 55 / 80), whose script, elements and
/// effects stay in the file unused (our script takes every sub).
pub fn kh2_base(kh2_name: &str) -> String {
    kh2_name.replace("GRAVITY", "BLIZZARD").replace("STOP", "THUNDER").replace("AERO", "THUNDER")
}

/// A spell KH2 doesn't have: our script for every sub, the Drive Forms too (owner, 2026-11-14).
pub fn kh1_only(kind: Kind) -> bool {
    matches!(kind, Kind::Gravity | Kind::Stop | Kind::Aero)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Fire,
    Blizzard,
    Thunder,
    Cure,
    Gravity,
    Stop,
    Aero,
}

/// KH2's own spell file (`kh2`, `magic/<SPELL>_n.mag`) with KH1's spell added (`kh1`, the matching
/// `xm_<spell>0n.mag`): KH2's script, elements and effects stay for the Drive Forms.
pub fn kh1_mag(kind: Kind, level: usize, kh2: &[u8], kh1: &[u8]) -> Option<Vec<u8>> {
    let bar = kh2_formats::bar::Bar::parse(kh2).ok()?;
    let pax_e = bar.entries.iter().find(|e| e.kind == 18)?;
    let bdx_e = bar.entries.iter().find(|e| e.kind == 3 && e.duplicate == 0)?;
    let name: [u8; 4] = {
        let mut n = [0u8; 4];
        let b = pax_e.name.as_bytes();
        n[..b.len().min(4)].copy_from_slice(&b[..b.len().min(4)]);
        n
    };
    let set = kh1_effect_set(kh1)?;
    let (pax, _) = merge_pax(pax_e.data, &set, |k| elements(kind, k))?;
    Some(mag(&name, &pax, &onto(kind, level, bdx_e.data)?))
}

/// The hit-effect group our attack rows get (`effect_on_hit`, patched by `kh2-hook` on base
/// Sora's rows of these spells' ATKPs), so KH2's lookups find our elements, not KH2's own.
pub const HIT_GROUP: u8 = 3;

/// Script ids of our elements (after KH2's own, which stay for the Drive Forms).
pub const CAST: i32 = 100;
pub const SHOT: i32 = 101;
pub const POINT: i32 = 104;
/// Blizzard's other shard (KH1's effect 5: the four cone shards off the level, Checked (files):
/// `xm_blizzard01.mag`'s spawner picks 1 for shards 0 and 3 and the straight one, else 5).
pub const SHOT_B: i32 = 105;
/// Thunder's area marker (KH1's effect 1) and its bolt on each enemy it strikes (effect 2).
pub const MARK: i32 = 102;
pub const BOLT: i32 = 103;
/// An effect held on the caster's hand at the release (Gravity).
pub const HAND: i32 = 108;

/// KH1's Fire (`xm_fire0n.mag`'s script; read with KH1-in-ReCom's disassembler, Checked (files)).
pub mod fire {
    /// The ball's speed (a frame), life (frames), turn toward the target (rad a frame) and
    /// radius (KH1's attack data `+0x5C`). The same in all three files.
    pub const SPEED: f32 = 40.0;
    pub const LIFE: f32 = 30.0;
    pub const TURN: f32 = 3.0 * std::f32::consts::PI / 180.0;
    pub const RADIUS: f32 = 50.0;
    /// The ball fades over this many frames once its life is under it.
    pub const FADE: f32 = 20.0;
    /// KH2's Fire attack (ATKP 502, its rows by the sub; Checked (files): `FIRE_1.mag`'s script).
    pub const ATKP: i32 = 502;
    /// KH1's pick for a ball cast with no target: enemies within this of the ball (`0x1402B0E90`).
    pub const PICK_RANGE: f32 = 500.0;
}

/// KH1's Blizzard (`xm_blizzard0n.mag`'s script, the same reading; KH1-in-ReCom's
/// `recom-m7-magic.md` §7.5).
pub mod blizzard {
    /// Shard speed by level (the spawner's argument, Checked (files): 20 / 30 / 40), life
    /// (frames), the height kept above the floor, radius by level (KH1's attack data `+0x5C`,
    /// btltbl, Checked (files) in KH1-in-ReCom).
    pub const SPEED: [f32; 3] = [20.0, 30.0, 40.0];
    pub const LIFE: f32 = 26.0;
    pub const HEIGHT: f32 = 50.0;
    pub const RADIUS: [f32; 3] = [70.0, 90.0, 120.0];
    /// KH2's Blizzard shard attack (ATKP 248, Checked (files): `BLIZZARD_1.mag`'s script).
    pub const ATKP: i32 = 248;
    /// The cone: (0.383, 0, 0.924) in Sora's frame (x right, y down, z forward), turned 60°
    /// about his facing for each of six shards, each starting its sideways part × 50 off the
    /// axis; then one straight ahead from the Keyblade's tip.
    pub const CONE: [f32; 3] = [0.383, 0.0, 0.924];
    pub const OFFSET: f32 = 50.0;

    /// (start offset, direction, effect id) of the seven shards in Sora's frame, in KH1's order.
    pub fn shards() -> [([f32; 3], [f32; 3], i32); 7] {
        let mut out = [([0.0; 3], [0.0, 0.0, 1.0], super::SHOT); 7];
        for (i, o) in out.iter_mut().take(6).enumerate() {
            let phi = (60.0 * i as f32).to_radians();
            let (x, y) = (CONE[0] * phi.cos() - CONE[1] * phi.sin(), CONE[0] * phi.sin() + CONE[1] * phi.cos());
            *o = ([x * OFFSET, y * OFFSET, 0.0], [x, y, CONE[2]], if i == 0 || i == 3 { super::SHOT } else { super::SHOT_B });
        }
        out
    }
}

/// KH1's Thunder (`xm_thunder0n.mag`, the same in all three files, Checked (files); KH1-in-ReCom's
/// `recom-m7-magic.md` §7.6).
pub mod thunder {
    /// The area: a cylinder of this radius and half height round the target (else Sora).
    pub const RADIUS: f32 = 600.0;
    pub const HALF_HEIGHT: f32 = 500.0;
    /// Frames between strikes; the tint's frames (down at the cast, back 40 after the release).
    pub const BETWEEN: f32 = 8.0;
    pub const TINT: f32 = 20.0;
    pub const TINT_HOLD: f32 = 40.0;
    /// KH1's tint: the colour × 0.5. KH2's light colour 0x80 is 1 (its default, `0x1403F6F80`).
    pub const TINT_COLOUR: i32 = 0x40;
    /// The marker 2 m ahead along the camera without a target.
    pub const AHEAD: f32 = 200.0;
    /// Enemies listed at most (KH1's 30 slots).
    pub const MAX: i32 = 30;
    /// KH2's Thunder burst (ATKP 250, its rows by the sub; Checked (files): `THUNDER_1.mag`).
    pub const ATKP: i32 = 250;
}

/// KH1's Gravity (`xm_gravity0n.mag`, KH1-in-ReCom §7.8, Checked (files + decomp) there; the
/// disassembly here agrees: `func_8cc(…, S, N)`, `func_3ea`'s parameters).
pub mod gravity {
    /// The sphere's scale at the impact and the frames of its fall, by level.
    pub const SCALE: [f32; 3] = [1.0, 1.5, 1.6];
    pub const FRAMES: [f32; 3] = [60.0, 70.0, 60.0];
    pub const START_SCALE: f32 = 0.01;
    /// Hit radius at scale 1 (`hp+0`), the fall (`hp+4`, the start 10 higher), the marker's frames
    /// (`hp+0x1C`), the fade's frames and growth (`hp+0x14`, `hp+0x18`).
    pub const RADIUS: f32 = 170.0;
    pub const FALL: f32 = 230.0;
    pub const ABOVE: f32 = 240.0;
    pub const MARK_FRAMES: f32 = 28.0;
    pub const FADE: f32 = 20.0;
    pub const FADE_GROWTH: f32 = 1.02;
    /// Without a target: the caster + 200 along his facing (offset taken at the cast).
    pub const AHEAD: f32 = 200.0;
    /// The floor-contact impact (effect 4) when the sphere ends within this of the floor, else 5.
    pub const NEAR_FLOOR: f32 = 100.0;
    /// Our ATKP (KH2's last is 2065, Checked (files)): `kh2-hook::magic_rows` adds its rows,
    /// a KH2 attack type of ours (7) for KH1's % of HP.
    pub const ATKP: i32 = 2066;
    /// KH1's mark on an enemy under the falling sphere (`+0x88 |= 4`, each fall frame): a KH2 stop
    /// kind of ours with no flags, one frame long (`kh2-hook` `magic_rows::SQUASH_ID` and its
    /// height step on KH2's stop tick).
    pub const SQUASH_KIND: i32 = 0x22;
}

/// KH1's Stop (`xm_stop0n.mag`; KH1-in-ReCom §7.9, Checked (files + decomp) there; our
/// disassembly agrees: `L00890`, `func_280`, `func_208`).
pub mod stop {
    /// The charge after 4 frames; the cast motion checked each frame for 24 more (cut: the charge
    /// fades, no spell); the release at 28.
    pub const CHARGE_AT: f32 = 4.0;
    pub const CHECK: f32 = 24.0;
    /// The cylinder round the target (else Sora): radius by level, ±300.
    pub const RADIUS: [f32; 3] = [250.0, 350.0, 500.0];
    pub const HALF_HEIGHT: f32 = 300.0;
    /// Frames between enemies.
    pub const BETWEEN: f32 = 4.0;
    /// Our ATKP (type 8: KH2's own stop on the struck object for KH1's time, `kh2-hook`).
    pub const ATKP: i32 = 2067;
    /// KH1's Stop look on a stopped enemy is killed when KH2's stop ends; checked from this many
    /// frames after the strike (KH2 applies the hit after the script's frame).
    pub const LOOK_FROM: f32 = 2.0;
}

/// KH1's Aero (`xm_air01.mag`, all levels; KH1-in-ReCom §7.10, Checked (files + decomp) there).
pub mod aero {
    /// The cast: up to 30 frames while the cast motion plays, at once when it changes.
    pub const CAST_FRAMES: f32 = 30.0;
    /// The lasting barrier look from 24 frames after the cast.
    pub const LOOK_AFTER: f32 = 24.0;
    /// The barrier: (18 + max MP bars) × 60 frames (KH1 `0x1402BEEE0` element 12).
    pub const BASE: f32 = 18.0;
    /// Aerora / Aeroga's wind: radius 80 (× Sora's size, 1), struck again every 30 frames.
    pub const WIND_RADIUS: f32 = 80.0;
    pub const WIND_EVERY: f32 = 30.0;
    /// The barrier strike on Sora (type 9 of ours: KH2's per-object timer on him; his damage
    /// taken halved while it runs; the hook leaves his max MP bars in the attack's `+0xB0`,
    /// read back by `trap_attack_get_reflect_count`) and the wind's attack.
    pub const ATKP: i32 = 2068;
    pub const WIND_ATKP: i32 = 2069;
}

/// KH1's Cure (`xm_cure0n.mag`, the same in all three files, Checked (files); KH1-in-ReCom §7.7).
pub mod cure {
    /// KH2's Cure heal on the caster (ATKP 708, kind 6: heals by Magic; rows by the sub; Checked
    /// (files): `CURE_1.mag`'s script, `00battle.bin`).
    pub const ATKP: i32 = 708;
    /// KH1 holds the heal effect at its bone 8's height over the target's feet, following him: KH2's
    /// body centre (`0x4001`) stands in for that bone (Assumption).
    pub const BONE: i32 = 0x4001;
}

/// Our elements, KH1's effect numbers moved up by `kh1` (`merge_pax`). KH1's sounds aren't in
/// KH2's banks yet: the matching KH2 spell's own stand in (Assumption on what they are: KH2's
/// PAX). Hit elements in `HIT_GROUP`: KH1's body hit (its set's `0x81` mask), its wall hit (`0x102`;
/// also on attack on attack). Unbound elements are placed by the script, as KH1's are.
pub fn elements(kind: Kind, kh1: u16) -> Vec<Element> {
    let g = 1u64 << (32 + HIT_GROUP as u64 - 1);
    let el = |id: i32, effect: u16, bone: u16, flags: u32, tag: u64, sound: i32| Element { id: id as u16, effect: kh1 + effect, bone, flags, tag, sound, scale: 1.0 };
    match kind {
        // KH1 Fire: 0 the cast on the Keyblade, 1 the ball, 4 the flash at the release point,
        // 3 / 2 the body / wall hits (Checked (files): the set's masks).
        Kind::Fire => vec![
            el(CAST, 0, 0x4003, 0x2A1, 0, 116),
            el(SHOT, 1, 0, 0, 0, -1),
            el(POINT, 4, 0, 0, 0, -1),
            el(103, 3, 0, 0, g | 1, 118),
            el(106, 2, 0, 0, g | 0x102, -1),
            el(107, 2, 0, 0, g | 0x10, -1),
        ],
        // KH1 Blizzard: 0 the cast, 1 / 5 the shards, 4 the release point, 2 / 3 body / wall.
        Kind::Blizzard => vec![
            el(CAST, 0, 0x4003, 0x2A1, 0, 120),
            el(SHOT, 1, 0, 0, 0, 121),
            el(SHOT_B, 5, 0, 0, 0, 121),
            el(POINT, 4, 0, 0, 0, -1),
            el(103, 2, 0, 0, g | 1, 122),
            el(106, 3, 0, 0, g | 0x102, 124),
            el(107, 3, 0, 0, g | 0x10, 125),
        ],
        // KH1 Thunder: 0 the cast, 3 the release point, 1 the area marker, 2 the bolt on each
        // enemy (played by the script, as KH1's; no hit element: KH2's spark doesn't play).
        Kind::Thunder => vec![
            el(CAST, 0, 0x4003, 0x2A1, 0, 126),
            el(POINT, 3, 0, 0, 0, -1),
            el(MARK, 1, 0, 0, 0, -1),
            el(BOLT, 2, 0, 0, 0, 127),
        ],
        // KH1 Cure: 0 the charge on the Keyblade, 1 the heal on Sora (placed by the script).
        Kind::Cure => vec![el(CAST, 0, 0x4003, 0x2A1, 0, 129), el(SHOT, 1, 0, 0, 0, 130)],
        // KH1 Gravity: 0 the charge on the Keyblade, 1 the hand effect (on the Keyblade's tip:
        // Assumption), 2 the marker, 3 the sphere, 4 / 5 the impact near / off the floor. No
        // hit element. Sounds: Blizzard's cast stands in (KH1's not in KH2's banks yet).
        Kind::Gravity => vec![
            el(CAST, 0, 0x4003, 0x2A1, 0, 120),
            el(HAND, 1, 0x4003, 0x2A1, 0, -1),
            el(MARK, 2, 0, 0, 0, -1),
            el(SHOT, 3, 0, 0, 0, -1),
            el(POINT, 4, 0, 0, 0, -1),
            el(SHOT_B, 5, 0, 0, 0, -1),
        ],
        // KH1 Stop: 0 the charge (KH1 on Sora's bone 12: the body centre here, Assumption), 1 at
        // the release (on the Keyblade's tip, Assumption), 2 the look on each stopped enemy
        // (bound to it). Thunder's cast sound stands in.
        Kind::Stop => vec![el(CAST, 0, 0x4001, 0x2A1, 0, 126), el(HAND, 1, 0x4003, 0x2A1, 0, -1), el(MARK, 2, 0, 0x2A1, 0, -1)],
        // KH1 Aero: 0 the charge, 1 the cast aura at Sora, 2 the lasting barrier and 12 the wind
        // (both on Sora's body centre: KH1 bone 32770, Assumption).
        Kind::Aero => vec![el(CAST, 0, 0x4003, 0x2A1, 0, 126), el(POINT, 1, 0, 0, 0, -1), el(SHOT, 2, 0x4001, 0x2A1, 0, -1), el(MARK, 12, 0x4001, 0x2A1, 0, -1)],
    }
}

// Work memory (`wp`): the caster's record (16 bytes, as the game hands it), the resources, the
// sub, the `magi` record, the cast effect, the cast motion; the release point, Sora's facing
// (sin, 0, cos, yaw), a zero point, Blizzard's first attack.
const W_CASTER: u16 = 0;
const W_RES: u16 = 16;
const W_SUB: u16 = 20;
const W_MAGI: u16 = 24;
const W_CAST_FX: u16 = 28;
const W_MOTION: u16 = 32;
const W_POINT: u16 = 48;
const W_FACING: u16 = 64;
const W_ZERO: u16 = 80;
const W_ATK0: u16 = 96;
const W_LIGHT: u16 = 100;
const W_RELEASED: u16 = 104;
/// Gravity: the target at the cast, the no-target offset.
const W_TARGET: u16 = 108;
const W_OFFSET: u16 = 112;
const WORK: u32 = 128;

// A shot's block (`memory_alloc`): a copy of the caster's record, its place, target, effect id,
// effect, attack and direction, a scratch vector.
const B_CASTER: u16 = 0;
const B_POS: u16 = 32;
const B_TARGET: u16 = 48;
const B_THREAD: u16 = 52;
const B_EFFECT: u16 = 56;
const B_FX: u16 = 64;
const B_ATK: u16 = 68;
const B_DIR: u16 = 80;
const B_SCRATCH: u16 = 96;
const BLOCK: i32 = 128;

/// KH2's weapon anchors (`0x1403B50F0`'s cases, Checked (decomp)): the tip, the middle. KH1 takes
/// the point along the Keyblade's bone (48) at a fraction: Blizzard 1 (the tip), Fire 0.5.
const TIP: i32 = 0x4003;
const MIDDLE: i32 = 0x4007;

/// A KH1 spell as a KH2 spell script of its own (the tests'; the build uses `onto`).
pub fn script(kind: Kind, level: usize) -> Vec<u8> {
    let mut a = Asm::new();
    code(kind, level, &mut a);
    a.finish("kh1spell", WORK, 0x400, 0x200, &[(21, "release"), (20, "cast")])
}

/// Kept for the Fire tests.
pub fn fire_script() -> Vec<u8> {
    script(Kind::Fire, 0)
}

/// KH1's spell onto KH2's own script (`kh2`, its BDX file): KH2's code stays where it was and
/// still runs for the Drive Forms (sub ≥ 10); base Sora's casts run ours. The trigger table points
/// at two stubs of ours that choose, and keep the choice in a work word past both scripts' own.
pub fn onto(kind: Kind, level: usize, kh2: &[u8]) -> Option<Vec<u8>> {
    use Mode::*;
    let p = kh2_bdx::Program::parse(kh2).ok()?;
    let (k20, k21) = (p.trigger(20)?, p.trigger(21)?);
    let ours = ((p.work_size.max(WORK) + 3) & !3) as u16;
    let mut a = Asm::new();
    a.label("t20").pop(Sp, 0).pop(Sp, 4).pop(Sp, 8).pop(Sp, 12);
    if kh1_only(kind) {
        a.push_i(1);
    } else {
        a.push_v(Sp, 4).push_i(10).bin("sub").un("sltz");
    }
    a.beqz("t20_kh2");
    a.push_i(1).pop(Wp, ours).push_v(Sp, 12).push_v(Sp, 8).push_v(Sp, 4).push_v(Sp, 0).b("cast");
    a.label("t20_kh2").push_i(0).pop(Wp, ours).push_v(Sp, 12).push_v(Sp, 8).push_v(Sp, 4).push_v(Sp, 0).b("kh2_20");
    a.label("t21").push_v(Wp, ours).bnez("release").b("kh2_21");
    code(kind, level, &mut a);
    // The table in KH2's own order (21 then 20, Checked (files): FIRE_n.mag, BLIZZARD_n.mag).
    let order: Vec<(i32, &str)> = p.triggers.iter().map(|(k, _)| (*k, if *k == 20 { "t20" } else { "t21" })).collect();
    a.finish_onto(kh2, ours as u32 + 16, 0x400, 0x200, &order, &[("kh2_20", k20), ("kh2_21", k21)]).ok()
}

fn code(kind: Kind, level: usize, a: &mut Asm) {
    if kind == Kind::Stop {
        return stop_code(level.min(2), a);
    }
    if kind == Kind::Aero {
        return aero_code(level.min(2), a);
    }
    cast_code(a, kind == Kind::Thunder, kind == Kind::Gravity);
    match kind {
        Kind::Stop | Kind::Aero => unreachable!(),
        Kind::Gravity => gravity_code(level.min(2), a),
        Kind::Fire => fire_code(a),
        Kind::Blizzard => blizzard_code(level.min(2), a),
        Kind::Thunder => thunder_code(a),
        Kind::Cure => cure_code(a),
    }
}

/// Trigger 20, the cast (every KH1 spell): the game pushes the caster, the resources, the sub and
/// `magi`. KH1's cast effect on the Keyblade, held while the cast motion plays (KH2's own spells
/// do: a cast cut short does nothing more).
fn cast_code(a: &mut Asm, tint: bool, marker: bool) {
    use Mode::*;
    a.label("cast").pop(Sp, 0).pop(Sp, 4).pop(Sp, 8).pop(Sp, 12);
    a.push_v(Sp, 12).memcpy(Wp, 16, W_CASTER);
    a.push_v(Sp, 8).pop(Wp, W_RES);
    a.push_v(Sp, 4).pop(Wp, W_SUB);
    a.push_v(Sp, 0).pop(Wp, W_MAGI);
    a.push_v(Wp, W_RES).push_i(CAST).push_a(Wp, W_CASTER).push_i(1).push_i(0).sys("trap_pax_start_bind").pop(Wp, W_CAST_FX);
    if tint {
        // KH1 Thunder's tint (`sys 52`, `0x1402BC190`: a scene colour taken to 0.5 over 20 frames)
        // as one of KH2's lights (`trap_light_create(1)` copies the scene's, `0x1403F6F80`;
        // `trap_light_set_color(light, r, g, b, frames)` blends to it, `0x1404309C0` →
        // `0x1403F76E0`; Checked (decomp)). Assumption: KH2's light colours what KH1's did.
        a.push_i(0).pop(Wp, W_RELEASED);
        a.push_i(1).sys("trap_light_create").pop(Wp, W_LIGHT);
        let c = thunder::TINT_COLOUR;
        a.push_v(Wp, W_LIGHT).push_i(c).push_i(c).push_i(c).push_f(thunder::TINT).sys("trap_light_set_color");
    }
    if marker {
        gravity_marker_start(a);
    }
    a.push_a(Wp, W_CASTER).push_dpop(4).sys("trap_sysobj_motion_id").pop(Wp, W_MOTION);
    a.label("cast_wait").push_a(Wp, W_CASTER).push_dpop(4).sys("trap_sysobj_motion_id").push_v(Wp, W_MOTION).bin("sub").un("seqz").beqz("cast_end").halt().b("cast_wait");
    a.label("cast_end");
    if tint {
        // A cast cut off before its release: the tint back now (KH1 `0xC48`, over 20 frames).
        a.push_v(Wp, W_RELEASED).bnez("cast_ret");
        a.push_v(Wp, W_LIGHT).push_f(thunder::TINT).sys("trap_light_fadeout");
        a.label("cast_ret");
    }
    a.ret();
}

/// The release's common part: the point on the Keyblade (`bone`) into `W_POINT`, Sora's facing
/// into `W_FACING`, KH1's flash (`POINT`) there turned by `turn` from his facing (KH1: Fire 0,
/// Blizzard π).
fn release_point(a: &mut Asm, bone: i32, turn: Option<f32>) {
    use Mode::*;
    a.label("release");
    for (o, v) in [(0, 0.0f32), (4, 0.0), (8, 0.0), (12, 1.0)] {
        a.push_f(v).pop(Wp, W_ZERO + o);
    }
    // (caster's record, bone or anchor, a local point): the point in the world, Checked (decomp:
    // trap table 1 entry 83, `0x14042F6F0` → `0x1403B50F0`).
    a.push_a(Wp, W_CASTER).push_i(bone).push_a(Wp, W_ZERO).sys("trap_obj_apply_bone_matrix").memcpy(Wp, 16, W_POINT);
    a.push_a(Wp, W_CASTER).sys("trap_obj_dir").memcpy(Wp, 16, W_FACING);
    a.push_v(Wp, W_RES).push_i(POINT).push_i(1).push_i(0).sys("trap_pax_start").pop(Sp, 4);
    a.push_v(Sp, 4).push_a(Wp, W_POINT).sys("trap_effect_set_pos");
    match turn {
        None => {}
        Some(t) if t == 0.0 => {
            a.push_v(Sp, 4).push_a(Wp, W_FACING).sys("trap_effect_set_dir");
        }
        Some(t) => {
            a.push_v(Sp, 4).push_a(Wp, W_FACING).push_f(t).sys("trap_vector_roty").sys("trap_effect_set_dir");
        }
    }
}

/// A new shot's block into `sp+0`, with the caster's record.
fn new_block(a: &mut Asm) {
    new_block_of(a, BLOCK);
}

fn new_block_of(a: &mut Asm, size: i32) {
    use Mode::*;
    a.push_i(size).sys("trap_memory_alloc").pop(Sp, 0);
    a.push_a(Wp, W_CASTER).memcpy(SpD, 16, B_CASTER);
}

fn fire_code(a: &mut Asm) {
    use Mode::*;
    // Trigger 21, the cast event (KH1's release): from the Keyblade's middle, along his facing
    // (KH1 faces the target at the cast); the flash there; the ball's block and thread.
    release_point(a, MIDDLE, Some(0.0));
    new_block_of(a, T_BLOCK);
    a.push_v(Sp, 0).push_i(B_TARGET as i32).bin("add").push_v(Sp, 0).sys("trap_btlobj_target").sys("trap_target_dup").store();
    a.push_a(Wp, W_POINT).memcpy(SpD, 16, B_POS);
    a.push_a(Wp, W_FACING).memcpy(SpD, 16, B_DIR);
    // KH1 re-picks only for a ball cast with no target (`L000fc`: the thread's target argument
    // 0); a ball whose target dies flies straight on (`L0013e`).
    a.push_v(SpD, B_TARGET).sys("trap_target_is_exist").un("seqz").pop(SpD, F_NO_TARGET);
    a.push_v(Sp, 0).push_i(B_THREAD as i32).bin("add").push_pc("ball").push_v(Sp, 0).sys("trap_magic_start_thread").store();
    a.ret();

    // The ball's thread, in KH1's order (`xm_fire01.mag` `0x7A`): while it has life, turn toward
    // a live target (at most 3° a frame), start the fade once under 20 frames, move, place the
    // effect and the attack, lose a frame, wait a frame, end on a hit.
    a.label("ball").pop(Sp, 0);
    a.push_v(Wp, W_RES).push_i(SHOT).push_i(1).push_i(0).sys("trap_pax_start").pop(SpD, B_FX);
    a.push_a(Wp, W_CASTER).push_i(fire::ATKP).push_i(-1).push_v(Wp, W_SUB).sys("trap_attack_new").pop(SpD, B_ATK);
    a.push_v(SpD, B_ATK).push_v(Wp, W_RES).sys("trap_attack_set_pax");
    // (attack, radius, height: 0 = the radius; 3 arguments, Checked (exe: trap table 2 entry 11,
    // `0x1404322F0`), as KH2's own Blizzard calls it).
    a.push_v(SpD, B_ATK).push_f(fire::RADIUS).push_f(0.0).sys("trap_attack_set_radius");
    // sp+4 the life left, sp+8 fading.
    a.push_f(fire::LIFE).pop(Sp, 4).push_i(0).pop(Sp, 8);
    a.label("fly");
    a.push_v(Sp, 4).un("sgtz.s").beqz("ball_end");
    a.push_v(SpD, F_NO_TARGET).bnez("pick");
    a.push_v(SpD, B_TARGET).sys("trap_target_is_exist").beqz("fade");
    a.push_v(SpD, B_TARGET).sys("trap_target_pos").memcpy(Sp, 16, 16);
    a.label("home");
    a.push_a(Sp, 16).push_a(SpD, B_POS).sys("trap_vector_sub").memcpy(Sp, 16, 32);
    a.push_a(SpD, B_DIR).push_a(Sp, 32).push_f(fire::TURN).sys("trap_vector_homing");
    a.b("fade");
    // KH1's pick for a ball cast with no target (sys 78 → `0x1402B0E90`), this frame only: every
    // enemy within 500 of the ball (KH2's target searcher, nearest first: the first out of reach
    // ends the list), score d × (1 + 2 × (1 − cos)) with cos between the ball's way and the way to
    // it (0 for a zero vector); the first taken, later ones only with a smaller score.
    a.label("pick");
    for k in 0..4u16 {
        a.push_i(0).pop(SpD, T_FAKE + 4 * k);
    }
    a.push_a(SpD, T_SEARCH).push_i(T_HISTORY).sys("trap_target_searcher_init");
    a.push_f(1.0e9).pop(SpD, T_SEARCH + 0x14);
    a.push_i(0).pop(SpD, T_TRIES).push_f(-1.0).pop(SpD, F_BEST);
    a.label("pick_next");
    a.push_v(SpD, T_TRIES).push_i(T_HISTORY).bin("sub").un("sltz").beqz("picked");
    a.push_v(SpD, T_TRIES).push_i(1).bin("add").pop(SpD, T_TRIES);
    a.push_a(SpD, T_SEARCH).push_a(SpD, T_REC).push_a(SpD, B_POS).push_a(SpD, B_POS).push_i(1).push_i(0).push_i(1).sys("trap_target_seracher_search");
    a.push_a(SpD, T_REC).sys("trap_target_is_exist").beqz("picked");
    a.push_a(SpD, T_REC).sys("trap_target_pos").memcpy(SpD, 16, B_SCRATCH);
    a.push_a(SpD, B_SCRATCH).push_a(SpD, B_POS).sys("trap_vector_sub").memcpy(Sp, 16, 32);
    a.push_a(Sp, 32).sys("trap_vector_len").pop(Sp, 48);
    a.push_v(Sp, 48).push_f(fire::PICK_RANGE).bin("sub.s").un("sltz.s").beqz("picked");
    // cos: (dir · way) / d, 0 when d is 0, kept to [−1, 1].
    a.push_f(0.0).pop(Sp, 52);
    a.push_v(Sp, 48).un("sgtz.s").beqz("pick_cos");
    a.push_v(SpD, B_DIR).push_v(Sp, 32).bin("mul.s").push_v(SpD, B_DIR + 4).push_v(Sp, 36).bin("mul.s").bin("add.s").push_v(SpD, B_DIR + 8).push_v(Sp, 40).bin("mul.s").bin("add.s");
    a.push_v(Sp, 48).bin("div.s").pop(Sp, 52);
    a.push_v(Sp, 52).push_f(1.0).bin("sub.s").un("sgtz.s").beqz("pick_lo").push_f(1.0).pop(Sp, 52);
    a.label("pick_lo").push_v(Sp, 52).push_f(-1.0).bin("sub.s").un("sltz.s").beqz("pick_cos").push_f(-1.0).pop(Sp, 52);
    a.label("pick_cos");
    a.push_v(Sp, 48).push_f(3.0).push_v(Sp, 52).push_f(2.0).bin("mul.s").bin("sub.s").bin("mul.s").pop(Sp, 56);
    a.push_v(SpD, F_BEST).un("sltz.s").bnez("pick_take");
    a.push_v(Sp, 56).push_v(SpD, F_BEST).bin("sub.s").un("sltz.s").beqz("pick_next");
    a.label("pick_take").push_v(Sp, 56).pop(SpD, F_BEST);
    a.push_a(SpD, B_SCRATCH).memcpy(SpD, 16, T_ORIGIN);
    a.b("pick_next");
    a.label("picked");
    a.push_v(SpD, F_BEST).un("sltz.s").bnez("fade");
    a.push_a(SpD, T_ORIGIN).memcpy(Sp, 16, 16).b("home");
    a.label("fade").push_v(Sp, 8).bnez("move");
    a.push_v(Sp, 4).push_f(fire::FADE).bin("sub.s").un("sltz.s").beqz("move");
    a.push_v(SpD, B_FX).push_f(fire::FADE).push_i(1).sys("trap_effect_fadeout").push_i(1).pop(Sp, 8);
    a.label("move").push_a(SpD, B_POS).push_a(SpD, B_DIR).push_f(fire::SPEED).sys("trap_vector_addf");
    a.push_v(SpD, B_FX).push_a(SpD, B_DIR).sys("trap_effect_set_dir");
    a.push_v(SpD, B_FX).push_a(SpD, B_POS).sys("trap_effect_set_pos");
    a.push_v(SpD, B_ATK).push_a(SpD, B_POS).sys("trap_attack_set_pos");
    a.push_v(Sp, 4).sys("trap_frametime").bin("sub.s").pop(Sp, 4);
    a.halt();
    a.push_v(SpD, B_ATK).sys("trap_attack_is_hit_bg").bnez("ball_end");
    a.push_v(SpD, B_ATK).sys("trap_attack_is_reflect").bnez("ball_end");
    a.push_v(SpD, B_ATK).sys("trap_attack_is_hit").bnez("ball_end");
    a.b("fly");
    // Gone (KH1 `sys 97`: the effect killed, Checked (decomp `0x1402BB900` → `0x1401EC470`)).
    a.label("ball_end").push_v(SpD, B_FX).sys("trap_effect_kill");
    a.push_v(SpD, B_ATK).sys("trap_attack_free");
    a.push_v(SpD, B_TARGET).sys("trap_target_free");
    a.push_v(Sp, 0).sys("trap_memory_free");
    a.ret();
}

fn blizzard_code(level: usize, a: &mut Asm) {
    use Mode::*;
    // Trigger 21 (KH1's release, frame 28): from the Keyblade's tip, the flash there turned away
    // from his facing; one attack for the cast, copied for each shard (`trap_attack_dup`: the same
    // id and group, so an enemy one shard hit is not hit by the others: KH1's shared hit bit,
    // Checked (decomp: trap table 2 entry 31, `0x1404324D0` → `0x1404372E0`)); seven shards.
    release_point(a, TIP, Some(std::f32::consts::PI));
    a.push_a(Wp, W_CASTER).push_i(blizzard::ATKP).push_i(-1).push_v(Wp, W_SUB).sys("trap_attack_new").pop(Wp, W_ATK0);
    for (i, (off, dir, fx)) in blizzard::shards().into_iter().enumerate() {
        new_block(a);
        // Sora's frame to the world: turned by his yaw (`trap_vector_roty`, about y, Checked
        // (decomp `0x14041CFE0` → `0x1403B7680`)), the start from the tip.
        for (k, v) in off.iter().enumerate() {
            a.push_f(*v).pop(SpD, B_POS + 4 * k as u16);
        }
        a.push_f(1.0).pop(SpD, B_POS + 12);
        a.push_a(SpD, B_POS).push_v(Wp, W_FACING + 12).sys("trap_vector_roty").memcpy(SpD, 16, B_POS);
        a.push_a(SpD, B_POS).push_a(Wp, W_POINT).sys("trap_vector_add").memcpy(SpD, 16, B_POS);
        for (k, v) in dir.iter().enumerate() {
            a.push_f(*v).pop(SpD, B_DIR + 4 * k as u16);
        }
        a.push_f(0.0).pop(SpD, B_DIR + 12);
        a.push_a(SpD, B_DIR).push_v(Wp, W_FACING + 12).sys("trap_vector_roty").memcpy(SpD, 16, B_DIR);
        if i == 0 {
            a.push_v(Wp, W_ATK0).pop(SpD, B_ATK);
        } else {
            a.push_v(Wp, W_ATK0).push_i(blizzard::ATKP).sys("trap_attack_dup").pop(SpD, B_ATK);
        }
        a.push_i(fx).pop(SpD, B_EFFECT);
        a.push_v(Sp, 0).push_i(B_THREAD as i32).bin("add").push_pc("shard").push_v(Sp, 0).sys("trap_magic_start_thread").store();
    }
    a.ret();

    // A shard's thread, in KH1's order (`xm_blizzard01.mag` `0x238`): while its effect lives and it
    // has life: x and z by its direction × speed; y glides (a ray straight down from it by 50 + its
    // own vertical step: a floor → 50 above it, none → its own step); place the effect and the
    // attack; wait a frame; end on a hit; lose a frame.
    let (speed, radius) = (blizzard::SPEED[level], blizzard::RADIUS[level]);
    a.label("shard").pop(Sp, 0);
    a.push_v(Wp, W_RES).push_v(SpD, B_EFFECT).push_i(1).push_i(0).sys("trap_pax_start").pop(SpD, B_FX);
    a.push_v(SpD, B_FX).push_a(SpD, B_DIR).sys("trap_effect_set_dir");
    a.push_v(SpD, B_ATK).push_v(Wp, W_RES).sys("trap_attack_set_pax");
    a.push_v(SpD, B_ATK).push_f(radius).push_f(0.0).sys("trap_attack_set_radius");
    // sp+4 the life left, sp+8 the step (speed × frame time), sp+12 the vertical step.
    a.push_f(blizzard::LIFE).pop(Sp, 4);
    a.label("glide");
    a.push_v(SpD, B_FX).sys("trap_effect_is_alive").beqz("shard_end");
    a.push_v(Sp, 4).un("sgtz.s").beqz("shard_end");
    a.push_f(speed).sys("trap_frametime").bin("mul.s").pop(Sp, 8);
    for k in [0u16, 8] {
        a.push_v(SpD, B_POS + k).push_v(SpD, B_DIR + k).push_v(Sp, 8).bin("mul.s").bin("add.s").pop(SpD, B_POS + k);
    }
    a.push_v(SpD, B_DIR + 4).push_v(Sp, 8).bin("mul.s").pop(Sp, 12);
    // (from, out, depth): a ray from `from` down by `depth`, the hit (else `from`) into `out`; 1 on
    // a hit. Checked (decomp: trap table 1 entry 141, `0x14042E8B0` → `0x1403F3DD0`).
    a.push_a(SpD, B_POS).push_a(SpD, B_SCRATCH).push_v(Sp, 12).push_f(blizzard::HEIGHT).bin("add.s").sys("trap_bg_ground_pos").beqz("no_floor");
    a.push_v(SpD, B_SCRATCH + 4).push_f(blizzard::HEIGHT).bin("sub.s").pop(SpD, B_POS + 4).b("placed");
    a.label("no_floor").push_v(SpD, B_POS + 4).push_v(Sp, 12).bin("add.s").pop(SpD, B_POS + 4);
    a.label("placed");
    a.push_v(SpD, B_FX).push_a(SpD, B_POS).sys("trap_effect_set_pos");
    a.push_v(SpD, B_ATK).push_a(SpD, B_POS).sys("trap_attack_set_pos");
    a.halt();
    a.push_v(SpD, B_ATK).sys("trap_attack_is_hit_bg").bnez("shard_end");
    a.push_v(SpD, B_ATK).sys("trap_attack_is_reflect").bnez("shard_end");
    a.push_v(SpD, B_ATK).sys("trap_attack_is_hit").bnez("shard_end");
    a.push_v(Sp, 4).sys("trap_frametime").bin("sub.s").pop(Sp, 4);
    a.b("glide");
    // Gone (KH1 `sys1 3`: the effect's end flag set, its loop let finish: Assumption, from
    // `0x140394DE0` setting the effect's first flag byte).
    a.label("shard_end").push_v(SpD, B_FX).sys("trap_effect_loop_end");
    a.push_v(SpD, B_ATK).sys("trap_attack_free");
    a.push_v(Sp, 0).sys("trap_memory_free");
    a.ret();
}

// Thunder's block: the shot block's fields (B_POS the area's centre, B_DIR the marker's offset,
// B_FX the marker), then the list origin, a target record, a stand-in object record, counters,
// KH2's target searcher (`0x1C + 8 × 64`) and the list (30 target records).
const T_ORIGIN: u16 = 112;
const T_REC: u16 = 128;
const T_FAKE: u16 = 144;
const T_N: u16 = 160;
const T_I: u16 = 164;
const T_J: u16 = 168;
const T_TRIES: u16 = 172;
const T_SEARCH: u16 = 176;
const T_HISTORY: i32 = 64;
const T_LIST: u16 = 720;
const T_BLOCK: i32 = 1216;
/// Fire's ball: cast with no target; the best score this frame (−1 none).
const F_NO_TARGET: u16 = T_I;
const F_BEST: u16 = T_J;

fn thunder_code(a: &mut Asm) {
    use Mode::*;
    // Trigger 21 (KH1's release, frame 30): the flash (effect 3) at the Keyblade's tip; the strike
    // thread; the tint back 40 frames on (its own thread).
    release_point(a, TIP, None);
    a.push_i(1).pop(Wp, W_RELEASED);
    new_block_of(a, T_BLOCK);
    a.push_v(Sp, 0).push_i(B_TARGET as i32).bin("add").push_v(Sp, 0).sys("trap_btlobj_target").sys("trap_target_dup").store();
    for k in 0..4u16 {
        a.push_i(0).pop(SpD, T_FAKE + 4 * k);
        a.push_f(0.0).pop(SpD, B_DIR + 4 * k);
    }
    // Without a target the marker sits 2 m ahead along the camera's level forward (KH1 `0x552`:
    // look point − eye, y 0, normalised, × 200; `trap_camera_eye` / `_at` are KH1's sys1 5 / 6).
    a.push_v(SpD, B_TARGET).sys("trap_target_is_exist").bnez("th_aimed");
    a.sys("trap_camera_at").memcpy(SpD, 16, B_DIR);
    a.push_a(SpD, B_DIR).sys("trap_camera_eye").sys("trap_vector_sub").memcpy(SpD, 16, B_DIR);
    a.push_f(0.0).pop(SpD, B_DIR + 4).push_f(0.0).pop(SpD, B_DIR + 12);
    a.push_a(SpD, B_DIR).sys("trap_vector_normalize").push_f(thunder::AHEAD).sys("trap_vector_mul").memcpy(SpD, 16, B_DIR);
    a.label("th_aimed");
    a.push_v(Sp, 0).push_i(B_THREAD as i32).bin("add").push_pc("strikes").push_v(Sp, 0).sys("trap_magic_start_thread").store();
    a.push_pc("tint").push_i(0).sys("trap_magic_start_thread").drop();
    a.ret();

    a.label("tint").pop(Sp, 0);
    a.push_f(thunder::TINT_HOLD).pop(Sp, 4);
    a.label("tint_wait").halt().push_v(Sp, 4).sys("trap_frametime").bin("sub.s").pop(Sp, 4);
    a.push_v(Sp, 4).un("sgtz.s").bnez("tint_wait");
    a.push_v(Wp, W_LIGHT).push_f(thunder::TINT).sys("trap_light_fadeout");
    a.ret();

    // The strike thread (KH1 `0x17E`): the centre on the target if it lives, else Sora; the marker
    // on the floor under it (a ray of 500) plus the offset; every enemy listed nearest first from
    // the target (else Sora); down the list, each one in the cylinder struck (8 frames apart).
    a.label("strikes").pop(Sp, 0);
    a.push_v(SpD, B_TARGET).sys("trap_target_is_exist").beqz("th_alone");
    a.push_v(SpD, B_TARGET).sys("trap_target_pos").memcpy(SpD, 16, B_POS).b("th_centred");
    a.label("th_alone").push_v(Sp, 0).sys("trap_obj_pos").memcpy(SpD, 16, B_POS);
    a.label("th_centred").push_a(SpD, B_POS).memcpy(SpD, 16, T_ORIGIN);
    a.push_a(SpD, B_POS).push_a(SpD, B_SCRATCH).push_f(thunder::HALF_HEIGHT).sys("trap_bg_ground_pos").bnez("th_floor");
    a.push_v(SpD, B_SCRATCH + 4).push_f(thunder::HALF_HEIGHT).bin("add.s").pop(SpD, B_SCRATCH + 4);
    a.label("th_floor").push_a(SpD, B_SCRATCH).push_a(SpD, B_DIR).sys("trap_vector_add").memcpy(SpD, 16, B_SCRATCH);
    a.push_v(Wp, W_RES).push_i(MARK).push_i(1).push_i(0).sys("trap_pax_start").pop(SpD, B_FX);
    a.push_v(SpD, B_FX).push_a(SpD, B_SCRATCH).sys("trap_effect_set_pos");
    a.push_a(Wp, W_CASTER).push_i(thunder::ATKP).push_i(-1).push_v(Wp, W_SUB).sys("trap_attack_new").pop(SpD, B_ATK);
    a.push_v(SpD, B_ATK).push_v(Wp, W_RES).sys("trap_attack_set_pax");
    a.push_v(SpD, B_ATK).push_a(SpD, B_POS).sys("trap_attack_set_pos");
    // The list: KH2's target searcher (table 6: `trap_target_searcher_init(searcher, history)`,
    // `0x14053DCE0` → `0x14053FB60`; `trap_target_seracher_search(searcher, out, from, dir,
    // nearest, level, skip the history)`, `0x14053DD20` → `0x14053FB80`: every battle target's
    // parts, by distance from `from` within the searcher's range `+0x14`, the nearest not in its
    // history, then added to it; Checked (decomp)). Its range opened (KH1 lists every enemy);
    // one entry per enemy (a part of one already listed is passed).
    a.push_a(SpD, T_SEARCH).push_i(T_HISTORY).sys("trap_target_searcher_init");
    a.push_f(1.0e9).pop(SpD, T_SEARCH + 0x14);
    a.push_i(0).pop(SpD, T_N).push_i(0).pop(SpD, T_TRIES);
    a.label("th_collect");
    a.push_v(SpD, T_N).push_i(thunder::MAX).bin("sub").un("sltz").beqz("th_listed");
    a.push_v(SpD, T_TRIES).push_i(T_HISTORY).bin("sub").un("sltz").beqz("th_listed");
    a.push_v(SpD, T_TRIES).push_i(1).bin("add").pop(SpD, T_TRIES);
    a.push_a(SpD, T_SEARCH).push_a(SpD, T_REC).push_a(SpD, T_ORIGIN).push_a(SpD, T_ORIGIN).push_i(1).push_i(0).push_i(1).sys("trap_target_seracher_search");
    a.push_a(SpD, T_REC).sys("trap_target_is_exist").beqz("th_listed");
    a.push_i(0).pop(SpD, T_J);
    a.label("th_dedupe");
    a.push_v(SpD, T_J).push_v(SpD, T_N).bin("sub").un("sltz").beqz("th_add");
    a.push_v(Sp, 0).push_i(T_LIST as i32).bin("add").push_v(SpD, T_J).push_i(16).bin("mul").bin("add").push_dpop(0);
    a.push_v(SpD, T_REC).bin("sub").un("seqz").bnez("th_collect");
    a.push_v(SpD, T_J).push_i(1).bin("add").pop(SpD, T_J).b("th_dedupe");
    a.label("th_add");
    for k in 0..4u16 {
        a.push_v(Sp, 0).push_i(T_LIST as i32 + 4 * k as i32).bin("add").push_v(SpD, T_N).push_i(16).bin("mul").bin("add");
        a.push_v(SpD, T_REC + 4 * k).store();
    }
    a.push_v(SpD, T_N).push_i(1).bin("add").pop(SpD, T_N).b("th_collect");
    a.label("th_listed");
    a.push_i(0).pop(SpD, T_I);
    a.label("th_next");
    a.push_v(SpD, T_I).push_v(SpD, T_N).bin("sub").un("sltz").beqz("th_done");
    a.push_v(Sp, 0).push_i(T_LIST as i32).bin("add").push_v(SpD, T_I).push_i(16).bin("mul").bin("add").pop(Sp, 4);
    for k in 0..4u16 {
        a.push_v(Sp, 4).push_dpop(4 * k).pop(SpD, T_REC + 4 * k);
    }
    a.push_v(SpD, T_I).push_i(1).bin("add").pop(SpD, T_I);
    a.push_a(SpD, T_REC).sys("trap_target_is_exist").beqz("th_next");
    // In the cylinder (KH1 `sys 90`, its bodies against it: here the struck part's centre,
    // Assumption).
    a.push_a(SpD, T_REC).sys("trap_target_pos").memcpy(SpD, 16, B_SCRATCH);
    for (k, at) in [(0u16, 8u16), (8, 12), (4, 16)] {
        a.push_v(SpD, B_SCRATCH + k).push_v(SpD, B_POS + k).bin("sub.s").pop(Sp, at);
    }
    a.push_v(Sp, 8).push_v(Sp, 8).bin("mul.s").push_v(Sp, 12).push_v(Sp, 12).bin("mul.s").bin("add.s");
    a.push_f(thunder::RADIUS * thunder::RADIUS).bin("sub.s").un("sgtz.s").bnez("th_next");
    a.push_v(Sp, 16).un("abs.s").push_f(thunder::HALF_HEIGHT).bin("sub.s").un("sgtz.s").bnez("th_next");
    // Struck straight on (KH1 `sys 160`; KH2's `trap_attack_strike(attack, object record, 0, 0)`,
    // table 2 entry 17, `0x1404328E0`: reads the record's object at `+4`, as Cure's call; Checked
    // (decomp)), KH1's bolt where it struck.
    a.push_v(SpD, T_REC).pop(SpD, T_FAKE + 4);
    a.push_v(SpD, B_ATK).push_a(SpD, T_FAKE).push_i(0).push_i(0).sys("trap_attack_strike");
    a.push_v(Wp, W_RES).push_i(BOLT).push_i(1).push_i(0).sys("trap_pax_start").push_a(SpD, B_SCRATCH).sys("trap_effect_set_pos");
    a.push_f(thunder::BETWEEN).pop(Sp, 20);
    a.label("th_wait").halt().push_v(Sp, 20).sys("trap_frametime").bin("sub.s").pop(Sp, 20);
    a.push_v(Sp, 20).un("sgtz.s").bnez("th_wait");
    a.b("th_next");
    a.label("th_done");
    a.push_v(SpD, B_ATK).sys("trap_attack_free");
    a.push_v(SpD, B_TARGET).sys("trap_target_free");
    a.push_v(Sp, 0).sys("trap_memory_free");
    a.ret();
}

/// Gravity's cast (KH1 `L008cc`): the target (or none) and the no-target offset (the caster's
/// facing × 200, taken now), and the marker thread.
fn gravity_marker_start(a: &mut Asm) {
    use Mode::*;
    a.push_a(Wp, W_CASTER).sys("trap_btlobj_target").sys("trap_target_dup").pop(Wp, W_TARGET);
    a.push_a(Wp, W_CASTER).sys("trap_obj_dir").push_f(gravity::AHEAD).sys("trap_vector_mul").memcpy(Wp, 16, W_OFFSET);
    a.push_pc("gr_marker").push_i(0).sys("trap_magic_start_thread").drop();
}

/// The point now: the target's (if it lives), else the caster's place + the cast's offset, into
/// `sp + to` of the current context's block (`SpD`) or the work (`Wp`).
fn gravity_point(a: &mut Asm, mode: Mode, to: u16, l: &str) {
    use Mode::*;
    a.push_v(Wp, W_TARGET).sys("trap_target_is_exist").beqz(&format!("{l}_alone"));
    a.push_v(Wp, W_TARGET).sys("trap_target_pos").memcpy(mode, 16, to).b(&format!("{l}_done"));
    a.label(&format!("{l}_alone")).push_a(Wp, W_CASTER).sys("trap_obj_pos").push_a(Wp, W_OFFSET).sys("trap_vector_add").memcpy(mode, 16, to);
    a.label(&format!("{l}_done"));
}

fn gravity_code(level: usize, a: &mut Asm) {
    use Mode::*;
    let (scale, frames) = (gravity::SCALE[level], gravity::FRAMES[level]);
    // The marker thread (KH1 `0x25A`, 28 frames from the cast): KH1's marker 240 above the point
    // each frame. Its block: the point (B_POS), the effect (B_FX), the frames left (B_SCRATCH).
    a.label("gr_marker").pop(Sp, 4);
    new_block(a);
    a.push_v(Wp, W_RES).push_i(MARK).push_i(1).push_i(0).sys("trap_pax_start").pop(SpD, B_FX);
    a.push_f(gravity::MARK_FRAMES).pop(SpD, B_SCRATCH);
    a.label("gr_mark_frame");
    gravity_point(a, SpD, B_POS, "gr_mp");
    a.push_v(SpD, B_POS + 4).push_f(gravity::ABOVE).bin("sub.s").pop(SpD, B_POS + 4);
    a.push_v(SpD, B_FX).push_a(SpD, B_POS).sys("trap_effect_set_pos");
    a.push_v(SpD, B_SCRATCH).sys("trap_frametime").bin("sub.s").pop(SpD, B_SCRATCH);
    a.push_v(SpD, B_SCRATCH).un("sgtz.s").beqz("gr_mark_end");
    a.halt().b("gr_mark_frame");
    a.label("gr_mark_end").push_v(Sp, 0).sys("trap_memory_free");
    a.ret();

    // Trigger 21 (KH1's release, 28): the hand effect; the point again; the sphere's block and
    // its thread.
    a.label("release");
    a.push_v(Wp, W_RES).push_i(HAND).push_a(Wp, W_CASTER).push_i(1).push_i(0).sys("trap_pax_start_bind").drop();
    new_block_of(a, T_BLOCK);
    gravity_point(a, SpD, B_POS, "gr_rp");
    a.push_v(SpD, B_POS + 4).push_f(gravity::ABOVE).bin("sub.s").pop(SpD, B_POS + 4);
    a.push_v(Sp, 0).push_i(B_THREAD as i32).bin("add").push_pc("gr_fall").push_v(Sp, 0).sys("trap_magic_start_thread").store();
    a.ret();

    // The fall (KH1 `0x44..0x258`): the sphere 230 down over N frames, scale 0.01 to S; then one
    // attack struck on every enemy within 170 × S of its centre; the impact effect; the sphere
    // grows × 1.02 a frame for 20 frames and is killed. Scale vector at B_DIR, frames left at T_I.
    a.label("gr_fall").pop(Sp, 0);
    a.push_v(Wp, W_RES).push_i(SHOT).push_i(1).push_i(0).sys("trap_pax_start").pop(SpD, B_FX);
    for k in 0..3u16 {
        a.push_f(gravity::START_SCALE).pop(SpD, B_DIR + 4 * k);
    }
    a.push_f(1.0).pop(SpD, B_DIR + 12);
    a.push_v(SpD, B_FX).push_a(SpD, B_POS).sys("trap_effect_set_pos");
    a.push_v(SpD, B_FX).push_a(SpD, B_DIR).sys("trap_effect_set_scale");
    a.push_f(frames).pop(SpD, T_I);
    a.label("gr_falling").halt();
    a.push_v(SpD, B_POS + 4).push_f(gravity::FALL / frames).sys("trap_frametime").bin("mul.s").bin("add.s").pop(SpD, B_POS + 4);
    for k in 0..3u16 {
        a.push_v(SpD, B_DIR + 4 * k).push_f((scale - gravity::START_SCALE) / frames).sys("trap_frametime").bin("mul.s").bin("add.s").pop(SpD, B_DIR + 4 * k);
    }
    a.push_v(SpD, B_FX).push_a(SpD, B_POS).sys("trap_effect_set_pos");
    a.push_v(SpD, B_FX).push_a(SpD, B_DIR).sys("trap_effect_set_scale");
    // KH1's `L0055c` each fall frame: every enemy touching the sphere (170 × its scale now round
    // its centre) is marked for its next update: KH2's own stop with our squash kind, one frame.
    for k in 0..4u16 {
        a.push_i(0).pop(SpD, T_FAKE + 4 * k);
    }
    a.push_a(SpD, T_SEARCH).push_i(T_HISTORY).sys("trap_target_searcher_init");
    a.push_f(1.0e9).pop(SpD, T_SEARCH + 0x14);
    a.push_i(0).pop(SpD, T_TRIES);
    a.label("gr_sq_next");
    a.push_v(SpD, T_TRIES).push_i(T_HISTORY).bin("sub").un("sltz").beqz("gr_sq_done");
    a.push_v(SpD, T_TRIES).push_i(1).bin("add").pop(SpD, T_TRIES);
    a.push_a(SpD, T_SEARCH).push_a(SpD, T_REC).push_a(SpD, B_POS).push_a(SpD, B_POS).push_i(1).push_i(0).push_i(1).sys("trap_target_seracher_search");
    a.push_a(SpD, T_REC).sys("trap_target_is_exist").beqz("gr_sq_done");
    a.push_a(SpD, T_REC).sys("trap_target_pos").memcpy(SpD, 16, B_SCRATCH);
    for (k, at) in [(0u16, 8u16), (4, 12), (8, 16)] {
        a.push_v(SpD, B_SCRATCH + k).push_v(SpD, B_POS + k).bin("sub.s").pop(Sp, at);
    }
    a.push_v(Sp, 8).push_v(Sp, 8).bin("mul.s").push_v(Sp, 12).push_v(Sp, 12).bin("mul.s").bin("add.s").push_v(Sp, 16).push_v(Sp, 16).bin("mul.s").bin("add.s");
    // (170 × scale)²; nearest first: the first one out of reach ends the list.
    a.push_v(SpD, B_DIR).push_f(gravity::RADIUS).bin("mul.s").pop(Sp, 8);
    a.push_v(Sp, 8).push_v(Sp, 8).bin("mul.s").bin("sub.s").un("sgtz.s").bnez("gr_sq_done");
    a.push_v(SpD, T_REC).pop(SpD, T_FAKE + 4);
    a.push_a(SpD, T_FAKE).push_i(gravity::SQUASH_KIND).push_f(1.0).sys("trap_obj_stop");
    a.b("gr_sq_next");
    a.label("gr_sq_done");
    a.push_v(SpD, T_I).sys("trap_frametime").bin("sub.s").pop(SpD, T_I);
    a.push_v(SpD, T_I).un("sgtz.s").bnez("gr_falling");
    // The impact effect: near the floor (a ray of 100 down finds it) 4, else 5.
    a.push_a(SpD, B_POS).push_a(SpD, B_SCRATCH).push_f(gravity::NEAR_FLOOR).sys("trap_bg_ground_pos").beqz("gr_air");
    a.push_v(Wp, W_RES).push_i(POINT).push_i(1).push_i(0).sys("trap_pax_start").pop(Sp, 4).b("gr_impact");
    a.label("gr_air").push_v(Wp, W_RES).push_i(SHOT_B).push_i(1).push_i(0).sys("trap_pax_start").pop(Sp, 4);
    a.label("gr_impact");
    a.push_v(Sp, 4).push_a(SpD, B_POS).sys("trap_effect_set_pos");
    a.push_v(Sp, 4).push_a(SpD, B_DIR).sys("trap_effect_set_scale");
    // The strikes: every enemy (KH2's target searcher, as Thunder's list) whose part centre is
    // within 170 × S of the centre (KH1 `sys 90` type 1 on its bodies: the part's centre here,
    // Assumption), struck once with one attack.
    a.push_a(Wp, W_CASTER).push_i(gravity::ATKP).push_i(-1).push_v(Wp, W_SUB).sys("trap_attack_new").pop(SpD, B_ATK);
    a.push_v(SpD, B_ATK).push_v(Wp, W_RES).sys("trap_attack_set_pax");
    a.push_v(SpD, B_ATK).push_a(SpD, B_POS).sys("trap_attack_set_pos");
    for k in 0..4u16 {
        a.push_i(0).pop(SpD, T_FAKE + 4 * k);
    }
    a.push_a(SpD, T_SEARCH).push_i(T_HISTORY).sys("trap_target_searcher_init");
    a.push_f(1.0e9).pop(SpD, T_SEARCH + 0x14);
    a.push_i(0).pop(SpD, T_TRIES).push_i(0).pop(SpD, T_N);
    a.label("gr_next");
    a.push_v(SpD, T_TRIES).push_i(T_HISTORY).bin("sub").un("sltz").beqz("gr_struck");
    a.push_v(SpD, T_TRIES).push_i(1).bin("add").pop(SpD, T_TRIES);
    a.push_a(SpD, T_SEARCH).push_a(SpD, T_REC).push_a(SpD, B_POS).push_a(SpD, B_POS).push_i(1).push_i(0).push_i(1).sys("trap_target_seracher_search");
    a.push_a(SpD, T_REC).sys("trap_target_is_exist").beqz("gr_struck");
    a.push_a(SpD, T_REC).sys("trap_target_pos").memcpy(SpD, 16, B_SCRATCH);
    for (k, at) in [(0u16, 8u16), (4, 12), (8, 16)] {
        a.push_v(SpD, B_SCRATCH + k).push_v(SpD, B_POS + k).bin("sub.s").pop(Sp, at);
    }
    a.push_v(Sp, 8).push_v(Sp, 8).bin("mul.s").push_v(Sp, 12).push_v(Sp, 12).bin("mul.s").bin("add.s").push_v(Sp, 16).push_v(Sp, 16).bin("mul.s").bin("add.s");
    // Nearest first: the first one out of reach ends the list.
    let r = gravity::RADIUS * scale;
    a.push_f(r * r).bin("sub.s").un("sgtz.s").bnez("gr_struck");
    // One strike an enemy (another part of one struck is passed).
    a.push_i(0).pop(SpD, T_J);
    a.label("gr_dedupe");
    a.push_v(SpD, T_J).push_v(SpD, T_N).bin("sub").un("sltz").beqz("gr_new");
    a.push_v(Sp, 0).push_i(T_LIST as i32).bin("add").push_v(SpD, T_J).push_i(4).bin("mul").bin("add").push_dpop(0);
    a.push_v(SpD, T_REC).bin("sub").un("seqz").bnez("gr_next");
    a.push_v(SpD, T_J).push_i(1).bin("add").pop(SpD, T_J).b("gr_dedupe");
    a.label("gr_new");
    a.push_v(SpD, T_N).push_i(thunder::MAX).bin("sub").un("sltz").beqz("gr_struck");
    a.push_v(Sp, 0).push_i(T_LIST as i32).bin("add").push_v(SpD, T_N).push_i(4).bin("mul").bin("add").push_v(SpD, T_REC).store();
    a.push_v(SpD, T_N).push_i(1).bin("add").pop(SpD, T_N);
    a.push_v(SpD, T_REC).pop(SpD, T_FAKE + 4);
    a.push_v(SpD, B_ATK).push_a(SpD, T_FAKE).push_i(0).push_i(0).sys("trap_attack_strike");
    a.b("gr_next");
    a.label("gr_struck");
    a.push_v(SpD, B_ATK).sys("trap_attack_free");
    a.push_f(gravity::FADE).pop(SpD, T_I);
    a.label("gr_fade").halt();
    for k in 0..3u16 {
        a.push_v(SpD, B_DIR + 4 * k).push_f(gravity::FADE_GROWTH).bin("mul.s").pop(SpD, B_DIR + 4 * k);
    }
    a.push_v(SpD, B_FX).push_a(SpD, B_DIR).sys("trap_effect_set_scale");
    a.push_v(SpD, T_I).sys("trap_frametime").bin("sub.s").pop(SpD, T_I);
    a.push_v(SpD, T_I).un("sgtz.s").bnez("gr_fade");
    a.push_v(SpD, B_FX).sys("trap_effect_kill");
    a.push_v(Wp, W_TARGET).sys("trap_target_free");
    a.push_v(Sp, 0).sys("trap_memory_free");
    a.ret();
}

fn stop_code(level: usize, a: &mut Asm) {
    use Mode::*;
    // Trigger 20 (KH1 `L00890`): wait 4; the charge; each frame for 24 the cast motion still
    // playing (else the charge fades and nothing more); at 28 the release.
    a.label("cast").pop(Sp, 0).pop(Sp, 4).pop(Sp, 8).pop(Sp, 12);
    a.push_v(Sp, 12).memcpy(Wp, 16, W_CASTER);
    a.push_v(Sp, 8).pop(Wp, W_RES);
    a.push_v(Sp, 4).pop(Wp, W_SUB);
    a.push_v(Sp, 0).pop(Wp, W_MAGI);
    a.push_a(Wp, W_CASTER).push_dpop(4).sys("trap_sysobj_motion_id").pop(Wp, W_MOTION);
    a.push_a(Wp, W_CASTER).sys("trap_btlobj_target").sys("trap_target_dup").pop(Wp, W_TARGET);
    a.push_f(stop::CHARGE_AT).pop(Sp, 16);
    a.label("st_wait").halt().push_v(Sp, 16).sys("trap_frametime").bin("sub.s").pop(Sp, 16);
    a.push_v(Sp, 16).un("sgtz.s").bnez("st_wait");
    a.push_v(Wp, W_RES).push_i(CAST).push_a(Wp, W_CASTER).push_i(1).push_i(0).sys("trap_pax_start_bind").pop(Wp, W_CAST_FX);
    a.push_f(stop::CHECK).pop(Sp, 16);
    a.label("st_check");
    a.push_a(Wp, W_CASTER).push_dpop(4).sys("trap_sysobj_motion_id").push_v(Wp, W_MOTION).bin("sub").un("seqz").bnez("st_still");
    a.push_v(Wp, W_CAST_FX).push_f(4.0).push_i(0).sys("trap_effect_fadeout");
    a.push_v(Wp, W_TARGET).sys("trap_target_free");
    a.ret();
    // KH1: the check, the step, the frame's end; out of the loop the next frame (28).
    a.label("st_still").push_v(Sp, 16).sys("trap_frametime").bin("sub.s").pop(Sp, 16);
    a.halt();
    a.push_v(Sp, 16).un("sgtz.s").bnez("st_check");
    // The release: effect 1; the strike thread.
    a.label("st_release");
    a.push_v(Wp, W_RES).push_i(HAND).push_a(Wp, W_CASTER).push_i(1).push_i(0).sys("trap_pax_start_bind").drop();
    new_block_of(a, T_BLOCK);
    a.push_v(Sp, 0).push_i(B_THREAD as i32).bin("add").push_pc("st_strikes").push_v(Sp, 0).sys("trap_magic_start_thread").store();
    a.ret();
    // KH2's own release event (trigger 21, at KH1 Thunder's 30 on these motions): KH1's Stop
    // released at 28 by itself.
    a.label("release").ret();

    // The strikes (KH1 `0x44..0x206`): the centre on the target if it lives, else Sora; every
    // enemy nearest first (KH2's target searcher, one entry an enemy); down the list, each one in
    // the cylinder gets KH1's look, bound to it, and one strike (ATKP 2067: KH2 stops it for
    // KH1's time, `kh2-hook`), then 4 frames; one outside is passed over at once.
    a.label("st_strikes").pop(Sp, 0);
    a.push_v(Wp, W_TARGET).sys("trap_target_is_exist").beqz("st_alone");
    a.push_v(Wp, W_TARGET).sys("trap_target_pos").memcpy(SpD, 16, B_POS).b("st_centred");
    a.label("st_alone").push_a(Wp, W_CASTER).sys("trap_obj_pos").memcpy(SpD, 16, B_POS);
    a.label("st_centred");
    a.push_a(Wp, W_CASTER).push_i(stop::ATKP).push_i(-1).push_v(Wp, W_SUB).sys("trap_attack_new").pop(SpD, B_ATK);
    a.push_v(SpD, B_ATK).push_v(Wp, W_RES).sys("trap_attack_set_pax");
    a.push_v(SpD, B_ATK).push_a(SpD, B_POS).sys("trap_attack_set_pos");
    for k in 0..4u16 {
        a.push_i(0).pop(SpD, T_FAKE + 4 * k);
    }
    a.push_a(SpD, T_SEARCH).push_i(T_HISTORY).sys("trap_target_searcher_init");
    a.push_f(1.0e9).pop(SpD, T_SEARCH + 0x14);
    a.push_i(0).pop(SpD, T_TRIES).push_i(0).pop(SpD, T_N);
    a.label("st_next");
    a.push_v(SpD, T_TRIES).push_i(T_HISTORY).bin("sub").un("sltz").beqz("st_done");
    a.push_v(SpD, T_TRIES).push_i(1).bin("add").pop(SpD, T_TRIES);
    a.push_a(SpD, T_SEARCH).push_a(SpD, T_REC).push_a(SpD, B_POS).push_a(SpD, B_POS).push_i(1).push_i(0).push_i(1).sys("trap_target_seracher_search");
    a.push_a(SpD, T_REC).sys("trap_target_is_exist").beqz("st_done");
    a.push_i(0).pop(SpD, T_J);
    a.label("st_dedupe");
    a.push_v(SpD, T_J).push_v(SpD, T_N).bin("sub").un("sltz").beqz("st_new");
    a.push_v(Sp, 0).push_i(T_LIST as i32).bin("add").push_v(SpD, T_J).push_i(4).bin("mul").bin("add").push_dpop(0);
    a.push_v(SpD, T_REC).bin("sub").un("seqz").bnez("st_next");
    a.push_v(SpD, T_J).push_i(1).bin("add").pop(SpD, T_J).b("st_dedupe");
    a.label("st_new");
    a.push_v(SpD, T_N).push_i(thunder::MAX).bin("sub").un("sltz").beqz("st_done");
    a.push_v(Sp, 0).push_i(T_LIST as i32).bin("add").push_v(SpD, T_N).push_i(4).bin("mul").bin("add").push_v(SpD, T_REC).store();
    a.push_v(SpD, T_N).push_i(1).bin("add").pop(SpD, T_N);
    // In the cylinder (KH1 `sys 90` type 2 on its bodies: the part's centre here, Assumption)?
    a.push_a(SpD, T_REC).sys("trap_target_pos").memcpy(SpD, 16, B_SCRATCH);
    for (k, at) in [(0u16, 8u16), (8, 12), (4, 16)] {
        a.push_v(SpD, B_SCRATCH + k).push_v(SpD, B_POS + k).bin("sub.s").pop(Sp, at);
    }
    let r = stop::RADIUS[level];
    a.push_v(Sp, 8).push_v(Sp, 8).bin("mul.s").push_v(Sp, 12).push_v(Sp, 12).bin("mul.s").bin("add.s");
    a.push_f(r * r).bin("sub.s").un("sgtz.s").bnez("st_next");
    a.push_v(Sp, 16).un("abs.s").push_f(stop::HALF_HEIGHT).bin("sub.s").un("sgtz.s").bnez("st_next");
    a.push_v(SpD, T_REC).pop(SpD, T_FAKE + 4);
    // KH1's look on it, in a block of its own with its watcher.
    a.push_i(32).sys("trap_memory_alloc").pop(Sp, 4);
    for k in 0..4u16 {
        a.push_v(Sp, 4).push_i(4 * k as i32).bin("add").push_v(SpD, T_FAKE + 4 * k).store();
    }
    a.push_v(Sp, 4).push_i(16).bin("add").push_v(Wp, W_RES).push_i(MARK).push_v(Sp, 4).push_i(1).push_i(0).sys("trap_pax_start_bind").store();
    a.push_pc("st_look").push_v(Sp, 4).sys("trap_magic_start_thread").drop();
    a.push_v(SpD, B_ATK).push_a(SpD, T_FAKE).push_i(0).push_i(0).sys("trap_attack_strike");
    a.push_f(stop::BETWEEN).pop(Sp, 20);
    a.label("st_between").halt().push_v(Sp, 20).sys("trap_frametime").bin("sub.s").pop(Sp, 20);
    a.push_v(Sp, 20).un("sgtz.s").bnez("st_between");
    a.b("st_next");
    a.label("st_done");
    a.push_v(SpD, B_ATK).sys("trap_attack_free");
    a.push_v(Wp, W_TARGET).sys("trap_target_free");
    a.push_v(Sp, 0).sys("trap_memory_free");
    a.ret();

    // A stopped enemy's look: from 2 frames on, while KH2 holds it stopped; then it ends.
    a.label("st_look").pop(Sp, 0);
    a.push_f(stop::LOOK_FROM).pop(Sp, 4);
    a.label("st_look_wait").halt().push_v(Sp, 4).sys("trap_frametime").bin("sub.s").pop(Sp, 4);
    a.push_v(Sp, 4).un("sgtz.s").bnez("st_look_wait");
    a.label("st_look_on").push_v(Sp, 0).sys("trap_obj_is_stop").beqz("st_look_end");
    a.halt().b("st_look_on");
    a.label("st_look_end").push_v(SpD, 16).sys("trap_effect_loop_end");
    a.push_v(Sp, 0).sys("trap_memory_free");
    a.ret();
}

fn aero_code(level: usize, a: &mut Asm) {
    use Mode::*;
    // Trigger 20 (KH1 `0x4C4..0x674`): the charge; up to 30 frames while the cast motion plays;
    // then (or at once when it changes: never dropped) the cast.
    a.label("cast").pop(Sp, 0).pop(Sp, 4).pop(Sp, 8).pop(Sp, 12);
    a.push_v(Sp, 12).memcpy(Wp, 16, W_CASTER);
    a.push_v(Sp, 8).pop(Wp, W_RES);
    a.push_v(Sp, 4).pop(Wp, W_SUB);
    a.push_v(Sp, 0).pop(Wp, W_MAGI);
    a.push_a(Wp, W_CASTER).push_dpop(4).sys("trap_sysobj_motion_id").pop(Wp, W_MOTION);
    a.push_v(Wp, W_RES).push_i(CAST).push_a(Wp, W_CASTER).push_i(1).push_i(0).sys("trap_pax_start_bind").pop(Wp, W_CAST_FX);
    a.push_f(aero::CAST_FRAMES).pop(Sp, 16);
    a.label("ae_wait").halt();
    a.push_a(Wp, W_CASTER).push_dpop(4).sys("trap_sysobj_motion_id").push_v(Wp, W_MOTION).bin("sub").un("seqz").beqz("ae_cast");
    a.push_v(Sp, 16).sys("trap_frametime").bin("sub.s").pop(Sp, 16);
    a.push_v(Sp, 16).un("sgtz.s").bnez("ae_wait");
    a.label("ae_cast");
    // The aura at Sora; the barrier strike on him (KH2 computes the barrier, `kh2-hook`); his
    // bars back from the attack; the barrier's frames.
    a.push_v(Wp, W_RES).push_i(POINT).push_i(1).push_i(0).sys("trap_pax_start").pop(Sp, 4);
    a.push_v(Sp, 4).push_a(Wp, W_CASTER).sys("trap_obj_pos").sys("trap_effect_set_pos");
    new_block(a);
    a.push_a(Wp, W_CASTER).push_i(aero::ATKP).push_i(-1).push_v(Wp, W_SUB).sys("trap_attack_new").pop(SpD, B_ATK);
    a.push_v(SpD, B_ATK).push_a(Wp, W_CASTER).push_i(0).push_i(0).sys("trap_attack_strike");
    a.push_v(SpD, B_ATK).sys("trap_attack_get_reflect_count").un("cvt.s.w").push_f(aero::BASE).bin("add.s").push_f(60.0).bin("mul.s").pop(SpD, B_SCRATCH);
    a.push_v(SpD, B_ATK).sys("trap_attack_free");
    a.push_v(Sp, 0).push_i(B_THREAD as i32).bin("add").push_pc("ae_barrier").push_v(Sp, 0).sys("trap_magic_start_thread").store();
    a.ret();
    // KH2's own release event (trigger 21): Aero is cast by the trigger above.
    a.label("release").ret();

    // The barrier's look (from 24 frames) and, from Aerora, the wind, while it lasts.
    a.label("ae_barrier").pop(Sp, 0);
    a.push_i(0).pop(SpD, B_FX).push_i(0).pop(SpD, B_EFFECT).push_i(0).pop(SpD, B_ATK);
    a.push_f(0.0).pop(SpD, T_ORIGIN).push_f(0.0).pop(SpD, T_ORIGIN + 4);
    if level > 0 {
        a.push_v(Wp, W_RES).push_i(MARK).push_a(Wp, W_CASTER).push_i(1).push_i(0).sys("trap_pax_start_bind").pop(SpD, B_EFFECT);
    }
    a.label("ae_frame");
    // The look, once, 24 frames in.
    a.push_v(SpD, B_FX).bnez("ae_looked");
    a.push_v(SpD, T_ORIGIN).push_f(aero::LOOK_AFTER).bin("sub.s").un("sltz.s").bnez("ae_looked");
    a.push_v(Wp, W_RES).push_i(SHOT).push_a(Wp, W_CASTER).push_i(1).push_i(0).sys("trap_pax_start_bind").pop(SpD, B_FX);
    a.label("ae_looked");
    if level > 0 {
        // The wind: a new attack every 30 frames (struck again), at Sora's body centre.
        a.push_v(SpD, T_ORIGIN + 4).un("sgtz.s").bnez("ae_blow");
        a.push_v(SpD, B_ATK).beqz("ae_fresh");
        a.push_v(SpD, B_ATK).sys("trap_attack_free");
        a.label("ae_fresh");
        a.push_a(Wp, W_CASTER).push_i(aero::WIND_ATKP).push_i(-1).push_v(Wp, W_SUB).sys("trap_attack_new").pop(SpD, B_ATK);
        a.push_v(SpD, B_ATK).push_v(Wp, W_RES).sys("trap_attack_set_pax");
        a.push_v(SpD, B_ATK).push_f(aero::WIND_RADIUS).push_f(aero::WIND_RADIUS).sys("trap_attack_set_radius");
        a.push_f(aero::WIND_EVERY).pop(SpD, T_ORIGIN + 4);
        a.label("ae_blow");
        for (o, v) in [(0, 0.0f32), (4, 0.0), (8, 0.0), (12, 1.0)] {
            a.push_f(v).pop(SpD, B_DIR + o);
        }
        a.push_a(Wp, W_CASTER).push_i(cure::BONE).push_a(SpD, B_DIR).sys("trap_obj_apply_bone_matrix").memcpy(SpD, 16, B_POS);
        a.push_v(SpD, B_ATK).push_a(SpD, B_POS).sys("trap_attack_set_pos");
        a.push_v(SpD, T_ORIGIN + 4).sys("trap_frametime").bin("sub.s").pop(SpD, T_ORIGIN + 4);
    }
    a.push_v(SpD, T_ORIGIN).sys("trap_frametime").bin("add.s").pop(SpD, T_ORIGIN);
    a.push_v(SpD, T_ORIGIN).push_v(SpD, B_SCRATCH).bin("sub.s").un("sltz.s").beqz("ae_over");
    a.halt().b("ae_frame");
    a.label("ae_over");
    a.push_v(SpD, B_FX).beqz("ae_no_look");
    a.push_v(SpD, B_FX).sys("trap_effect_loop_end");
    a.label("ae_no_look");
    if level > 0 {
        a.push_v(SpD, B_EFFECT).sys("trap_effect_loop_end");
        a.push_v(SpD, B_ATK).sys("trap_attack_free");
    }
    a.push_v(Sp, 0).sys("trap_memory_free");
    a.ret();
}

fn cure_code(a: &mut Asm) {
    use Mode::*;
    // Trigger 21 (KH1's release, frame 36; `xm_cure01.mag` `0x56`): the heal struck straight onto
    // Sora (KH1 `sys 23` / `sys 60` onto the command's member, else the caster; KH2's menu picks
    // none, so Sora), then the heal effect, held at the body centre's height over his feet and
    // following him while it plays.
    a.label("release");
    for (o, v) in [(0, 0.0f32), (4, 0.0), (8, 0.0), (12, 1.0)] {
        a.push_f(v).pop(Wp, W_ZERO + o);
    }
    new_block(a);
    a.push_a(Wp, W_CASTER).push_i(cure::ATKP).push_i(-1).push_v(Wp, W_SUB).sys("trap_attack_new").pop(SpD, B_ATK);
    a.push_v(SpD, B_ATK).push_a(Wp, W_CASTER).push_i(0).push_i(0).sys("trap_attack_strike");
    a.push_v(SpD, B_ATK).sys("trap_attack_free");
    a.push_v(Wp, W_RES).push_i(SHOT).push_i(1).push_i(0).sys("trap_pax_start").pop(SpD, B_FX);
    a.push_a(Wp, W_CASTER).push_i(cure::BONE).push_a(Wp, W_ZERO).sys("trap_obj_apply_bone_matrix").push_dpop(4);
    a.push_a(Wp, W_CASTER).sys("trap_obj_pos").push_dpop(4).bin("sub.s").pop(SpD, B_SCRATCH);
    a.push_v(Sp, 0).push_i(B_THREAD as i32).bin("add").push_pc("heal").push_v(Sp, 0).sys("trap_magic_start_thread").store();
    a.ret();

    a.label("heal").pop(Sp, 0);
    a.label("heal_follow");
    a.push_v(SpD, B_FX).sys("trap_effect_is_alive").beqz("heal_end");
    a.push_v(Sp, 0).sys("trap_obj_pos").memcpy(SpD, 16, B_POS);
    a.push_v(SpD, B_POS + 4).push_v(SpD, B_SCRATCH).bin("add.s").pop(SpD, B_POS + 4);
    a.push_v(SpD, B_FX).push_a(SpD, B_POS).sys("trap_effect_set_pos");
    a.halt().b("heal_follow");
    a.label("heal_end").push_v(Sp, 0).sys("trap_memory_free");
    a.ret();
}

/// KH2's own spell PAX with KH1's set added: KH2's elements and effects first, unchanged (the
/// Drive Forms run KH2's own script on them), then ours; one DPX of both sets' effects (KH1's
/// renumbered by `KH1_NUMBERS`: KH2 finds an effect by its number, Checked (decomp `0x1402BC9B0`
/// per the sister's `kh2-effects.md` §3) and both DPDs, each on a 128-byte boundary as KH2's own
/// sits. KH2's DPX load relocates every DPD of the list (`0x1402BDDE0` → `0x1402C21E0` for each),
/// Checked (decomp). Returns the PAX and `KH1_NUMBERS`.
pub fn merge_pax(kh2_pax: &[u8], kh1_set: &[u8], ours: impl Fn(u16) -> Vec<Element>) -> Option<(Vec<u8>, u16)> {
    if kh2_pax.get(0..4)? != b"PAX_" {
        return None;
    }
    let n_el = u32_at(kh2_pax, 8)? as usize;
    let dpx2 = kh2_pax.get(u32_at(kh2_pax, 12)? as usize..)?;
    let (n2, n1) = (u32_at(dpx2, 12)? as usize, u32_at(kh1_set, 12)? as usize);
    let dpd_of = |set: &[u8], n: usize| -> Option<(usize, usize)> {
        let at = 16 + n * 32;
        if u32_at(set, at)? != 1 {
            return None; // one DPD each (KH2's spells and KH1's, Checked (files))
        }
        let start = u32_at(set, at + 4)? as usize;
        Some((start, set.len()))
    };
    let (a2, e2) = dpd_of(dpx2, n2)?;
    let (a1, e1) = dpd_of(kh1_set, n1)?;
    let head = 16 + (n2 + n1) * 32 + 4 + 8;
    let align = |x: usize| (x + 127) & !127;
    let at2 = align(head);
    let at1 = align(at2 + (e2 - a2));
    let mut dpx = Vec::with_capacity(at1 + (e1 - a1));
    dpx.extend_from_slice(&dpx2[0..12]);
    dpx.extend_from_slice(&((n2 + n1) as u32).to_le_bytes());
    for i in 0..n2 {
        let mut e = dpx2.get(16 + i * 32..48 + i * 32)?.to_vec();
        e[0..4].copy_from_slice(&(at2 as u32).to_le_bytes());
        dpx.extend_from_slice(&e);
    }
    for i in 0..n1 {
        let mut e = kh1_set.get(16 + i * 32..48 + i * 32)?.to_vec();
        e[0..4].copy_from_slice(&(at1 as u32).to_le_bytes());
        let number = u32_at(&e, 8)? + KH1_NUMBERS as u32;
        e[8..12].copy_from_slice(&number.to_le_bytes());
        dpx.extend_from_slice(&e);
    }
    dpx.extend_from_slice(&2u32.to_le_bytes());
    dpx.extend_from_slice(&(at2 as u32).to_le_bytes());
    dpx.extend_from_slice(&(at1 as u32).to_le_bytes());
    dpx.resize(at2, 0);
    dpx.extend_from_slice(&dpx2[a2..e2]);
    dpx.resize(at1, 0);
    dpx.extend_from_slice(&kh1_set[a1..e1]);
    let mine = ours(KH1_NUMBERS);
    let total = n_el + mine.len();
    let el_end = 16 + total * 80;
    let dpx_at = (el_end + 15) & !15;
    let mut out = Vec::with_capacity(dpx_at + dpx.len());
    out.extend_from_slice(b"PAX_");
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(dpx_at as u32).to_le_bytes());
    out.extend_from_slice(kh2_pax.get(16..16 + n_el * 80)?);
    let ours_bytes = pax(&mine, &[]);
    out.extend_from_slice(&ours_bytes[16..16 + mine.len() * 80]);
    out.resize(dpx_at, 0);
    out.extend_from_slice(&dpx);
    Some((out, KH1_NUMBERS))
}

/// KH1's effect numbers in a merged set: KH1's own plus 200 (KH2's spells number theirs from 0,
/// KH1's from 0 and 100, Checked (files): FIRE_1.mag 0..7, xm_fire01.mag 0..4 and 100..104).
pub const KH1_NUMBERS: u16 = 200;

#[cfg(test)]
mod tests {
    use super::*;
    use kh2_bdx::{BdxError, Cell, CtxId, Host, Vm};
    use std::collections::HashMap;

    /// Just enough of KH2's traps (as the sister's `kh2-magic` and our decomp reading document
    /// them) to fly KH1's shots. The Keyblade's tip at (0, -100, 30), its middle at (0, -100, 15);
    /// Sora faces +z (yaw 0); the floor at y = 0 (y down).
    pub(super) struct World {
        target: [f32; 3],
        target_alive: bool,
        hit_at: Option<usize>,
        frame: usize,
        /// Each attack's positions, by its handle.
        pub(super) attacks: HashMap<i32, Vec<[f32; 3]>>,
        /// Each attack's group (a dup shares its source's).
        pub(super) groups: HashMap<i32, i32>,
        pub(super) radius: HashMap<i32, f32>,
        next: i32,
        fx_alive: HashMap<i32, bool>,
        pub(super) started: Vec<(i32, [f32; 3])>,
        pub(super) fx_ids: HashMap<i32, i32>,
        fx_pos: HashMap<i32, [f32; 3]>,
        pub(super) faded_at: Option<usize>,
        pub(super) loop_ended: usize,
        pub(super) freed: usize,
        threads: Vec<CtxId>,
        slot: u32,
        slot2: u32,
        /// The frame the cast motion ends.
        pub(super) motion_end: usize,
        /// Battle targets: (object handle, position, alive).
        pub(super) enemies: Vec<(i32, [f32; 3], bool)>,
        /// The searcher's history, by its address.
        history: HashMap<u32, Vec<i32>>,
        /// (frame, object) of each strike.
        pub(super) strikes: Vec<(usize, i32)>,
        pub(super) light: Vec<(usize, &'static str, f32)>,
        pub(super) scales: Vec<(usize, i32, f32)>,
        /// Stop: each struck enemy stays stopped until this frame (KH2's stop, set by the hook).
        pub(super) stopped_until: HashMap<i32, usize>,
        /// Attacks made: (frame, ATKP).
        pub(super) made: Vec<(usize, i32)>,
        pub(super) bound: Vec<(usize, i32)>,
        /// KH2's stop put on an object by the script: (frame, object, kind, frames).
        pub(super) obj_stops: Vec<(usize, i32, i32, f32)>,
    }

    impl World {
        fn new(target: Option<[f32; 3]>, hit_at: Option<usize>) -> World {
            World {
                target: target.unwrap_or_default(),
                target_alive: target.is_some(),
                hit_at,
                frame: 0,
                attacks: HashMap::new(),
                groups: HashMap::new(),
                radius: HashMap::new(),
                next: 100,
                fx_alive: HashMap::new(),
                started: vec![],
                fx_ids: HashMap::new(),
                fx_pos: HashMap::new(),
                faded_at: None,
                loop_ended: 0,
                freed: 0,
                threads: vec![],
                slot: 0,
                slot2: 0,
                motion_end: 28,
                enemies: vec![],
                history: HashMap::new(),
                strikes: vec![],
                light: vec![],
                scales: vec![],
                stopped_until: HashMap::new(),
                made: vec![],
                bound: vec![],
                obj_stops: vec![],
            }
        }

        fn ret_vec(&mut self, vm: &mut Vm, v: [f32; 4]) -> Cell {
            if self.slot == 0 {
                self.slot = vm.mem.alloc(16);
            }
            vm.mem.set_vec4(self.slot, v).unwrap();
            Cell::addr(self.slot)
        }

        fn handle(&mut self) -> i32 {
            self.next += 1;
            self.next
        }

        /// The ball's positions (the one attack Fire makes).
        pub(super) fn positions(&self) -> Vec<[f32; 3]> {
            self.attacks.values().next().cloned().unwrap_or_default()
        }
    }

    impl Host for World {
        fn syscall(&mut self, vm: &mut Vm, ctx: CtxId, table: u8, index: u16, args: &mut [Cell]) -> Result<(), BdxError> {
            let name = kh2_bdx::trap_name(table, index).unwrap_or("?");
            let v = |vm: &Vm, a: Cell| vm.mem.vec4(a.v).unwrap();
            match name {
                "trap_memory_alloc" => args[0] = Cell::addr(vm.mem.alloc(args[0].i() as u32)),
                "trap_memory_free" => self.freed += 1,
                "trap_btlobj_target" | "trap_target_dup" => args[0] = Cell::int(7),
                "trap_target_free" | "trap_attack_set_pax" | "trap_effect_set_dir" => {}
                "trap_attack_set_radius" => {
                    self.radius.insert(args[0].i(), args[1].f());
                }
                "trap_pax_start_bind" | "trap_pax_start" => {
                    if name == "trap_pax_start_bind" {
                        self.bound.push((self.frame, args[1].i()));
                    }
                    let h = self.handle();
                    self.fx_alive.insert(h, true);
                    self.fx_ids.insert(h, args[1].i());
                    args[0] = Cell::int(h);
                }
                "trap_effect_set_pos" => {
                    let p = v(vm, args[1]);
                    let h = args[0].i();
                    if !self.fx_pos.contains_key(&h) {
                        self.started.push((self.fx_ids[&h], [p[0], p[1], p[2]]));
                    }
                    self.fx_pos.insert(h, [p[0], p[1], p[2]]);
                }
                "trap_attack_new" => {
                    self.made.push((self.frame, args[1].i()));
                    let h = self.handle();
                    self.groups.insert(h, h);
                    self.attacks.insert(h, vec![]);
                    args[0] = Cell::int(h)
                }
                "trap_attack_dup" => {
                    let h = self.handle();
                    let g = self.groups[&args[0].i()];
                    self.groups.insert(h, g);
                    self.attacks.insert(h, vec![]);
                    args[0] = Cell::int(h)
                }
                "trap_sysobj_motion_id" => args[0] = Cell::int(if self.frame < self.motion_end { 56 } else { 0 }),
                "trap_obj_apply_bone_matrix" => {
                    let z = match args[1].i() {
                        0x4003 => 30.0,
                        0x4007 => 15.0,
                        0x4001 => {
                            args[0] = self.ret_vec(vm, [0.0, -90.0, 0.0, 1.0]);
                            return Ok(());
                        }
                        b => panic!("bone {b:#x}"),
                    };
                    args[0] = self.ret_vec(vm, [0.0, -100.0, z, 1.0])
                }
                "trap_obj_target_pos" => args[0] = self.ret_vec(vm, [0.0, -85.0, 0.0, 1.0]),
                "trap_obj_dir" => args[0] = self.ret_vec(vm, [0.0, 0.0, 1.0, 0.0]),
                "trap_vector_roty" => {
                    // (sin y, 0, cos y) turned by a: about y, as KH2's own facing turns.
                    let (p, ang) = (v(vm, args[0]), args[1].f());
                    let (s, c) = ang.sin_cos();
                    args[0] = self.ret_vec(vm, [p[0] * c + p[2] * s, p[1], -p[0] * s + p[2] * c, p[3]])
                }
                "trap_vector_add" => {
                    let (a, b) = (v(vm, args[0]), v(vm, args[1]));
                    args[0] = self.ret_vec(vm, [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3]])
                }
                "trap_bg_ground_pos" => {
                    // The floor at y = 0: a hit when the ray from y down by `depth` reaches it.
                    let (p, depth) = (v(vm, args[0]), args[2].f());
                    let hit = p[1] <= 0.0 && p[1] + depth >= 0.0;
                    let out = if hit { [p[0], 0.0, p[2], p[3]] } else { p };
                    vm.mem.set_vec4(args[1].v, out).unwrap();
                    args[0] = Cell::int(hit as i32);
                }
                "trap_attack_set_pos" => {
                    let p = v(vm, args[1]);
                    self.attacks.get_mut(&args[0].i()).unwrap().push([p[0], p[1], p[2]]);
                }
                "trap_effect_is_alive" => args[0] = Cell::int(self.fx_alive.get(&args[0].i()).copied().unwrap_or(false) as i32),
                "trap_attack_is_hit_bg" | "trap_attack_is_reflect" => args[0] = Cell::int(0),
                "trap_attack_is_hit" => args[0] = Cell::int(self.hit_at.is_some_and(|h| self.frame >= h) as i32),
                // The battle target's record is 7; anything else is a record in script memory (its
                // object at +0).
                "trap_target_is_exist" => {
                    let alive = if args[0].i() == 7 {
                        self.target_alive
                    } else {
                        let o = vm.mem.u32(args[0].v).unwrap() as i32;
                        self.enemies.iter().any(|e| e.0 == o && e.2)
                    };
                    args[0] = Cell::int(alive as i32)
                }
                "trap_target_pos" => {
                    let t = if args[0].i() == 7 {
                        self.target
                    } else {
                        let o = vm.mem.u32(args[0].v).unwrap() as i32;
                        self.enemies.iter().find(|e| e.0 == o).unwrap().1
                    };
                    args[0] = self.ret_vec(vm, [t[0], t[1], t[2], 1.0])
                }
                "trap_obj_pos" => args[0] = self.ret_vec(vm, [0.0, 0.0, 0.0, 1.0]),
                "trap_camera_at" => args[0] = self.ret_vec(vm, [0.0, -100.0, 300.0, 1.0]),
                "trap_camera_eye" => {
                    if self.slot2 == 0 {
                        self.slot2 = vm.mem.alloc(16);
                    }
                    vm.mem.set_vec4(self.slot2, [0.0, -200.0, -300.0, 1.0]).unwrap();
                    args[0] = Cell::addr(self.slot2)
                }
                "trap_vector_len" => {
                    let p = v(vm, args[0]);
                    args[0] = Cell::float((p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt())
                }
                "trap_vector_normalize" => {
                    let p = v(vm, args[0]);
                    let l = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
                    args[0] = self.ret_vec(vm, [p[0] / l, p[1] / l, p[2] / l, p[3]])
                }
                "trap_vector_mul" => {
                    let (p, k) = (v(vm, args[0]), args[1].f());
                    args[0] = self.ret_vec(vm, [p[0] * k, p[1] * k, p[2] * k, p[3]])
                }
                "trap_light_create" => {
                    self.light.push((self.frame, "create", args[0].i() as f32));
                    args[0] = Cell::int(77)
                }
                "trap_light_set_color" => self.light.push((self.frame, "colour", args[4].f())),
                "trap_light_fadeout" => self.light.push((self.frame, "fadeout", args[1].f())),
                "trap_target_searcher_init" => {
                    self.history.insert(args[0].v, vec![]);
                }
                "trap_target_seracher_search" => {
                    // The nearest live target not in the history, from args[2]; none: record 0.
                    let (from, near) = (v(vm, args[2]), args[4].i() != 0);
                    assert!(near && args[6].i() != 0);
                    let seen = self.history.get(&args[0].v).cloned().unwrap_or_default();
                    let d = |p: [f32; 3]| ((p[0] - from[0]).powi(2) + (p[1] - from[1]).powi(2) + (p[2] - from[2]).powi(2)).sqrt();
                    let best = self.enemies.iter().filter(|e| e.2 && !seen.contains(&e.0)).min_by(|a, b| d(a.1).partial_cmp(&d(b.1)).unwrap()).map(|e| e.0);
                    let rec = args[1].v;
                    vm.mem.set_u32(rec, best.unwrap_or(0) as u32).unwrap();
                    vm.mem.set_u32(rec + 4, 0).unwrap();
                    if let Some(o) = best {
                        self.history.get_mut(&args[0].v).unwrap().push(o);
                    }
                }
                "trap_obj_stop" => {
                    let o = vm.mem.u32(args[0].v + 4).unwrap() as i32;
                    self.obj_stops.push((self.frame, o, args[1].v as i32, args[2].f()));
                }
                "trap_attack_strike" => {
                    let o = vm.mem.u32(args[1].v + 4).unwrap() as i32;
                    self.stopped_until.insert(o, self.frame + 30);
                    self.strikes.push((self.frame, o));
                }
                "trap_vector_sub" => {
                    let (a, b) = (v(vm, args[0]), v(vm, args[1]));
                    args[0] = self.ret_vec(vm, [a[0] - b[0], a[1] - b[1], a[2] - b[2], 0.0])
                }
                "trap_vector_homing" => {
                    // Turns the unit vector at args[0] toward args[1] by at most args[2] (radians).
                    let (d, t, max) = (v(vm, args[0]), v(vm, args[1]), args[2].f());
                    let tl = (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt();
                    let tn = [t[0] / tl, t[1] / tl, t[2] / tl];
                    let ang = (d[0] * tn[0] + d[1] * tn[1] + d[2] * tn[2]).clamp(-1.0, 1.0).acos();
                    let k = if ang <= max { 1.0 } else { max / ang };
                    let mix = [d[0] + (tn[0] - d[0]) * k, d[1] + (tn[1] - d[1]) * k, d[2] + (tn[2] - d[2]) * k];
                    let l = (mix[0] * mix[0] + mix[1] * mix[1] + mix[2] * mix[2]).sqrt();
                    vm.mem.set_vec4(args[0].v, [mix[0] / l, mix[1] / l, mix[2] / l, d[3]]).unwrap();
                }
                "trap_vector_addf" => {
                    let (p, d, s) = (v(vm, args[0]), v(vm, args[1]), args[2].f());
                    let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                    vm.mem.set_vec4(args[0].v, [p[0] + d[0] / l * s, p[1] + d[1] / l * s, p[2] + d[2] / l * s, p[3]]).unwrap();
                }
                "trap_frametime" => args[0] = Cell::float(1.0),
                "trap_effect_fadeout" => self.faded_at = Some(self.frame),
                "trap_effect_kill" => {
                    self.fx_alive.insert(args[0].i(), false);
                }
                "trap_effect_loop_end" => {
                    self.loop_ended += 1;
                    self.fx_alive.insert(args[0].i(), false);
                }
                "trap_attack_free" => {}
                "trap_attack_get_reflect_count" => args[0] = Cell::int(3),
                "trap_obj_is_stop" => {
                    let o = vm.mem.u32(args[0].v + 4).unwrap() as i32;
                    args[0] = Cell::int(self.stopped_until.get(&o).is_some_and(|&u| self.frame < u) as i32)
                }
                "trap_effect_set_scale" => {
                    let p = v(vm, args[1]);
                    self.scales.push((self.frame, args[0].i(), p[0]));
                }
                "trap_magic_start_thread" => {
                    let wp = vm.ctxs[ctx].wp;
                    let t = vm.spawn(Some(wp));
                    vm.start_at(t, args[0].i() as u32, &[args[1]]).unwrap();
                    self.threads.push(t);
                    args[0] = Cell::int(t as i32);
                }
                _ => panic!("trap {name} not modelled"),
            }
            Ok(())
        }
    }

    /// Casts `script` (sub `sub`) at frame 0, releases at 28 (KH1's release, t69), runs 120 frames.
    pub(super) fn cast_in(script: &[u8], sub: i32, target: Option<[f32; 3]>, hit_at: Option<usize>) -> World {
        cast_with(script, sub, World::new(target, hit_at))
    }

    pub(super) fn cast_with(script: &[u8], sub: i32, w: World) -> World {
        let prog = kh2_bdx::Program::parse(script).unwrap();
        let mut vm = Vm::new(prog);
        let mut w = w;
        let caster = vm.mem.alloc(16);
        let cast = vm.spawn(None);
        vm.trigger(cast, 20, &[Cell::addr(caster), Cell::int(3), Cell::int(sub), Cell::addr(0)]).unwrap();
        let mut release = None;
        for f in 0..120 {
            w.frame = f;
            if !vm.ctxs[cast].done && vm.run(cast, &mut w).unwrap() != kh2_bdx::Status::Yield {
                vm.ctxs[cast].done = true;
            }
            if f == 28 {
                let r = vm.spawn(Some(vm.ctxs[cast].wp));
                vm.trigger(r, 21, &[]).unwrap();
                vm.run(r, &mut w).unwrap();
                release = Some(r);
            }
            // Threads a thread starts run from the next frame (their argument is on their stack
            // until then).
            let ran = w.threads.clone();
            for &t in &ran {
                if !vm.ctxs[t].done && vm.run(t, &mut w).unwrap() != kh2_bdx::Status::Yield {
                    vm.ctxs[t].done = true;
                }
            }
            // Every trap call took what it was given: nothing left on any value stack between frames.
            for c in [cast].into_iter().chain(release).chain(ran.iter().copied()) {
                assert!(vm.ctxs[c].tp.is_empty(), "frame {f}: {:?} left on context {c} at pc {} ({})", vm.ctxs[c].tp, vm.ctxs[c].pc, kh2_bdx::disasm::decode(&vm.prog, vm.ctxs[c].pc.saturating_sub(1)).0);
            }
        }
        assert!(release.is_some());
        w
    }

    pub(super) fn fly_in(script: &[u8], target: Option<[f32; 3]>, hit_at: Option<usize>) -> World {
        cast_in(script, 0, target, hit_at)
    }

    fn fly(target: Option<[f32; 3]>, hit_at: Option<usize>) -> World {
        fly_in(&fire_script(), target, hit_at)
    }

    #[test]
    fn kh1_fire_flies_as_kh1s() {
        // Straight on (no target): from the Keyblade's middle, 40 a frame from its first frame,
        // 30 frames, then gone.
        let w = fly(None, None);
        let p = w.positions();
        assert_eq!(p.len(), 30, "{p:?}");
        assert_eq!(p[0], [0.0, -100.0, 15.0 + 40.0]);
        for k in 1..p.len() {
            assert!((p[k][2] - p[k - 1][2] - 40.0).abs() < 1e-3, "{k}: {p:?}");
        }
        // The fade starts on the 12th frame (life 19 < 20, KH1's test before the move).
        assert_eq!(w.faded_at, Some(28 + 11), "fades once under 20 frames of life");
        assert_eq!(w.freed, 1);
        // KH1's flash where the ball starts.
        assert!(w.started.contains(&(POINT, [0.0, -100.0, 15.0])), "{:?}", w.started);
        // A hit ends it at once.
        let w = fly(None, Some(35));
        assert_eq!(w.positions().len(), 35 - 28);
        // Homing: off to the side, it turns at most 3° a frame.
        let w = fly(Some([400.0, -85.0, 0.0]), None);
        let p = w.positions();
        assert_eq!(p.len(), 30);
        assert!(p[29][0] > 50.0, "it turned toward the target: {:?}", p[29]);
        for k in 2..p.len() {
            let (a, b) = ([p[k - 1][0] - p[k - 2][0], p[k - 1][2] - p[k - 2][2]], [p[k][0] - p[k - 1][0], p[k][2] - p[k - 1][2]]);
            let turn = (a[0] * b[0] + a[1] * b[1]) / ((a[0].hypot(a[1])) * (b[0].hypot(b[1])));
            assert!(turn.clamp(-1.0, 1.0).acos() <= fire::TURN + 1e-4, "{k}");
        }
    }

    #[test]
    fn kh1_fire_with_no_target_picks_as_kh1s() {
        // Cast with no target: each frame KH1's best within 500 of the ball, d × (1 + 2(1 − cos)).
        // 1 ahead and to the right at ~300, 2 just behind the start at 150 (nearer, but behind:
        // score × 5), 3 far ahead out of reach. The ball turns right, toward 1.
        let mut w = World::new(None, None);
        w.enemies = vec![(1, [250.0, -100.0, 200.0], true), (2, [0.0, -100.0, -135.0], true), (3, [0.0, -100.0, 1500.0], true)];
        let w = cast_with(&fire_script(), 0, w);
        let p = w.positions();
        assert!(p[10][0] > 5.0, "turned toward 1: {:?}", p[10]);
        // Nothing within 500: straight.
        let mut w = World::new(None, None);
        w.enemies = vec![(3, [600.0, -100.0, 0.0], true)];
        let w = cast_with(&fire_script(), 0, w);
        let p = w.positions();
        assert!(p[0][0].abs() < 1e-3, "{:?}", p[0]);
    }

    #[test]
    fn kh1_blizzard_shards_as_kh1s() {
        for level in 0..3 {
            let w = cast_in(&script(Kind::Blizzard, level), 2 * level as i32, None, None);
            // Seven shards, one attack copied six times: one group.
            assert_eq!(w.attacks.len(), 7);
            assert_eq!(w.groups.values().collect::<std::collections::HashSet<_>>().len(), 1);
            assert!(w.radius.values().all(|r| *r == blizzard::RADIUS[level]));
            let speed = blizzard::SPEED[level];
            let mut tracks: Vec<_> = w.attacks.values().cloned().collect();
            tracks.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
            for t in &tracks {
                // 26 frames each, at the level's speed in x and z, 50 over the floor once it finds one.
                assert_eq!(t.len(), 26, "{t:?}");
                for k in 1..t.len() {
                    let d = (t[k][0] - t[k - 1][0]).hypot(t[k][2] - t[k - 1][2]);
                    // KH1 does not normalise the cone vector (0.383² + 0.924² = 1.0005).
                    assert!(d <= speed * 1.001, "{d}");
                }
                assert!((t[25][1] + blizzard::HEIGHT).abs() < 1e-3 || t[25][1] < -blizzard::HEIGHT, "{:?}", t[25]);
            }
            // The straight one from the tip, ahead.
            let straight = tracks.iter().find(|t| t[0][0].abs() < 1e-4 && t[1][0].abs() < 1e-4).expect("straight shard");
            assert!((straight[0][2] - (30.0 + speed)).abs() < 1e-3, "{:?}", straight[0]);
            // Shards 1 / 2 / 4 / 5 play KH1's effect 5, the rest 1; the flash at the tip.
            let ids: Vec<i32> = w.fx_ids.values().copied().collect();
            assert_eq!(ids.iter().filter(|i| **i == SHOT).count(), 3);
            assert_eq!(ids.iter().filter(|i| **i == SHOT_B).count(), 4);
            assert!(w.started.contains(&(POINT, [0.0, -100.0, 30.0])), "{:?}", w.started);
            assert_eq!((w.loop_ended, w.freed), (7, 7));
        }
        // A hit ends every shard on the frame it lands.
        let w = cast_in(&script(Kind::Blizzard, 0), 0, None, Some(31));
        assert!(w.attacks.values().all(|t| t.len() == 31 - 28));
    }

    #[test]
    fn kh1_thunder_strikes_as_kh1s() {
        // Enemies round the target at (0, 0, 400): four inside the cylinder at various distances,
        // one too far out, one too high, one dead.
        let mut w = World::new(Some([0.0, 0.0, 400.0]), None);
        w.enemies = vec![
            (11, [300.0, 0.0, 400.0], true),
            (12, [0.0, 0.0, 400.0], true),
            (13, [0.0, 0.0, 1100.0], true),
            (14, [0.0, -700.0, 400.0], true),
            (15, [-100.0, 0.0, 400.0], true),
            (16, [50.0, 0.0, 400.0], false),
            (17, [0.0, 0.0, 100.0], true),
        ];
        w.motion_end = 68;
        let w = cast_with(&script(Kind::Thunder, 0), 4, w);
        // Nearest first from the target, every 8 frames from the release; out of the area passed.
        assert_eq!(w.strikes, vec![(28, 12), (36, 15), (44, 11), (52, 17)], "{:?}", w.strikes);
        // The tint: made at the cast, down over 20; back over 20, 40 frames after the release.
        assert_eq!(w.light, vec![(0, "create", 1.0), (0, "colour", 20.0), (28 + 40, "fadeout", 20.0)]);
        assert!(w.started.contains(&(POINT, [0.0, -100.0, 30.0])), "{:?}", w.started);
        assert_eq!(w.started.iter().filter(|s| s.0 == BOLT).count(), 4);
        // The marker on the floor under the target (y 0 here).
        assert!(w.started.iter().any(|s| s.0 == MARK && s.1 == [0.0, 0.0, 400.0]), "{:?}", w.started);
        // No target: the area round Sora, the marker 2 m ahead along the camera.
        let mut w = World::new(None, None);
        w.enemies = vec![(21, [0.0, 0.0, 100.0], true)];
        let w = cast_with(&script(Kind::Thunder, 0), 0, w);
        assert_eq!(w.strikes, vec![(28, 21)]);
        assert!(w.started.iter().any(|s| s.0 == MARK && (s.1[2] - 200.0).abs() < 1e-3 && s.1[0].abs() < 1e-3), "{:?}", w.started);
        // A cast cut off before the release: the tint back at once, no strikes.
        let mut w = World::new(None, None);
        w.motion_end = 10;
        let w = cast_with(&script(Kind::Thunder, 0), 0, w);
        assert_eq!(w.light[2].0..w.light[2].0 + 1, 10..11);
    }

    #[test]
    fn kh1_gravity_falls_and_crushes_as_kh1s() {
        for level in 0..3 {
            let (s, n) = (gravity::SCALE[level], gravity::FRAMES[level] as usize);
            let r = gravity::RADIUS * s;
            let mut w = World::new(Some([0.0, -80.0, 500.0]), None);
            // Two parts of one enemy at the centre, one just inside, one just outside, one dead.
            w.enemies = vec![(1, [0.0, -90.0, 500.0], true), (1, [0.0, -80.0, 500.0], true), (2, [r - 5.0, -90.0, 500.0], true), (3, [r + 5.0, -90.0, 500.0], true), (4, [0.0, -90.0, 500.0], false)];
            let w = cast_with(&script(Kind::Gravity, level), 2 * level as i32, w);
            // The marker 240 above the target from the cast.
            assert!(w.started.iter().any(|(id, p)| *id == MARK && (p[1] - (-80.0 - 240.0)).abs() < 0.01), "{:?}", w.started);
            // The sphere starts 240 above at the release (28) and lands 10 above after N frames.
            let sphere = w.started.iter().find(|(id, _)| *id == SHOT).unwrap().1;
            assert!((sphere[1] - (-320.0)).abs() < 0.01 && (sphere[2] - 500.0).abs() < 0.01, "{sphere:?}");
            // Struck once each, at the impact: 1 and 2, not 3 (out of reach) or 4 (dead).
            let struck: Vec<i32> = w.strikes.iter().map(|x| x.1).collect();
            assert_eq!(struck, vec![1, 2], "level {level}");
            let at = w.strikes[0].0;
            assert_eq!(at, 28 + n, "impact frame, level {level}");
            // The scale reaches S at the impact, then × 1.02 for 20 frames.
            // KH1's `L0055c` each fall frame (N of them, 29..28 + N; not the impact run): the
            // enemies touching the sphere of 170 × its scale that frame, each for one frame.
            let marked = |f: usize| -> Vec<i32> { w.obj_stops.iter().filter(|x| x.0 == f).map(|x| x.1).collect() };
            assert!(w.obj_stops.iter().all(|x| x.2 == gravity::SQUASH_KIND && x.3 == 1.0));
            assert!(w.obj_stops.iter().all(|x| (29..=28 + n).contains(&x.0)), "{:?}", w.obj_stops.iter().map(|x| x.0).collect::<Vec<_>>());
            // Near the end of the fall the sphere is big enough for the one just inside; at the
            // start (scale 0.01 → 1.7 cm reach, centre 230 up) for none.
            assert!(marked(29).is_empty());
            let last = marked(28 + n);
            assert!(last.contains(&1) && last.contains(&2) && !last.contains(&3) && !last.contains(&4), "{last:?}");
            let s_at = w.scales.iter().filter(|x| x.0 == at).map(|x| x.2).fold(0.0f32, f32::max);
            assert!((s_at - s).abs() < 1e-3, "{s_at} {s}");
            let last = w.scales.iter().map(|x| x.2).fold(0.0f32, f32::max);
            assert!((last - s * 1.02f32.powi(20)).abs() < 1e-3, "{last}");
        }
        // Without a target: 200 ahead of Sora (+z) at the release.
        let w = cast_with(&script(Kind::Gravity, 0), 0, World::new(None, None));
        let sphere = w.started.iter().find(|(id, _)| *id == SHOT).unwrap().1;
        assert!((sphere[2] - 200.0).abs() < 0.01 && (sphere[1] + 240.0).abs() < 0.01, "{sphere:?}");
    }

    #[test]
    fn kh1_stop_stops_as_kh1s() {
        for level in 0..3 {
            let r = stop::RADIUS[level];
            let mut w = World::new(Some([0.0, -80.0, 400.0]), None);
            w.motion_end = 60;
            // Nearest first from the target: 1 at it, 2 just in, 3 out of reach, 4 too high, 5 dead.
            w.enemies = vec![(1, [0.0, -90.0, 400.0], true), (2, [r - 10.0, -90.0, 400.0], true), (3, [r + 10.0, -90.0, 400.0], true), (4, [0.0, -500.0, 400.0], true), (5, [5.0, -90.0, 400.0], false)];
            let w = cast_with(&script(Kind::Stop, level), 2 * level as i32, w);
            assert_eq!(w.strikes, vec![(28, 1), (32, 2)], "level {level}");
            // KH1's look on each struck one, ended when its stop ends (30 frames here).
            assert_eq!(w.started.len(), 0);
            assert_eq!(w.loop_ended, 2);
        }
        // A cast cut before 28: the charge fades, nothing struck.
        let mut w = World::new(Some([0.0, -80.0, 400.0]), None);
        w.motion_end = 20;
        w.enemies = vec![(1, [0.0, -90.0, 400.0], true)];
        let w = cast_with(&script(Kind::Stop, 0), 0, w);
        assert!(w.strikes.is_empty() && w.faded_at == Some(20));
    }

    #[test]
    fn kh1_aero_barrier_and_wind_as_kh1s() {
        for level in 0..3 {
            let mut w = World::new(None, None);
            w.motion_end = 100;
            let w = cast_with(&script(Kind::Aero, level), 2 * level as i32, w);
            // Cast at 30 (the cast motion still playing): the barrier strike on Sora.
            assert_eq!(w.strikes.len(), 1);
            assert_eq!(w.strikes[0].0, 30);
            assert!(w.made.contains(&(30, aero::ATKP)));
            // The look 24 frames after the cast.
            assert!(w.bound.contains(&(30 + 24, SHOT)), "{:?}", w.bound);
            // The wind from Aerora: a fresh attack every 30 frames from the cast, radius 80.
            let winds: Vec<usize> = w.made.iter().filter(|m| m.1 == aero::WIND_ATKP).map(|m| m.0).collect();
            if level == 0 {
                assert!(winds.is_empty());
            } else {
                assert_eq!(winds, vec![30, 60, 90], "level {level}");
                assert!(w.radius.values().all(|&r| r == aero::WIND_RADIUS));
            }
        }
        // The cast motion cut at 12: cast at once (KH1 never drops Aero).
        let mut w = World::new(None, None);
        w.motion_end = 12;
        let w = cast_with(&script(Kind::Aero, 0), 0, w);
        assert_eq!(w.strikes[0].0, 12);
    }

    #[test]
    fn kh1_cure_heals_sora_as_kh1s() {
        let mut w = World::new(None, None);
        w.motion_end = 68;
        let w = cast_with(&script(Kind::Cure, 0), 0, w);
        // One heal, on Sora (his record's object), at the release; the effect over his body centre.
        assert_eq!(w.strikes.len(), 1);
        assert_eq!(w.strikes[0].0, 28);
        assert!(w.started.iter().any(|s| s.0 == SHOT && s.1 == [0.0, -90.0, 0.0]), "{:?}", w.started);
    }

    #[test]
    fn blizzard_cone_is_kh1s() {
        let s = blizzard::shards();
        for (off, dir, _) in &s[..6] {
            assert!((dir[2] - 0.924).abs() < 1e-6);
            assert!(((off[0] * off[0] + off[1] * off[1]).sqrt() - 19.15).abs() < 1e-3);
            assert!((dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2] - 1.0).abs() < 1e-3);
        }
        assert_eq!(s[6], ([0.0; 3], [0.0, 0.0, 1.0], SHOT));
    }

    #[test]
    fn script_avoids_the_games_30_fps_patches() {
        // KH2 replaces some float pushes at fixed code positions (`kh2_bdx::vm::pc_patch`); none of
        // ours may sit there.
        for kind in [Kind::Fire, Kind::Blizzard, Kind::Thunder, Kind::Cure, Kind::Gravity, Kind::Stop, Kind::Aero] {
            for level in 0..3 {
                let p = kh2_bdx::Program::parse(&script(kind, level)).unwrap();
                let mut pc = (12 + 8 * (p.triggers.len() as u32 + 1)) / 2;
                while pc < p.words() {
                    let (text, size) = kh2_bdx::disasm::decode(&p, pc);
                    if let Some(v) = text.strip_prefix("push.s ").map(|v| v.trim().parse::<f32>().unwrap()) {
                        assert_eq!(kh2_bdx::vm::pc_patch(&p.name, pc * 2, v), None, "{kind:?} pc {pc}");
                    }
                    pc += size;
                }
            }
        }
    }

    #[test]
    fn pax_and_mag_read_back() {
        let pax_bytes = pax(&elements(Kind::Fire, 8), &[0x82, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let p = kh2_formats::pax::Pax::parse(&pax_bytes).unwrap();
        assert_eq!(p.elements.len(), 6);
        assert_eq!((p.elements[0].bone, p.elements[3].tag, p.elements[1].effect), (0x4003, 0x4_0000_0001, 9));
        let m = mag(b"fire", &pax_bytes, &fire_script());
        let bar = kh2_formats::bar::Bar::parse(&m).unwrap();
        assert_eq!(bar.entries.iter().map(|e| (e.kind, e.duplicate)).collect::<Vec<_>>(), vec![(18, 0), (3, 0), (3, 1)]);
    }
}

#[cfg(test)]
mod files {

    /// The owner's KH1 spell files (staged at `KH1_MAGS`, a folder of `xm_*.mag`) and KH2's
    /// (`KH2_MAGS`, `FIRE_1.mag` …): each KH1 set into KH2's file, KH2's elements, effects and
    /// code untouched and first, KH1's after; base Sora's cast flies as our script alone.
    #[test]
    fn kh1_spells_onto_kh2s_files() {
        let (Ok(k1), Ok(k2)) = (std::env::var("KH1_MAGS"), std::env::var("KH2_MAGS")) else { return };
        for (kh1_name, kh2_name, kind, level) in super::SPELLS {
            let (Ok(kh1), Ok(kh2)) = (std::fs::read(format!("{k1}/{kh1_name}")), std::fs::read(format!("{k2}/{}", super::kh2_base(kh2_name).trim_start_matches("magic/")))) else {
                eprintln!("{kh1_name} / {kh2_name}: not staged");
                continue;
            };
            let set = super::kh1_effect_set(&kh1).unwrap();
            let dpd_at = u32::from_le_bytes(set[16..20].try_into().unwrap()) as usize;
            let d = kh2_formats::dpd::Dpd::parse(&set[dpd_at..]).unwrap();
            assert!(d.particles.iter().all(|x| x.se == 0));
            let m = super::kh1_mag(kind, level, &kh2, &kh1).unwrap();
            let bar = kh2_formats::bar::Bar::parse(&m).unwrap();
            let old = kh2_formats::bar::Bar::parse(&kh2).unwrap();
            let p = kh2_formats::pax::Pax::parse(bar.entries[0].data).unwrap();
            let q = kh2_formats::pax::Pax::parse(old.entries[0].data).unwrap();
            assert_eq!(p.elements.len(), q.elements.len() + super::elements(kind, 0).len());
            for (a, b) in p.elements.iter().zip(&q.elements) {
                assert_eq!((a.effect, a.id, a.tag, a.bone), (b.effect, b.id, b.tag, b.bone));
            }
            let numbers: Vec<u32> = p.effects.iter().map(|e| e.number).collect();
            for e in &p.elements[q.elements.len()..] {
                assert!(numbers.contains(&(e.effect as u32)), "{kh2_name}: element {} effect {}", e.id, e.effect);
            }
            for e in &q.effects {
                assert_eq!(p.effect(e.number as u16).unwrap().emitters.len(), e.emitters.len());
            }
            let pr = kh2_bdx::Program::parse(bar.entries[1].data).unwrap();
            let po = kh2_bdx::Program::parse(old.entries[1].data).unwrap();
            assert_eq!(pr.triggers.iter().map(|t| t.0).collect::<Vec<_>>(), po.triggers.iter().map(|t| t.0).collect::<Vec<_>>());
            // KH2's code word for word after its header (sizes and trigger table are ours).
            let code_at = 12 + 8 * (po.triggers.len() + 1) + 16;
            assert_eq!(&bar.entries[1].data[code_at..old.entries[1].data.len()], &old.entries[1].data[code_at..]);
            // Base Sora's cast runs ours in the merged script, as in our own.
            let ours = super::tests::cast_in(&super::script(kind, level), 2 * level as i32, Some([400.0, -85.0, 0.0]), None);
            let merged = super::tests::cast_in(bar.entries[1].data, 2 * level as i32, Some([400.0, -85.0, 0.0]), None);
            let sorted = |w: &super::tests::World| {
                let mut v: Vec<_> = w.attacks.values().cloned().collect();
                v.sort_by(|a, b| a.partial_cmp(b).unwrap());
                v
            };
            assert_eq!(sorted(&merged), sorted(&ours));
            assert_eq!((merged.faded_at, merged.freed), (ours.faded_at, ours.freed));
            // None of our float pushes sits where the game rewrites one for 60 fps.
            let mut pc = po.words();
            while pc < pr.words() {
                let (text, size) = kh2_bdx::disasm::decode(&pr, pc);
                if let Some(v) = text.strip_prefix("push.s ").map(|v| v.trim().parse::<f32>().unwrap()) {
                    assert_eq!(kh2_bdx::vm::pc_patch(&pr.name, pc * 2, v), None, "{kh2_name} pc {pc}");
                }
                pc += size;
            }
            eprintln!("{kh2_name}: merged {} bytes (KH2's {}), {} particles, {} textures of KH1's", m.len(), kh2.len(), d.particles.len(), d.textures.len());
        }
    }
}
