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

/// KH2's own spell file (`kh2`, `magic/FIRE_n.mag`) with KH1's Fire added (`kh1`, the matching
/// `xm_fire0n.mag`): KH2's script, elements and effects stay for the Drive Forms.
pub fn fire_mag(kh2: &[u8], kh1: &[u8]) -> Option<Vec<u8>> {
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
    let (pax, _) = merge_pax(pax_e.data, &set, fire_elements)?;
    Some(mag(&name, &pax, &fire_onto(bdx_e.data)?))
}

/// KH1's Fire (`xm_fire01.mag`'s script, KH1-in-ReCom's decoding; Checked (files) there).
pub mod fire {
    /// The ball's speed (cm a 60 Hz frame), life (frames), turn toward the target (rad a frame)
    /// and radius (attack data `+0x5C`).
    pub const SPEED: f32 = 40.0;
    pub const LIFE: f32 = 30.0;
    pub const TURN: f32 = 3.0 * std::f32::consts::PI / 180.0;
    pub const RADIUS: f32 = 50.0;
    /// The ball starts fading this many frames before its end.
    pub const FADE: f32 = 20.0;
    /// KH2's Fire attack (ATKP 502, its rows by the sub; Checked (files): `FIRE_1.mag`'s script).
    pub const ATKP: i32 = 502;
    /// Script ids of our elements (after KH2's own, which stay for the Drive Forms): the cast on
    /// the Keyblade, the ball.
    pub const CAST: i32 = 100;
    pub const BALL: i32 = 101;
    /// The hit-effect group our attack rows get (`effect_on_hit`, patched by `kh2-hook` on base
    /// Sora's Fire rows: ATKP 502, subs 0..5), so KH2's lookups find our elements, not KH2's own.
    pub const HIT_GROUP: u8 = 3;
}

