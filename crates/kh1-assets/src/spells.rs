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
pub const SPELLS: [(&str, &str, Kind, usize); 3] = [
    ("xm_fire01.mag", "magic/FIRE_1.mag", Kind::Fire, 0),
    ("xm_fire02.mag", "magic/FIRE_2.mag", Kind::Fire, 1),
    ("xm_fire03.mag", "magic/FIRE_3.mag", Kind::Fire, 2),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Fire,
    Blizzard,
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
const WORK: u32 = 112;

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
    a.push_v(Sp, 4).push_i(10).bin("sub").un("sltz").beqz("t20_kh2");
    a.push_i(1).pop(Wp, ours).push_v(Sp, 12).push_v(Sp, 8).push_v(Sp, 4).push_v(Sp, 0).b("cast");
    a.label("t20_kh2").push_i(0).pop(Wp, ours).push_v(Sp, 12).push_v(Sp, 8).push_v(Sp, 4).push_v(Sp, 0).b("kh2_20");
    a.label("t21").push_v(Wp, ours).bnez("release").b("kh2_21");
    code(kind, level, &mut a);
    // The table in KH2's own order (21 then 20, Checked (files): FIRE_n.mag, BLIZZARD_n.mag).
    let order: Vec<(i32, &str)> = p.triggers.iter().map(|(k, _)| (*k, if *k == 20 { "t20" } else { "t21" })).collect();
    a.finish_onto(kh2, ours as u32 + 16, 0x400, 0x200, &order, &[("kh2_20", k20), ("kh2_21", k21)]).ok()
}

fn code(kind: Kind, level: usize, a: &mut Asm) {
    cast_code(a);
    match kind {
        Kind::Fire => fire_code(a),
        Kind::Blizzard => blizzard_code(level.min(2), a),
    }
}

/// Trigger 20, the cast (every KH1 spell): the game pushes the caster, the resources, the sub and
/// `magi`. KH1's cast effect on the Keyblade, held while the cast motion plays (KH2's own spells
/// do: a cast cut short does nothing more).
fn cast_code(a: &mut Asm) {
    use Mode::*;
    a.label("cast").pop(Sp, 0).pop(Sp, 4).pop(Sp, 8).pop(Sp, 12);
    a.push_v(Sp, 12).memcpy(Wp, 16, W_CASTER);
    a.push_v(Sp, 8).pop(Wp, W_RES);
    a.push_v(Sp, 4).pop(Wp, W_SUB);
    a.push_v(Sp, 0).pop(Wp, W_MAGI);
    a.push_v(Wp, W_RES).push_i(CAST).push_a(Wp, W_CASTER).push_i(1).push_i(0).sys("trap_pax_start_bind").pop(Wp, W_CAST_FX);
    a.push_a(Wp, W_CASTER).push_dpop(4).sys("trap_sysobj_motion_id").pop(Wp, W_MOTION);
    a.label("cast_wait").push_a(Wp, W_CASTER).push_dpop(4).sys("trap_sysobj_motion_id").push_v(Wp, W_MOTION).bin("sub").un("seqz").beqz("cast_end").halt().b("cast_wait");
    a.label("cast_end").ret();
}

/// The release's common part: the point on the Keyblade (`bone`) into `W_POINT`, Sora's facing
/// into `W_FACING`, KH1's flash (`POINT`) there turned by `turn` from his facing (KH1: Fire 0,
/// Blizzard π).
fn release_point(a: &mut Asm, bone: i32, turn: f32) {
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
    if turn == 0.0 {
        a.push_v(Sp, 4).push_a(Wp, W_FACING).sys("trap_effect_set_dir");
    } else {
        a.push_v(Sp, 4).push_a(Wp, W_FACING).push_f(turn).sys("trap_vector_roty").sys("trap_effect_set_dir");
    }
}

/// A new shot's block into `sp+0`, with the caster's record.
fn new_block(a: &mut Asm) {
    use Mode::*;
    a.push_i(BLOCK).sys("trap_memory_alloc").pop(Sp, 0);
    a.push_a(Wp, W_CASTER).memcpy(SpD, 16, B_CASTER);
}

fn fire_code(a: &mut Asm) {
    use Mode::*;
    // Trigger 21, the cast event (KH1's release): from the Keyblade's middle, along his facing
    // (KH1 faces the target at the cast); the flash there; the ball's block and thread.
    release_point(a, MIDDLE, 0.0);
    new_block(a);
    a.push_v(Sp, 0).push_i(B_TARGET as i32).bin("add").push_v(Sp, 0).sys("trap_btlobj_target").sys("trap_target_dup").store();
    a.push_a(Wp, W_POINT).memcpy(SpD, 16, B_POS);
    a.push_a(Wp, W_FACING).memcpy(SpD, 16, B_DIR);
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
    a.push_v(SpD, B_TARGET).sys("trap_target_is_exist").beqz("fade");
    a.push_v(SpD, B_TARGET).sys("trap_target_pos").memcpy(Sp, 16, 16);
    a.push_a(Sp, 16).push_a(SpD, B_POS).sys("trap_vector_sub").memcpy(Sp, 16, 32);
    a.push_a(SpD, B_DIR).push_a(Sp, 32).push_f(fire::TURN).sys("trap_vector_homing");
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
    release_point(a, TIP, std::f32::consts::PI);
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
                "trap_sysobj_motion_id" => args[0] = Cell::int(if self.frame < 28 { 56 } else { 0 }),
                "trap_obj_apply_bone_matrix" => {
                    let z = match args[1].i() {
                        0x4003 => 30.0,
                        0x4007 => 15.0,
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
                "trap_target_is_exist" => args[0] = Cell::int(self.target_alive as i32),
                "trap_target_pos" => {
                    let t = self.target;
                    args[0] = self.ret_vec(vm, [t[0], t[1], t[2], 1.0])
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
        let prog = kh2_bdx::Program::parse(script).unwrap();
        let mut vm = Vm::new(prog);
        let mut w = World::new(target, hit_at);
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
            for t in w.threads.clone() {
                if !vm.ctxs[t].done && vm.run(t, &mut w).unwrap() != kh2_bdx::Status::Yield {
                    vm.ctxs[t].done = true;
                }
            }
            // Every trap call took what it was given: nothing left on any value stack between frames.
            for c in [cast].into_iter().chain(release).chain(w.threads.iter().copied()) {
                assert!(vm.ctxs[c].tp.is_empty(), "frame {f}: {:?} left on context {c}", vm.ctxs[c].tp);
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
        for kind in [Kind::Fire, Kind::Blizzard] {
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
            let (Ok(kh1), Ok(kh2)) = (std::fs::read(format!("{k1}/{kh1_name}")), std::fs::read(format!("{k2}/{}", kh2_name.trim_start_matches("magic/")))) else {
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
