//! KH1's Gravity on an enemy's height (pure; the hook side is `win::squash`).
//!
//! KH1 has no slow under the falling sphere: it squashes the model (Checked (files + decomp),
//! KH1-in-ReCom `recom-m7-magic.md` "M7 f2" and "0.122.0"):
//! - **Under the falling sphere** (the falling thread sets `+0x88 |= 4` on every enemy touching it,
//!   each fall frame), the actor update `0x140297FE0` (normal branch; not while stopped or flattened)
//!   eases its height `+0x480 = (h − 0.8) × 0.95 + 0.8 − sin(p) × 0.05`, `p = wrap(p + π/2)` into
//!   [−π, π]; without the bit `h += 0.05` up to 1 and `p = 0`; the bit is then cleared. `+0x480` is
//!   the world matrix's y scale (`0x1402A0A40`), its only reader.
//! - **The hit flattens** (reaction kind 8, `0x1402B7DE0`): on the ground the timer `+0x404` = 68
//!   (request 0x66), in the air or a special movement 75 (movement 0x16 / 0x17, request 0x68). Each
//!   frame (`0x1402B7260`): timer − 1; above 60: `h = 0.1 + 0.9 × (t − 60) / 8` (ground) or `/ 15`
//!   (air); else 0.1; at 0 the get-up (request 0x67): `h += (1 − h) × 0.3` while it plays; after, the
//!   normal branch's +0.05 a frame. A flinch on a flattened actor knocks it out of it (`0x1402B8B90`):
//!   then +0.05 a frame.

/// One enemy's height state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The normal branch: the phase of the wobble.
    Normal { phase: f32 },
    /// Flattened: frames left on KH1's timer, the air variant.
    Flat { timer: f32, air: bool },
    /// KH1's get-up after a flatten.
    GetUp,
}

pub const FLAT_GROUND: f32 = 68.0;
pub const FLAT_AIR: f32 = 75.0;
const SQUASHED: f32 = 0.1;
/// Assumption: KH2's enemies have no KH1 get-up motion (0x67); its ease runs until the height is
/// this close to 1, then the normal spring takes over.
const GETUP_UNTIL: f32 = 0.95;

fn wrap(p: f32) -> f32 {
    use std::f32::consts::PI;
    let mut p = p;
    while p > PI {
        p -= 2.0 * PI;
    }
    while p < -PI {
        p += 2.0 * PI;
    }
    p
}

/// One frame of KH1's height for an enemy at height `h`. Returns the new state and height.
pub fn step(state: State, h: f32, under: bool) -> (State, f32) {
    match state {
        State::Normal { phase } => {
            if under {
                let h = (h - 0.8) * 0.95 + 0.8 - phase.sin() * 0.05;
                (State::Normal { phase: wrap(phase + std::f32::consts::FRAC_PI_2) }, h)
            } else {
                (State::Normal { phase: 0.0 }, (h + 0.05).min(1.0))
            }
        }
        State::Flat { timer, air } => {
            let t = timer - 1.0;
            if t <= 0.0 {
                let h = h + (1.0 - h) * 0.3;
                return (State::GetUp, h);
            }
            let h = if t > 60.0 { SQUASHED + 0.9 * (t - 60.0) / if air { 15.0 } else { 8.0 } } else { SQUASHED };
            (State::Flat { timer: t, air }, h)
        }
        State::GetUp => {
            if h >= GETUP_UNTIL {
                (State::Normal { phase: 0.0 }, (h + 0.05).min(1.0))
            } else {
                (State::GetUp, h + (1.0 - h) * 0.3)
            }
        }
    }
}

/// Whether a state at height `h` is over (back to the normal height, nothing left to do).
pub fn done(state: State, h: f32) -> bool {
    matches!(state, State::Normal { .. }) && h >= 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_the_sphere_kh1s_ease_and_wobble() {
        let (mut s, mut h) = (State::Normal { phase: 0.0 }, 1.0);
        let mut hs = vec![];
        for _ in 0..40 {
            (s, h) = step(s, h, true);
            hs.push(h);
        }
        // First frame: (1 − 0.8) × 0.95 + 0.8 − sin 0 × 0.05 = 0.99; then − 0.05 at sin(π/2).
        assert!((hs[0] - 0.99).abs() < 1e-6);
        assert!((hs[1] - ((0.99 - 0.8) * 0.95 + 0.8 - 0.05)).abs() < 1e-6);
        // It settles around 0.8 with a 4-frame wobble.
        assert!(hs[36..].iter().all(|&h| (0.7..0.9).contains(&h)));
        // Out from under: +0.05 a frame back to 1.
        let mut n = 0;
        while !done(s, h) {
            (s, h) = step(s, h, false);
            n += 1;
        }
        assert!((4..=7).contains(&n), "{n}");
    }

    #[test]
    fn the_flatten_is_kh1s_frame_by_frame() {
        let (mut s, mut h) = (State::Flat { timer: FLAT_GROUND, air: false }, 1.0);
        let mut hs = vec![];
        for _ in 0..68 {
            (s, h) = step(s, h, false);
            hs.push(h);
        }
        // The 7 squashed frames 0.8875 … 0.2125, then 0.1 from the 8th to the 67th, then the get-up.
        assert!((hs[0] - 0.8875).abs() < 1e-6 && (hs[6] - 0.2125).abs() < 1e-6);
        assert!(hs[7..67].iter().all(|&h| (h - 0.1).abs() < 1e-6));
        assert_eq!(s, State::GetUp);
        assert!((hs[67] - 0.37).abs() < 1e-6);
        let (_, h2) = step(s, h, false);
        assert!((h2 - 0.559).abs() < 1e-6);
        // Air: 75 and / 15.
        let (s, h) = step(State::Flat { timer: FLAT_AIR, air: true }, 1.0, false);
        assert!((h - (0.1 + 0.9 * 14.0 / 15.0)).abs() < 1e-6);
        assert_eq!(s, State::Flat { timer: 74.0, air: true });
    }
}