/// The elements of KH1's Fire: KH1's effect 0 on the Keyblade (KH1 starts it on the casting bone
/// at the cast), 1 the ball, 3 / 2 its hit on a body / a wall or attack (the tags KH2's Fire and
/// Blizzard give theirs: groups 1 and 2 hits, the wall, attack on attack). KH1's sounds aren't in
/// KH2's banks yet: KH2's Fire's own (116 the cast, 118 / 119 the hits) stand in (Assumption on
/// what they are: KH2's Fire PAX).
/// `kh1` is what KH1's effect numbers are moved up by in the merged set (`merge_pax`).
pub fn fire_elements(kh1: u16) -> Vec<Element> {
    let g = 1u64 << (32 + fire::HIT_GROUP as u64 - 1);
    vec![
        Element { id: fire::CAST as u16, effect: kh1, bone: 0x4003, flags: 0x2A1, tag: 0, sound: 116, scale: 1.0 },
        Element { id: fire::BALL as u16, effect: kh1 + 1, bone: 0, flags: 0, tag: 0, sound: -1, scale: 1.0 },
        Element { id: 103, effect: kh1 + 3, bone: 0, flags: 0, tag: g | 1, sound: 118, scale: 1.0 },
        Element { id: 105, effect: kh1 + 2, bone: 0, flags: 0, tag: g | 0x102, sound: -1, scale: 1.0 },
        Element { id: 106, effect: kh1 + 2, bone: 0, flags: 0, tag: g | 0x10, sound: -1, scale: 1.0 },
    ]
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

// Work memory (`wp`): the caster's record (16 bytes, as the game hands it), the resources, the
// sub, the `magi` record, the cast effect, the cast motion.
const W_CASTER: u16 = 0;
const W_RES: u16 = 16;
const W_SUB: u16 = 20;
const W_MAGI: u16 = 24;
const W_CAST_FX: u16 = 28;
const W_MOTION: u16 = 32;
const WORK: u32 = 48;

// The ball's block (`memory_alloc`): a copy of the caster's record, its place, target, effect,
// attack and direction.
const B_CASTER: u16 = 0;
const B_POS: u16 = 32;
const B_TARGET: u16 = 48;
const B_THREAD: u16 = 52;
const B_FX: u16 = 64;
const B_ATK: u16 = 68;
const B_DIR: u16 = 80;
const BLOCK: i32 = 112;

/// KH1's Fire as a KH2 spell script of its own (the tests'; the build uses `fire_onto`).
pub fn fire_script() -> Vec<u8> {
    let mut a = Asm::new();
    fire_code(&mut a);
    a.finish("kh1fire", WORK, 0x400, 0x200, &[(21, "release"), (20, "cast")])
}

/// KH1's Fire onto KH2's own `fire` script (`kh2`, its BDX file): KH2's code stays where it was and
/// still runs for the Drive Forms (sub ≥ 10); base Sora's casts run ours. The trigger table points
/// at two stubs of ours that choose, and keep the choice in a work word past both scripts' own.
pub fn fire_onto(kh2: &[u8]) -> Option<Vec<u8>> {
    use Mode::*;
    let p = kh2_bdx::Program::parse(kh2).ok()?;
    let (k20, k21) = (p.trigger(20)?, p.trigger(21)?);
    let ours = ((p.work_size.max(WORK as u32) + 3) & !3) as u16;
    let mut a = Asm::new();
    a.label("t20").pop(Sp, 0).pop(Sp, 4).pop(Sp, 8).pop(Sp, 12);
    a.push_v(Sp, 4).push_i(10).bin("sub").un("sltz").beqz("t20_kh2");
    a.push_i(1).pop(Wp, ours).push_v(Sp, 12).push_v(Sp, 8).push_v(Sp, 4).push_v(Sp, 0).b("cast");
    a.label("t20_kh2").push_i(0).pop(Wp, ours).push_v(Sp, 12).push_v(Sp, 8).push_v(Sp, 4).push_v(Sp, 0).b("kh2_20");
    a.label("t21").push_v(Wp, ours).bnez("release").b("kh2_21");
    fire_code(&mut a);
    // The table in KH2's own order (21 then 20, Checked (files): every FIRE_n.mag).
    let order: Vec<(i32, &str)> = p.triggers.iter().map(|(k, _)| (*k, if *k == 20 { "t20" } else { "t21" })).collect();
    a.finish_onto(kh2, ours as u32 + 16, 0x400, 0x200, &order, &[("kh2_20", k20), ("kh2_21", k21)]).ok()
}

fn fire_code(a: &mut Asm) {
    use Mode::*;
    // Trigger 20, the cast: the game pushes the caster, the resources, the sub and `magi`.
    a.label("cast").pop(Sp, 0).pop(Sp, 4).pop(Sp, 8).pop(Sp, 12);
    a.push_v(Sp, 12).memcpy(Wp, 16, W_CASTER);
    a.push_v(Sp, 8).pop(Wp, W_RES);
    a.push_v(Sp, 4).pop(Wp, W_SUB);
    a.push_v(Sp, 0).pop(Wp, W_MAGI);
    // KH1's cast effect on the Keyblade.
    a.push_v(Wp, W_RES).push_i(fire::CAST).push_a(Wp, W_CASTER).push_i(1).push_i(0).sys("trap_pax_start_bind").pop(Wp, W_CAST_FX);
    // Hold while the cast motion plays (KH2's own spells do: a cast cut short does nothing more).
    a.push_a(Wp, W_CASTER).push_dpop(4).sys("trap_sysobj_motion_id").pop(Wp, W_MOTION);
    a.label("cast_wait").push_a(Wp, W_CASTER).push_dpop(4).sys("trap_sysobj_motion_id").push_v(Wp, W_MOTION).bin("sub").un("seqz").beqz("cast_end").halt().b("cast_wait");
    a.label("cast_end").ret();

    // Trigger 21, the cast event (KH1's release): the ball's block and thread.
    a.label("release").push_i(BLOCK).sys("trap_memory_alloc").pop(Sp, 0);
    a.push_a(Wp, W_CASTER).memcpy(SpD, 16, B_CASTER);
    // Its target: the caster's battle target, held.
    a.push_v(Sp, 0).push_i(B_TARGET as i32).bin("add").push_v(Sp, 0).sys("trap_btlobj_target").sys("trap_target_dup").store();
    // From the Keyblade (the cast effect's place), else the caster's body.
    a.push_v(Wp, W_CAST_FX).beqz("from_body");
    a.push_v(Wp, W_CAST_FX).sys("trap_effect_pos").memcpy(SpD, 16, B_POS).b("aim");
    a.label("from_body").push_v(Sp, 0).sys("trap_obj_target_pos").memcpy(SpD, 16, B_POS);
    // Along his facing (KH1 faces the target at the cast).
    a.label("aim").push_v(Sp, 0).sys("trap_obj_dir").memcpy(SpD, 16, B_DIR);
    a.push_v(Sp, 0).push_i(B_THREAD as i32).bin("add").push_pc("ball").push_v(Sp, 0).sys("trap_magic_start_thread").store();
    a.ret();

    // The ball's thread: one argument, the block.
    a.label("ball").pop(Sp, 0);
    a.push_v(Wp, W_RES).push_i(fire::BALL).push_i(1).push_i(0).sys("trap_pax_start").pop(SpD, B_FX);
    a.push_a(Wp, W_CASTER).push_i(fire::ATKP).push_i(-1).push_v(Wp, W_SUB).sys("trap_attack_new").pop(SpD, B_ATK);
    a.push_v(SpD, B_ATK).push_v(Wp, W_RES).sys("trap_attack_set_pax");
    a.push_v(SpD, B_ATK).push_f(fire::RADIUS).push_f(fire::RADIUS).push_f(0.0).sys("trap_attack_set_radius");
    // sp+4 the life left, sp+8 fading.
    a.push_f(fire::LIFE).pop(Sp, 4).push_i(0).pop(Sp, 8);
    a.label("fly");
    a.push_v(SpD, B_FX).sys("trap_effect_is_alive").beqz("ball_end");
    a.push_v(SpD, B_ATK).sys("trap_attack_is_hit_bg").bnez("ball_end");
    a.push_v(SpD, B_ATK).sys("trap_attack_is_reflect").bnez("ball_end");
    a.push_v(SpD, B_ATK).sys("trap_attack_is_hit").bnez("ball_end");
    a.push_v(SpD, B_FX).push_a(SpD, B_DIR).sys("trap_effect_set_dir");
    a.push_v(SpD, B_FX).push_a(SpD, B_POS).sys("trap_effect_set_pos");
    a.push_v(SpD, B_ATK).push_a(SpD, B_POS).sys("trap_attack_set_pos");
    a.halt();
    // KH1's turn toward a live target, at most 3° a frame.
    a.push_v(SpD, B_TARGET).sys("trap_target_is_exist").beqz("move");
    a.push_v(SpD, B_TARGET).sys("trap_target_pos").memcpy(Sp, 16, 16);
    a.push_a(Sp, 16).push_a(SpD, B_POS).sys("trap_vector_sub").memcpy(Sp, 16, 32);
    a.push_a(SpD, B_DIR).push_a(Sp, 32).push_f(fire::TURN).sys("trap_vector_homing");
    a.label("move").push_a(SpD, B_POS).push_a(SpD, B_DIR).push_f(fire::SPEED).sys("trap_vector_addf");
    a.push_v(Sp, 4).sys("trap_frametime").bin("sub.s").pop(Sp, 4);
    // KH1's fade over its last 20 frames.
    a.push_v(Sp, 8).bnez("life");
    a.push_v(Sp, 4).push_f(fire::FADE).bin("sub.s").un("slez.s").beqz("life");
    a.push_v(SpD, B_FX).push_f(fire::FADE).push_i(1).sys("trap_effect_fadeout").push_i(1).pop(Sp, 8);
    a.label("life").push_v(Sp, 4).un("slez.s").bnez("ball_end").b("fly");
    // Gone: the ball's effect, its attack, the target, the block.
    a.label("ball_end").push_v(SpD, B_FX).sys("trap_effect_kill");
    a.push_v(SpD, B_ATK).sys("trap_attack_free");
    a.push_v(SpD, B_TARGET).sys("trap_target_free");
    a.push_v(Sp, 0).sys("trap_memory_free");
    a.ret();
}

#[cfg(test)]
mod tests {
    use super::*;
    use kh2_bdx::{BdxError, Cell, CtxId, Host, Vm};

    /// Just enough of KH2's traps (as the sister's `kh2-magic` documents them) to fly the ball.
    pub(super) struct World {
        target: [f32; 3],
        target_alive: bool,
        hit_at: Option<usize>,
        frame: usize,
        pub(super) positions: Vec<[f32; 3]>,
        fx_alive: bool,
        pub(super) faded_at: Option<usize>,
        pub(super) freed: bool,
        threads: Vec<CtxId>,
        slot: u32,
    }

    impl World {
        fn ret_vec(&mut self, vm: &mut Vm, v: [f32; 4]) -> Cell {
            if self.slot == 0 {
                self.slot = vm.mem.alloc(16);
            }
            vm.mem.set_vec4(self.slot, v).unwrap();
            Cell::addr(self.slot)
        }
    }

    impl Host for World {
        fn syscall(&mut self, vm: &mut Vm, ctx: CtxId, table: u8, index: u16, args: &mut [Cell]) -> Result<(), BdxError> {
            let name = kh2_bdx::trap_name(table, index).unwrap_or("?");
            let v = |vm: &Vm, a: Cell| vm.mem.vec4(a.v).unwrap();
            match name {
                "trap_memory_alloc" => args[0] = Cell::addr(vm.mem.alloc(args[0].i() as u32)),
                "trap_memory_free" => self.freed = true,
                "trap_btlobj_target" | "trap_target_dup" => args[0] = Cell::int(7),
                "trap_target_free" | "trap_attack_set_pax" | "trap_attack_set_radius" | "trap_effect_set_dir" | "trap_effect_set_pos" => {}
                "trap_pax_start_bind" | "trap_pax_start" => args[0] = Cell::int(5),
                "trap_attack_new" => {
                    assert_eq!(args[1].i(), fire::ATKP);
                    args[0] = Cell::int(9)
                }
                "trap_sysobj_motion_id" => args[0] = Cell::int(if self.frame < 28 { 56 } else { 0 }),
                "trap_effect_pos" => args[0] = self.ret_vec(vm, [0.0, -100.0, 30.0, 1.0]),
                "trap_obj_target_pos" => args[0] = self.ret_vec(vm, [0.0, -85.0, 0.0, 1.0]),
                "trap_obj_dir" => args[0] = self.ret_vec(vm, [0.0, 0.0, 1.0, 0.0]),
                "trap_attack_set_pos" => {
                    let p = v(vm, args[1]);
                    self.positions.push([p[0], p[1], p[2]]);
                }
                "trap_effect_is_alive" => args[0] = Cell::int(self.fx_alive as i32),
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
                "trap_effect_kill" => self.fx_alive = false,
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

    fn fly(target: Option<[f32; 3]>, hit_at: Option<usize>) -> World {
        fly_in(&fire_script(), target, hit_at)
    }

    pub(super) fn fly_in(script: &[u8], target: Option<[f32; 3]>, hit_at: Option<usize>) -> World {
        let prog = kh2_bdx::Program::parse(script).unwrap();
        let mut vm = Vm::new(prog);
        let mut w = World { target: target.unwrap_or_default(), target_alive: target.is_some(), hit_at, frame: 0, positions: vec![], fx_alive: true, faded_at: None, freed: false, threads: vec![], slot: 0 };
        let caster = vm.mem.alloc(16);
        let cast = vm.spawn(None);
        vm.trigger(cast, 20, &[Cell::addr(caster), Cell::int(3), Cell::int(0), Cell::addr(0)]).unwrap();
        let mut release = None;
        for f in 0..120 {
            w.frame = f;
            if !vm.ctxs[cast].done {
                if vm.run(cast, &mut w).unwrap() != kh2_bdx::Status::Yield {
                    vm.ctxs[cast].done = true;
                }
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
        }
        assert!(release.is_some());
        w
    }

    #[test]
    fn kh1_fire_flies_as_kh1s() {
        // Straight on (no target): from the Keyblade, 40 a frame, 30 frames, then gone.
        let w = fly(None, None);
        let p = &w.positions;
        assert_eq!(p.len(), 30, "{p:?}");
        assert_eq!(p[0], [0.0, -100.0, 30.0]);
        for k in 1..p.len() {
            assert!((p[k][2] - p[k - 1][2] - 40.0).abs() < 1e-3, "{k}: {p:?}");
        }
        assert_eq!(w.faded_at, Some(28 + 10), "fades 20 frames before its end");
        assert!(w.freed && !w.fx_alive);
        // A hit ends it at once.
        let w = fly(None, Some(35));
        assert_eq!(w.positions.len(), 35 - 28);
        // Homing: off to the side, it turns at most 3° a frame.
        let w = fly(Some([400.0, -85.0, 0.0]), None);
        let p = &w.positions;
        assert_eq!(p.len(), 30);
        assert!(p[29][0] > 50.0, "it turned toward the target: {:?}", p[29]);
        for k in 2..p.len() {
            let (a, b) = ([p[k - 1][0] - p[k - 2][0], p[k - 1][2] - p[k - 2][2]], [p[k][0] - p[k - 1][0], p[k][2] - p[k - 1][2]]);
            let turn = (a[0] * b[0] + a[1] * b[1]) / ((a[0].hypot(a[1])) * (b[0].hypot(b[1])));
            assert!(turn.clamp(-1.0, 1.0).acos() <= fire::TURN + 1e-4, "{k}");
        }
    }

    #[test]
    fn script_avoids_the_games_30_fps_patches() {
        // KH2 replaces some float pushes at fixed code positions (`kh2_bdx::vm::pc_patch`); none of
        // ours may sit there.
        let s = fire_script();
        assert!(s.len() < 0x2e8 + 0x10, "the script reaches KH2's first patched position");
    }

    #[test]
    fn pax_and_mag_read_back() {
        let pax_bytes = pax(&fire_elements(8), &[0x82, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let p = kh2_formats::pax::Pax::parse(&pax_bytes).unwrap();
        assert_eq!(p.elements.len(), 5);
        assert_eq!((p.elements[0].bone, p.elements[2].tag, p.elements[1].effect), (0x4003, 0x4_0000_0001, 9));
        let m = mag(b"fire", &pax_bytes, &fire_script());
        let bar = kh2_formats::bar::Bar::parse(&m).unwrap();
        assert_eq!(bar.entries.iter().map(|e| (e.kind, e.duplicate)).collect::<Vec<_>>(), vec![(18, 0), (3, 0), (3, 1)]);
    }
}

#[cfg(test)]
mod files {
    /// The owner's KH1 Fire (`xm_fire01.mag`, staged at `KH1_MAG`): its set goes into our PAX
    /// and reads back with KH2's DPD reader, sounds cleared.
    #[test]
    fn kh1_fire_set_in_a_kh2_pax() {
        let Ok(path) = std::env::var("KH1_MAG") else { return };
        let kh1 = std::fs::read(path).unwrap();
        let set = super::kh1_effect_set(&kh1).unwrap();
        let pax = super::pax(&super::fire_elements(0), &set);
        let p = kh2_formats::pax::Pax::parse(&pax).unwrap();
        eprintln!("effects {:?}", p.effects.iter().map(|e| (e.number, e.loops(), e.length(None))).collect::<Vec<_>>());
        assert!(p.effects.len() >= 5);
        let dpd_at = u32::from_le_bytes(set[16..20].try_into().unwrap()) as usize;
        let d = kh2_formats::dpd::Dpd::parse(&set[dpd_at..]).unwrap();
        assert!(d.particles.iter().all(|x| x.se == 0));
        eprintln!("{} particles, {} textures", d.particles.len(), d.textures.len());
        let m = super::mag(b"fire", &pax, &super::fire_script());
        eprintln!("FIRE_1.mag of ours: {} bytes", m.len());
    }

    /// KH2's own `FIRE_1.mag` (staged at `KH2_MAG`) with KH1's Fire added: KH2's elements and
    /// effects untouched and first, KH1's after; the script keeps KH2's code and both triggers.
    #[test]
    fn kh1_fire_onto_kh2s_file() {
        let (Ok(k1), Ok(k2)) = (std::env::var("KH1_MAG"), std::env::var("KH2_MAG")) else { return };
        let (kh1, kh2) = (std::fs::read(k1).unwrap(), std::fs::read(k2).unwrap());
        let m = super::fire_mag(&kh2, &kh1).unwrap();
        let bar = kh2_formats::bar::Bar::parse(&m).unwrap();
        let old = kh2_formats::bar::Bar::parse(&kh2).unwrap();
        let p = kh2_formats::pax::Pax::parse(bar.entries[0].data).unwrap();
        let q = kh2_formats::pax::Pax::parse(old.entries[0].data).unwrap();
        assert_eq!(p.elements.len(), q.elements.len() + 5);
        for (a, b) in p.elements.iter().zip(&q.elements) {
            assert_eq!((a.effect, a.id, a.tag, a.bone), (b.effect, b.id, b.tag, b.bone));
        }
        let numbers: Vec<u32> = p.effects.iter().map(|e| e.number).collect();
        eprintln!("numbers {numbers:?}");
        for e in &p.elements[q.elements.len()..] {
            assert!(numbers.contains(&(e.effect as u32)), "element {} effect {}", e.id, e.effect);
        }
        for e in &q.effects {
            let n = p.effect(e.number as u16).unwrap();
            assert_eq!(n.emitters.len(), e.emitters.len());
        }
        let pr = kh2_bdx::Program::parse(bar.entries[1].data).unwrap();
        let po = kh2_bdx::Program::parse(old.entries[1].data).unwrap();
        assert_eq!(pr.triggers.iter().map(|t| t.0).collect::<Vec<_>>(), po.triggers.iter().map(|t| t.0).collect::<Vec<_>>());
        // KH2's code word for word after its header (sizes and trigger table are ours).
        assert_eq!(&bar.entries[1].data[64..old.entries[1].data.len()], &old.entries[1].data[64..]);
        // Base Sora's cast (sub 0) runs ours in the merged script, as in our own.
        let ours = super::tests::fly_in(&super::fire_script(), Some([400.0, -85.0, 0.0]), None);
        let merged = super::tests::fly_in(bar.entries[1].data, Some([400.0, -85.0, 0.0]), None);
        assert_eq!(merged.positions, ours.positions);
        assert_eq!((merged.faded_at, merged.freed), (ours.faded_at, ours.freed));
        // None of our float pushes sits where the game rewrites one for 60 fps.
        let from = po.words();
        let mut pc = from;
        while pc < pr.words() {
            let (text, size) = kh2_bdx::disasm::decode(&pr, pc);
            if let Some(v) = text.strip_prefix("push.s ").map(|v| { let t = v.trim(); t.parse::<f32>().unwrap_or_else(|_| panic!("{pc}: {text}")) }) {
                assert_eq!(kh2_bdx::vm::pc_patch(&pr.name, pc * 2, v), None, "pc {pc}");
            }
            pc += size;
        }
        eprintln!("merged FIRE_1.mag {} bytes (KH2's {})", m.len(), kh2.len());
    }
}
