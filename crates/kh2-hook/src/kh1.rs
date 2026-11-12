//! KH1's movement numbers (Steam KH1, `KINGDOM HEARTS FINAL MIX.exe`), plain Rust so they are
//! tested on any platform. Each is Checked (decomp + exe); `docs/movement.md`.

/// KH1's gravity, units per 60 Hz frame²: the takeoff `0x1402AFDA0`, the air step `0x1402AF6A0`,
/// the constant at RVA `0x3E9D14`.
pub const KH1_GRAVITY: f32 = 0.484_693_88;
/// KH1's fall cap, units per 60 Hz frame (× the frame step): RVA `0x42F968`, the air step.
pub const KH1_FALL_CAP: f32 = 20.0;

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
    }
}
