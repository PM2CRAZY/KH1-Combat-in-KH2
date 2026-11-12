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
/// KH1 Sora's turn rate on the ground, radians a 60 Hz frame: π/16. Checked (trace): the most his
/// facing turns in one frame in the KH1 traces (0.196 / 0.197).
pub const KH1_TURN: f32 = std::f32::consts::PI / 16.0;
/// KH1's air acceleration (stats `+0x14`): 0.1 × the stick's speed (8 at full tilt) a frame.
/// Checked (trace): +0.8 a frame from rest with the stick held.
pub const KH1_AIR_ACCEL: f32 = 0.1;
/// KH1's air drag (stats `+0x18`), per 30 Hz frame; its square root a 60 Hz frame. Checked (trace):
/// 0.97468 a frame with the stick let go (√0.95 = 0.97468).
pub const KH1_AIR_DRAG: f32 = 0.95;

/// The time an arc of height `h` takes to its apex (or to fall `h`) with KH1's gravity, as
/// KH1's takeoff computes it (`T = sqrt(|2h| / g)`).
pub fn apex_time(h: f32) -> f32 {
    ((h + h) / KH1_GRAVITY).abs().sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kh1_apex_time() {
        assert!((apex_time(100.0) - 20.3125).abs() < 1e-3);
        assert_eq!(apex_time(-100.0), apex_time(100.0));
        // The KH1 traces' takeoffs.
        assert!((apex_time(KH1_JUMP) - 28.0).abs() < 1e-3);
        assert!((apex_time(290.0) - 34.592).abs() < 1e-3);
        assert!((KH1_AIR_DRAG.sqrt() - 0.97468).abs() < 1e-4);
    }
}
