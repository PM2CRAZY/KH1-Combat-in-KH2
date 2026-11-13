//! KH1's movement numbers (Steam KH1, `KINGDOM HEARTS FINAL MIX.exe`), plain Rust so they are
//! tested on any platform. Each is Checked (decomp + exe); `docs/movement.md`.

/// KH1's gravity, units per 60 Hz frame²: the takeoff `0x1402AFDA0`, the air step `0x1402AF6A0`,
/// the constant at RVA `0x3E9D14`.
pub const KH1_GRAVITY: f32 = 0.484_693_88;
/// KH1's fall cap, units per 60 Hz frame (× the frame step): RVA `0x42F968`, the air step.
pub const KH1_FALL_CAP: f32 = 20.0;

/// KH1 Sora's jump height: 190 (290 with KH1's High Jump). Checked (trace): the sister mod's KH1
/// traces, the takeoff's `h = -190`, `T = 28.0` (= `apex_time(190)`) and `h = -290`, `T = 34.592`.
pub const KH1_JUMP: f32 = 190.0;
/// KH1 Sora's jump height with KH1's High Jump. Checked (trace), as `KH1_JUMP`.
pub const KH1_HIGH_JUMP: f32 = 290.0;
/// KH1 Sora's turn rate on the ground, radians a 60 Hz frame: π/16. Checked (trace): the most his
/// facing turns in one frame in the KH1 traces (0.196 / 0.197).
pub const KH1_TURN: f32 = std::f32::consts::PI / 16.0;
/// KH1's air acceleration (stats `+0x14`): 0.1 × the stick's speed (8 at full tilt) a frame.
/// Checked (trace): +0.8 a frame from rest with the stick held.
pub const KH1_AIR_ACCEL: f32 = 0.1;
/// KH1's air drag (stats `+0x18`), per 30 Hz frame; its square root a 60 Hz frame. Checked (trace):
/// 0.97468 a frame with the stick let go (√0.95 = 0.97468).
pub const KH1_AIR_DRAG: f32 = 0.95;

/// KH1's air steering with the stick held, one 60 Hz frame (KH1's air step `0x1402AF6A0`, Checked
/// (decomp); its numbers Checked (trace)): `v += dir × accel × speed`, then its length kept to the
/// larger of its old length and the run speed. `v` and `dir` are (x, z).
pub fn air_steer(v: [f32; 2], dir: [f32; 2], speed: f32, run: f32) -> [f32; 2] {
    let old = v[0].hypot(v[1]);
    let n = [v[0] + dir[0] * KH1_AIR_ACCEL * speed, v[1] + dir[1] * KH1_AIR_ACCEL * speed];
    let len = n[0].hypot(n[1]);
    let cap = old.max(run);
    if len > cap && len > 0.0 {
        [n[0] * cap / len, n[1] * cap / len]
    } else {
        n
    }
}

/// KH1's early-release hop: a launch at speed 5 (`0x1402A7B30` → `0x1402AF8F0 (actor, 5.0)`,
/// Checked (decomp)): its time to the apex is 5 frames and its height `g/2 · 5²` (6.06). Checked
/// (trace): the KH1 traces' `T = 5, h = -6.059`.
pub const KH1_HOP_TIME: f32 = 5.0;
pub fn hop_height() -> f32 {
    KH1_GRAVITY * 0.5 * KH1_HOP_TIME * KH1_HOP_TIME
}

/// The time an arc of height `h` takes to its apex (or to fall `h`) with KH1's gravity, as
/// KH1's takeoff computes it (`T = sqrt(|2h| / g)`).
pub fn apex_time(h: f32) -> f32 {
    ((h + h) / KH1_GRAVITY).abs().sqrt()
}

/// KH1's air knock-back (`0x14029CC40` → `0x1402B7F20(actor, 20.0, dir)`, Checked (decomp)): a
/// launch of height 20 (`0x1402AF960`) with the sideways speed of btltbl's reaction block `+0x3C`,
/// 16 (the sister project's `kh1-hit-reactions-native.md` §4, Checked (files) there), the
/// sideways speed then × `KH1_AIR_DRAG` a 30 Hz frame with the stick let go (state 2's step, the
/// sister project's `kh1-physics.md`).
pub const KH1_KNOCKBACK_HOP: f32 = 20.0;
pub const KH1_KNOCKBACK_SPEED: f32 = 16.0;

/// Height above the launch point `t` frames into a KH1 launch of height `h` (KH1's arc: rising
/// `h · sin(π/2 · sin(π/2 · t / T))` to the apex at `T`, then `h − g/2 · (t − T)²`; the sister
/// project's `kh1-physics.md`, Checked (decomp) there).
pub fn arc_height(t: f32, h: f32) -> f32 {
    let tt = apex_time(h);
    if t < tt {
        let half = std::f32::consts::FRAC_PI_2;
        h * (half * (half * t / tt).sin()).sin()
    } else {
        h - KH1_GRAVITY * 0.5 * (t - tt) * (t - tt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kh1_knockback_arc() {
        assert_eq!(arc_height(0.0, KH1_KNOCKBACK_HOP), 0.0);
        let t = apex_time(KH1_KNOCKBACK_HOP);
        assert!((arc_height(t, KH1_KNOCKBACK_HOP) - 20.0).abs() < 1e-3);
        assert!(arc_height(t * 0.5, KH1_KNOCKBACK_HOP) > 10.0);
        assert!(arc_height(2.0 * t, KH1_KNOCKBACK_HOP) < 0.1);
    }

    #[test]
    fn kh1_apex_time() {
        assert!((apex_time(100.0) - 20.3125).abs() < 1e-3);
        assert_eq!(apex_time(-100.0), apex_time(100.0));
        // The KH1 traces' takeoffs.
        assert!((apex_time(KH1_JUMP) - 28.0).abs() < 1e-3);
        assert!((apex_time(KH1_HIGH_JUMP) - 34.592).abs() < 1e-3);
        assert!((KH1_AIR_DRAG.sqrt() - 0.97468).abs() < 1e-4);
        assert!((hop_height() - 6.059).abs() < 1e-3);
    }

    #[test]
    fn kh1_air_steer_matches_the_trace() {
        // From rest, stick full forward (speed 8): +0.8 a frame up to 8, then held at 8.
        let mut v = [0.0, 0.0];
        let mut seen = vec![];
        for _ in 0..12 {
            v = air_steer(v, [0.0, 1.0], 8.0, 8.0);
            seen.push((v[1] * 1000.0).round() / 1000.0);
        }
        assert_eq!(&seen[..3], &[0.8, 1.6, 2.4]);
        assert_eq!(seen[9], 8.0);
        assert_eq!(seen[11], 8.0);
        // Faster than the run (a launch), steering keeps the speed but never adds to it.
        let w = air_steer([12.0, 0.0], [0.0, 1.0], 8.0, 8.0);
        assert!((w[0].hypot(w[1]) - 12.0).abs() < 1e-4);
    }
}
