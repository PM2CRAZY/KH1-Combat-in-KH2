//! KH1 FM's follow and lock camera, engine-agnostic, as KH2 runs it with `camera = kh1`
//! (`win/camera.rs`). Copied from the owner's KH1-in-ReCom (`crates/kh1-recom/src/camera.rs`,
//! `pub mod kh1`, and `crates/kh1-core/src/math.rs`, testing branch), where every number was read
//! from the KH1 Steam decomp / exe (the tags on each item are that project's). Only the Re:CoM
//! stick and inversion helpers are left out; `win/camera.rs` gives KH2's own.
#![allow(dead_code)]

pub mod math {
    //! A tiny 3-vector. KH1 world units (about 1 cm), **y down** (up is −y), as in the game.
    //! Yaw is `atan2(x, z)`; forward for a yaw is `(sin yaw, 0, cos yaw)` (`0x140297E10`).

    use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Vec3 {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }

    impl Vec3 {
        pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };

        pub const fn new(x: f32, y: f32, z: f32) -> Vec3 {
            Vec3 { x, y, z }
        }

        pub fn dot(self, o: Vec3) -> f32 {
            self.x * o.x + self.y * o.y + self.z * o.z
        }

        pub fn length(self) -> f32 {
            self.dot(self).abs().sqrt()
        }

        /// Length on the ground plane (`0x1402A9260`).
        pub fn horizontal_length(self) -> f32 {
            (self.x * self.x + self.z * self.z).abs().sqrt()
        }

        /// The same vector with y = 0.
        pub fn flat(self) -> Vec3 {
            Vec3::new(self.x, 0.0, self.z)
        }

        /// Unit vector, or zero for a zero vector.
        pub fn normalized(self) -> Vec3 {
            let l = self.length();
            if l > 0.0 {
                self * (1.0 / l)
            } else {
                Vec3::ZERO
            }
        }

        /// `0x1402A93D0`: the unit vector and the length. A zero vector gives (0, 0, 1) and 0.
        pub fn normalize_len(self) -> (Vec3, f32) {
            let l = self.length();
            if l != 0.0 {
                (self * (1.0 / l), l)
            } else {
                (Vec3::new(0.0, 0.0, 1.0), 0.0)
            }
        }

        pub fn is_zero_xz(self) -> bool {
            self.x == 0.0 && self.z == 0.0
        }

        /// Forward vector for a yaw (`0x140297E10`).
        pub fn from_yaw(yaw: f32) -> Vec3 {
            Vec3::new(yaw.sin(), 0.0, yaw.cos())
        }

        /// Yaw of a vector (`0x140297FB0`: `atan2(x, z)`).
        pub fn yaw(self) -> f32 {
            self.x.atan2(self.z)
        }

        /// Rotates a local (x right, z forward) offset by a yaw. Assumption: KH1's actor matrix maps
        /// local +z to `from_yaw(yaw)` and local +x to `(cos yaw, 0, −sin yaw)`.
        pub fn rotate_yaw(self, yaw: f32) -> Vec3 {
            let (s, c) = yaw.sin_cos();
            Vec3::new(self.x * c + self.z * s, self.y, -self.x * s + self.z * c)
        }

        pub fn lerp(self, o: Vec3, t: f32) -> Vec3 {
            self + (o - self) * t
        }
    }

    impl Add for Vec3 {
        type Output = Vec3;
        fn add(self, o: Vec3) -> Vec3 {
            Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
        }
    }

    impl AddAssign for Vec3 {
        fn add_assign(&mut self, o: Vec3) {
            *self = *self + o;
        }
    }

    impl Sub for Vec3 {
        type Output = Vec3;
        fn sub(self, o: Vec3) -> Vec3 {
            Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
        }
    }

    impl SubAssign for Vec3 {
        fn sub_assign(&mut self, o: Vec3) {
            *self = *self - o;
        }
    }

    impl Mul<f32> for Vec3 {
        type Output = Vec3;
        fn mul(self, k: f32) -> Vec3 {
            Vec3::new(self.x * k, self.y * k, self.z * k)
        }
    }

    impl Neg for Vec3 {
        type Output = Vec3;
        fn neg(self) -> Vec3 {
            Vec3::new(-self.x, -self.y, -self.z)
        }
    }

    /// `0x1402A8280`: wraps an angle into [−π, π].
    pub fn wrap_angle(mut a: f32) -> f32 {
        use std::f32::consts::{PI, TAU};
        let mut n = 0;
        while n < 0x400 {
            if a > PI {
                a -= TAU;
            } else if a < -PI {
                a += TAU;
            } else {
                break;
            }
            n += 1;
        }
        a
    }

    /// `0x1402A8300`: moves `from` toward `to` by at most `rate × step`.
    pub fn turn_toward(from: f32, to: f32, rate: f32, step: f32) -> f32 {
        let d = wrap_angle(to - from);
        wrap_angle(from + clamp_sym(d, rate * step))
    }

    /// The exe's symmetric clamp: `d` limited to [−m, m] (a NaN `d` gives `m`).
    pub fn clamp_sym(d: f32, m: f32) -> f32 {
        if d <= m {
            if -m <= d {
                d
            } else {
                -m
            }
        } else {
            m
        }
    }

}

/// KH1 FM's follow camera (mode 1 `0x1401E1920`) and lock camera (mode 3 `0x1401E2650`).
pub mod kh1 {
    use super::math::{wrap_angle, Vec3};
    use std::f32::consts::PI;

    /// Built-in defaults (`0x140506CA0`: look offset (0, −100, −50) in Sora's frame, 1 m up and
    /// 0.5 m **ahead** of him through his matrix ([`actor_point`]); `0x140506CB0`
    /// pitch 10° (bits 0x3E32B8C3); `0x140506CBC` distance 400). Checked (exe). Owner's answer Q8 (a):
    /// these everywhere (Re:CoM's rooms have none of KH1's room camera blocks).
    pub const OFFSET: Vec3 = Vec3::new(0.0, -100.0, -50.0);
    pub const PITCH_DEFAULT: f32 = 0.174_532_94;
    pub const DIST_DEFAULT: f32 = 400.0;
    /// The look height above Sora's feet, in metres (the offset's −100).
    pub const LOOK_HEIGHT_M: f32 = 1.0;
    /// FOV of the follow (`0x1403E9E44`). Checked (exe).
    pub const FOV: f32 = 53.0;

    /// The look point's spring (`0x1401E1920`): 20 % of the horizontal gap, vertically 10 % up /
    /// 50 % down, all halved below a frame step of 2.0. Checked (decomp).
    pub const LOOK_H: f32 = 0.2;
    pub const LOOK_UP: f32 = 0.1;
    pub const LOOK_DOWN: f32 = 0.5;

    /// The stick (`0x1401E7320`): pad maximum 2.5° (bits 0x3D32B8C3), acceleration 0.15°, decay
    /// 0.24°; mouse acceleration 0.3°, decay 4°, maximum 2.5° + 0.0875° × sensitivity; pitch
    /// clamped −15°..+50°. Checked (decomp).
    pub const PAD_MAX: f32 = 0.043_633_23;
    pub const PAD_ACCEL: f32 = 0.002_617_994;
    pub const PAD_DECAY: f32 = 0.004_188_790_5;
    pub const MOUSE_ACCEL: f32 = 0.005_235_988;
    pub const MOUSE_DECAY: f32 = 0.069_813_17;
    /// KH1's PC mouse sensitivity (0..40); its default 20 (Epic notes; owner Q7 (a): the default).
    pub const MOUSE_SENSITIVITY: f32 = 20.0;
    pub const PITCH_MIN: f32 = -0.261_799_4;
    pub const PITCH_MAX: f32 = 0.872_664_63;

    /// The recenter (`0x1401E62A0`): 60 frames, 30° wall steps; the "swap" recenter below 5° does
    /// nothing; its pitch rises up to 60° × 0.7. Checked (decomp).
    pub const RECENTER_FRAMES: f32 = 60.0;
    pub const RECENTER_STEP: f32 = 0.523_598_8;
    pub const SWAP_MIN: f32 = 0.087_266_47;
    pub const SWAP_PITCH: f32 = 1.047_197_6;

    /// The Manual branch's wall ray: 60 past the eye, the eye 60 from the wall, the look point at
    /// least 20 from the eye. Checked (decomp).
    pub const RAY_PAST: f32 = 60.0;
    pub const NEAR_LOOK: f32 = 20.0;
    /// The re-place's sphere walk (`0x1401E4820`, `0x140196560` with 40.0). Checked (decomp).
    pub const SPHERE: f32 = 40.0;

    /// KH1's PC right stick: the raw byte's dead zone 60 of 127.5 removed, radial
    /// (`0x14028B4F0`, `0x14298CB00` = 60.0); with the mouse 1.0. Checked (decomp).
    pub const DEAD_ZONE: f32 = 60.0;
    pub const MOUSE_DEAD_ZONE: f32 = 1.0;

    /// KH1's collision as Re:CoM's camera collision object answers it (`win::camera`).
    pub trait World {
        /// The first contact of a sphere of `radius` moved from `from` to `to`: where it stops, or
        /// None when the way is clear. A `radius` of [`RAY`] stands for KH1's ray and segment tests
        /// (`0x140193790`, `0x140193180`).
        fn sweep(&mut self, from: Vec3, to: Vec3, radius: f32) -> Option<Vec3>;
        /// The surface normal of the last contact (KH1's polygon normal `0x140191DD0`; KH1 units,
        /// y down). Default: up.
        fn normal(&mut self) -> Vec3 {
            Vec3::new(0.0, -1.0, 0.0)
        }
        /// KH1's `0x140196300` with a flat circle of `radius` at `at` (the swing-behind's wall
        /// probe): a wall within `radius` horizontally.
        fn near_wall(&mut self, _at: Vec3, _radius: f32) -> bool {
            false
        }
    }

    /// The sphere radius that stands for KH1's ray (Re:CoM's own smallest camera sweep, its tail's
    /// 0.05 × 0.1 m). Assumption: as good as a ray against Re:CoM's walls.
    pub const RAY: f32 = 0.5;

    /// No walls (tests).
    pub struct Open;
    impl World for Open {
        fn sweep(&mut self, _: Vec3, _: Vec3, _: f32) -> Option<Vec3> {
            None
        }
    }

    /// One frame's input, in KH1's terms.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Input {
        /// Sora's feet (his matrix's translation) and facing.
        pub pos: Vec3,
        pub facing: f32,
        /// The right stick (−1..1; y down positive), after KH1's dead zone.
        pub stick: [f32; 2],
        /// Keyboard and mouse in use (Re:CoM's `0x140C2FF64`): KH1's mouse rule (owner Q13 (a)).
        pub mouse: bool,
        /// Sora can act (KH1 `0x140297950` false; Re:CoM: the camera's input lock `+0x188 & 1` clear).
        pub can_act: bool,
        /// The recenter button was pressed this frame (KH1's R3; Re:CoM's reset bit 0x400).
        pub recenter: bool,
        /// KH1's inversion floats (`0x140506CDC` left / right, `0x140506CD8` up / down).
        pub inv: [f32; 2],
        /// The frame step (1.0 a 60 Hz frame).
        pub step: f32,
        /// KH1's Auto camera (its Config `+0x8` = 0; owner: the Auto camera row).
        pub auto: bool,
        /// Sora's KH1 movement state (`+0x70`: 0 ground, 2 air, 0x15 glide …), for the speed
        /// thresholds and the pitch assist's skips.
        pub state: u32,
        /// Sora's air velocity (`+0x3C0`, units a frame, y down).
        pub vel: Vec3,
    }

    impl Input {
        /// KH1's look target (`0x1401E1920`: `0x140386A40(target, Sora + 0xD0, offset)`): Sora's
        /// matrix × the offset, **1 m up and 0.5 m ahead of him** (his matrix's local +z is his
        /// back, [`actor_point`]); and the point above him (the offset with x and z zeroed).
        /// Checked (decomp + disasm); 0.134.0–0.139.2 put it 0.5 m behind (`docs/recom-m12-camera-plan.md`
        /// § 18).
        pub fn target(&self) -> Vec3 {
            actor_point(self.pos, self.facing, OFFSET)
        }
        pub fn above(&self) -> Vec3 {
            actor_point(self.pos, self.facing, Vec3::new(0.0, OFFSET.y, 0.0))
        }
    }

    /// A point in Sora's local frame through KH1's actor matrix (`+0xD0`, built by `0x1402A0A40`
    /// with `+0x374 & 0x1000` clear): row 0 = facing × up (`0x1400E82F0`, a × b), row 1 = row 2 × row 0
    /// = up (0, 1, 0), **row 2 = (0, 0, 0, 1) − facing** (`0x140386D10` from `0x140527FF0`), row 3 = his
    /// position; facing = `+0x30` = (sin ψ, 0, cos ψ) (`0x140297E10`); `0x1400E5AD0` gives
    /// Σ vᵢ · rowᵢ. So local +z points behind him and +x to (−cos ψ, 0, sin ψ). Checked (decomp +
    /// disasm, KH1 Steam). The slope branch (`0x140292010`, two floor points along ±facing) only
    /// tilts row 2 along the floor: not carried (Assumption: Sora on flat floors, or level enough).
    pub fn actor_point(pos: Vec3, facing: f32, local: Vec3) -> Vec3 {
        let f = Vec3::from_yaw(facing);
        let row0 = Vec3::new(-f.z, 0.0, f.x);
        let row1 = Vec3::new(0.0, 1.0, 0.0);
        let row2 = f * -1.0;
        pos + row0 * local.x + row1 * local.y + row2 * local.z
    }

    /// KH1's eye offset for a pitch, yaw and distance: `Rx(pitch) · Ry(yaw) · (0, 0, d)` with
    /// KH1's row-vector matrices (`0x1400E6AC0`, `0x1400E6C80`): the eye above for a positive
    /// pitch (y down), and `yaw = atan2(eye − look)` (`0x14028C720`). Checked (decomp).
    pub fn offset(d: f32, pitch: f32, yaw: f32) -> Vec3 {
        let (sp, cp) = pitch.sin_cos();
        let (sy, cy) = yaw.sin_cos();
        Vec3::new(d * cp * sy, -d * sp, d * cp * cy)
    }

    /// KH1's yaw from an eye and a look point (`0x14028C720`).
    pub fn yaw_of(eye: Vec3, look: Vec3) -> f32 {
        (eye.x - look.x).atan2(eye.z - look.z)
    }

    /// A running recenter (`0x142537EE0..0x142537EF8`).
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Recenter {
        /// Frames left (`0x142537EEC`); 0 none.
        pub left: f32,
        pub total: f32,
        /// 0 turn negative, 1 turn positive, 2 the swap past a wall.
        pub kind: u32,
        pub from: f32,
        pub delta: f32,
        pub dist: f32,
        pub pitch: f32,
    }

    /// The swing-behind's ramps (`0x1425380DC..0x1425380E8`).
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Swing {
        /// The wall ramp (`0x1425380E0`), the second time ramp (`E4`), the first (`E8`).
        pub wall: f32,
        pub second: f32,
        pub first: f32,
        /// Sora's facing last frame (`0x1425380DC`).
        pub facing: f32,
    }

    /// The wall-avoid's swing (`0x142537ED0..ED8`).
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Avoid {
        pub from: f32,
        pub delta: f32,
        pub left: f32,
    }

    /// The pitch assist's timers (`0x1425380D0` drop, `D4` floor under the eye, `D8` standing).
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Assist {
        pub drop: f32,
        pub floor: f32,
        pub still: f32,
    }

    /// KH1's follow camera state (its globals `0x142537E60..`, `0x1425380C0..0x1425380F0`).
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Follow {
        pub look: Vec3,
        pub eye: Vec3,
        pub pitch: f32,
        pub yaw: f32,
        pub dist: f32,
        /// Stick velocities (pitch, yaw), per frame.
        pub vel: [f32; 2],
        pub rc: Recenter,
        pub swing: Swing,
        pub avoid: Avoid,
        pub assist: Assist,
        /// Sora's position last frame (KH1's `+0x490`) and his speeds this frame: 3D (`0x142537E50`)
        /// and horizontal (`E54`), units a frame (`0x1401E5D90`).
        pub prev: Option<Vec3>,
        pub speed: f32,
        pub speed_h: f32,
        /// The lock camera's state (mode 3).
        pub lock: Lock,
        /// The camera task's mode-switch blend of the drawn view.
        pub blend: Blend,
    }

    /// KH1's mode-switch blend (`0x1401E5360`, the task `0x1401E5D90`): the drawn eye / look go
    /// from where they were to the new mode's with weight `cos(left / total · π/2)` on the old.
    /// Checked (decomp; `0x140386B70` = `a·t + b·(1 − t)`, `0x1400E7CB0`).
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Blend {
        pub from_eye: Vec3,
        pub from_look: Vec3,
        pub left: f32,
        pub total: f32,
    }

    /// The lock camera's own globals: the anchor's height assist (`0x14253810C`), the keep-Sora
    /// counter (`0x142538108`), the re-lock blend (`0x142537F08`, from `0x142537F10`).
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Lock {
        pub height: f32,
        pub screen: f32,
        pub relock: f32,
        pub old_look: Vec3,
    }

    /// The lock target as Re:CoM gives it: its point (Re:CoM's lock-on point `player+0x870`, its
    /// `CamTrgOfs` point) and KH1's lock radius for it (the twin's, `win::stats::lock_radius`).
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Target {
        pub point: Vec3,
        pub radius: f32,
    }

    /// The lock camera's numbers (`0x1401E2650`; Checked (decomp + exe)): the anchor
    /// (0, −100, 350) in Sora's frame turned by the camera's yaw (`0x140506DF0`), the look-past
    /// (radius + 100), the eye's 75 dead zone and 90 % close, the ±5° yaw band at half the error
    /// up to 10° a frame, the point 100 ahead (`0x140506E00`), the keep-Sora counter (to 40, −2 a
    /// frame inside 80 % of the half FOV, 80 % pull), the re-lock blend of 60, the height assist
    /// (floor under the eye: 30 / 150 → up to −170; a target over 200 above the eye → up to 20).
    pub mod lockcam {
        use crate::kh1cam::math::Vec3;
        pub const ANCHOR: Vec3 = Vec3::new(0.0, -100.0, 350.0);
        pub const AHEAD: Vec3 = Vec3::new(0.0, 0.0, -100.0);
        pub const PAST: f32 = 100.0;
        pub const DEAD: f32 = 75.0;
        pub const CLOSE: f32 = 0.9;
        pub const BAND: f32 = 0.087_266_47;
        pub const TURN_MAX: f32 = 0.174_532_94;
        pub const SCREEN_MAX: f32 = 40.0;
        pub const SCREEN_PULL: f32 = 0.8;
        pub const RELOCK: f32 = 60.0;
        /// Sora's body point (code `0x8003`: his record {0x8002, 0, 85}; `recom-m13-kh1-fx.md`).
        pub const BODY: f32 = 85.0;
        pub const ENTER_BLEND: f32 = 60.0;
        pub const EXIT_BLEND: f32 = 45.0;
        pub const FLOOR_PROBE: f32 = 10000.0;
    }

    impl Default for Follow {
        fn default() -> Self {
            Follow { look: Vec3::ZERO, eye: Vec3::ZERO, pitch: PITCH_DEFAULT, yaw: 0.0, dist: DIST_DEFAULT, vel: [0.0; 2], rc: Recenter::default(), swing: Swing::default(), avoid: Avoid::default(), assist: Assist::default(), prev: None, speed: 0.0, speed_h: 0.0, lock: Lock::default(), blend: Blend::default() }
        }
    }

    /// Sora "moves" for the camera: faster than his state's threshold (1.0 a frame on the
    /// ground, 7.0 in states 8 / 0x18, else 0.1; `0x1401E6080`). Checked (decomp).
    pub fn moving(state: u32, speed: f32) -> bool {
        let t = match state {
            0 => 1.0,
            8 | 0x18 => 7.0,
            _ => 0.1,
        };
        t < speed
    }

    /// The Auto pieces' numbers (Checked (decomp)): the swing-behind `0x1401E6FF0` (ramps of 60
    /// then 90 frames, a wall ramp of 60, with the mouse 0 / 40 / 20; gains 2 % + 2 % of the angle
    /// to Sora's back, faded past 120°, none at 180°; a facing change of 30° resets); its wall
    /// probe radius 250; the wall-avoid `0x1401E6C10` (side rays 50 off, ±30° rays of 300, a
    /// 40-frame sin swing); the pitch assist `0x1401E6670` (3° a frame × its ramp of 30, at most
    /// 85°, the eye's floor ray 55 with 49.5, back to the default pitch 3 % a frame; the standing
    /// ledge look after 60 frames: 100 ahead, then 500 down from 30 ahead 100 up; in the air 1000
    /// down, or along a fall faster than 5 for 250); the eye walk (a 40 sphere, re-placed past
    /// default + 300, a jump at 1000); the visibility re-place `0x1401E7780` (5 segments from the
    /// eye: above Sora, ±15° of it, ±50 up and down); the auto recenter (eye within 40
    /// horizontally, pitch under 60°, Sora moving); the distance back 10 % a frame.
    pub mod auto {
        pub const RAMP1: f32 = 60.0;
        pub const RAMP2: f32 = 90.0;
        pub const WALL_RAMP: f32 = 60.0;
        pub const MOUSE_RAMP1: f32 = 0.0;
        pub const MOUSE_RAMP2: f32 = 40.0;
        pub const MOUSE_WALL_RAMP: f32 = 20.0;
        pub const GAIN: f32 = 0.02;
        pub const FACING_RESET: f32 = 0.523_598_8;
        pub const FADE_FROM: f32 = 2.094_395_2;
        pub const FADE_OVER: f32 = 1.047_197_6;
        pub const PROBE: f32 = 250.0;
        pub const SIDE: f32 = 50.0;
        pub const FAN: f32 = 300.0;
        pub const FAN_ANGLE: f32 = 0.523_598_8;
        pub const AVOID_FRAMES: f32 = 40.0;
        pub const ASSIST_STEP: f32 = 0.052_359_88;
        pub const ASSIST_RAMP: f32 = 30.0;
        pub const PITCH_TOP: f32 = 1.483_529_8;
        pub const EYE_FLOOR: f32 = 55.0;
        pub const EYE_FLOOR_NEAR: f32 = 49.5;
        pub const PITCH_BACK: f32 = 0.03;
        pub const STILL_FRAMES: f32 = 60.0;
        pub const LEDGE_AHEAD: f32 = 100.0;
        pub const LEDGE_FROM: f32 = 30.0;
        pub const LEDGE_DOWN: f32 = 500.0;
        pub const AIR_DOWN: f32 = 1000.0;
        pub const FALL_FAST: f32 = 5.0;
        pub const FALL_AHEAD: f32 = 250.0;
        pub const REPLACE_PAST: f32 = 300.0;
        pub const JUMP: f32 = 1000.0;
        pub const SEE_TURN: f32 = 0.261_799_4;
        pub const SEE_UP: f32 = 50.0;
        pub const SQUEEZE: f32 = 40.0;
        pub const SQUEEZE_PITCH: f32 = 1.047_197_6;
        pub const DIST_BACK: f32 = 0.1;
    }

    impl Follow {
        /// KH1's re-place `0x1401E4820` (through `0x1401E4700`): pitch and distance to the defaults,
        /// the look point to Sora's target, the eye behind it at `yaw`; if a ray from above Sora
        /// to that eye meets a wall, a 40-unit sphere walked from above Sora toward the wall stops
        /// at its contact and is the eye. Checked (decomp); KH1's second test `0x140193650` (when the
        /// ray misses) is not carried (Assumption: Re:CoM's camera collision covers it).
        pub fn replace(&mut self, yaw: f32, inp: &Input, w: &mut dyn World) {
            self.pitch = PITCH_DEFAULT;
            self.dist = DIST_DEFAULT;
            self.look = inp.target();
            self.eye = self.look + offset(self.dist, self.pitch, wrap_angle(yaw));
            let above = inp.above();
            if let Some(hit) = w.sweep(above, self.eye, RAY) {
                self.eye = w.sweep(above, hit, SPHERE).unwrap_or(hit);
            }
            self.yaw = yaw_of(self.eye, self.look);
        }

        /// KH1's mode-1 entry (`0x142537E44` bit 2): the yaw of the current view, then the re-place.
        pub fn enter(&mut self, view_yaw: f32, inp: &Input, w: &mut dyn World) {
            self.replace(view_yaw, inp, w);
        }

        /// Behind Sora (his facing + π): the camera's first frame in a room and the return from an
        /// event camera (owner's answer Q4 (b): KH1's way).
        pub fn behind(&mut self, inp: &Input, w: &mut dyn World) {
            self.replace(wrap_angle(inp.facing + PI), inp, w);
        }

        /// The look point's spring; during a mode blend it snaps to the target.
        fn spring_look(&mut self, inp: &Input, w: &mut dyn World) {
            if 0.0 < self.blend.left {
                self.look = inp.target();
                return;
            }
            let above = inp.above();
            let mut goal = inp.target();
            if w.sweep(above, goal, RAY).is_some() {
                goal = above;
            }
            let mut d = goal - self.look;
            let dy = d.y;
            d.y = 0.0;
            let (n, len) = d.normalize_len();
            let mut m = n * (len * LOOK_H);
            m.y = if dy <= 0.0 { dy * LOOK_UP } else { dy * LOOK_DOWN };
            if inp.step < 2.0 {
                m = m * 0.5;
            }
            self.look += m;
        }

        /// KH1's stick `0x1401E7320`.
        fn stick(&mut self, inp: &Input) {
            if !inp.can_act || self.blend.left > 0.0 || self.rc.left > 0.0 {
                return;
            }
            let half = if inp.step < 2.0 { 0.5 } else { 1.0 };
            let (max, accel, decay) = if inp.mouse { ((MOUSE_SENSITIVITY * 0.05 * 1.75 + 2.5) * PI / 180.0, MOUSE_ACCEL, MOUSE_DECAY) } else { (PAD_MAX, PAD_ACCEL, PAD_DECAY) };
            let [sx, sy] = inp.stick;
            let axis = |v: f32, push: f32, held: bool| -> f32 {
                let v = v + push;
                if v == 0.0 {
                    return 0.0;
                }
                let c = if v <= max { v } else { max };
                let mut r = if -max <= c { c } else { -max };
                if !held {
                    let a = (r.abs() - decay * half).max(0.0);
                    r = a * (r / r.abs());
                }
                r
            };
            self.vel[0] = axis(self.vel[0], -accel * sy * half, sy != 0.0);
            self.vel[1] = axis(self.vel[1], accel * sx * half, sx != 0.0);
            if inp.mouse {
                // KH1's mouse multiplier `|dx × 10|` (at least 1) has no Re:CoM source: 1.
                if sy == 0.0 {
                    self.vel[0] = 0.0;
                }
                if sx == 0.0 {
                    self.vel[1] = 0.0;
                }
                if (0.0 < -self.vel[0] && sy < 0.0) || (-self.vel[0] < 0.0 && 0.0 < sy) {
                    self.vel[0] = 0.0;
                }
                if (0.0 < self.vel[1] && sx < 0.0) || (self.vel[1] < 0.0 && 0.0 < sx) {
                    self.vel[1] = 0.0;
                }
            }
            let p = wrap_angle(inp.inv[1] * self.vel[0] * inp.step + self.pitch);
            self.pitch = p.max(PITCH_MIN).min(PITCH_MAX);
            self.yaw = wrap_angle(inp.inv[0] * self.vel[1] * inp.step + self.yaw);
        }

        /// The recenter's blend, each frame while it runs (`0x1401E1920`).
        fn blend(&mut self, step: f32) {
            if self.rc.left <= 0.0 {
                return;
            }
            let t = self.rc.left - step;
            self.rc.left = t;
            let r = &self.rc;
            if r.kind < 2 {
                let s = ((1.0 - t / r.total) * PI * 0.5).sin();
                if r.kind == 1 {
                    self.yaw = wrap_angle(r.delta * s + r.from);
                } else {
                    self.yaw = wrap_angle(r.from - r.delta * s);
                }
                self.pitch = (1.0 - r.left / r.total) * PITCH_DEFAULT + r.pitch * (r.left / r.total);
            } else if SWAP_MIN < r.delta.abs() {
                let c = ((t / r.total) * PI).cos();
                if 0.0 < c {
                    self.yaw = wrap_angle(r.delta + r.from);
                }
                let f = c.abs() * 0.7 + 0.3;
                self.dist = r.dist * f;
                self.pitch = wrap_angle((1.0 - f) * SWAP_PITCH + PITCH_DEFAULT);
            }
            if self.rc.left <= 0.0 {
                self.dist = DIST_DEFAULT;
                self.pitch = PITCH_DEFAULT;
                self.rc.left = 0.0;
            }
        }

        /// KH1's recenter start `0x1401E62A0`: toward Sora's back (facing + π) in 60 frames, the
        /// short way if every 30° step of the swing is clear, else the long way, else the swap.
        pub fn start_recenter(&mut self, facing: f32, w: &mut dyn World) {
            if 0.0 < self.rc.left {
                return;
            }
            let delta = wrap_angle(facing + PI - self.yaw);
            let mut kind = (0.0 < delta) as u32;
            let at = |yaw: f32| self.look + offset(self.dist, self.pitch, wrap_angle(yaw));
            let (kind, amount) = if w.sweep(self.look, at(self.yaw + delta), RAY).is_none() {
                let step = if kind == 0 { -RECENTER_STEP } else { RECENTER_STEP };
                let ad = delta.abs();
                let mut prev = self.eye;
                let mut f = step;
                let mut clear = true;
                while f.abs() <= ad {
                    let next = at(self.yaw + f);
                    if w.sweep(prev, next, RAY).is_some() {
                        clear = false;
                        break;
                    }
                    prev = next;
                    f += step;
                }
                if clear {
                    (kind, ad)
                } else {
                    // The other way round.
                    let ad = 2.0 * PI - ad;
                    let mut prev = self.eye;
                    let mut f = -step;
                    loop {
                        if ad < f.abs() {
                            kind ^= 1;
                            break (kind, ad);
                        }
                        let next = at(self.yaw + f);
                        if w.sweep(prev, next, RAY).is_some() {
                            break (2, ad);
                        }
                        prev = next;
                        f -= step;
                    }
                }
            } else {
                (2, delta)
            };
            self.rc = Recenter { left: RECENTER_FRAMES, total: RECENTER_FRAMES, kind, from: self.yaw, delta: amount, dist: DIST_DEFAULT, pitch: self.pitch };
        }

        /// The Manual branch's eye (`0x1401E1920`, config `+0x8` = 1).
        fn manual_eye(&mut self, w: &mut dyn World) {
            let goal = self.look + offset(self.dist, self.pitch, self.yaw);
            self.eye = goal;
            let (n, _) = (goal - self.look).normalize_len();
            let past = goal + n * RAY_PAST;
            if let Some(hit) = w.sweep(self.look, past, RAY) {
                let (back, len) = (self.look - hit).normalize_len();
                self.eye = if len <= RAY_PAST { hit } else { hit + back * RAY_PAST };
            }
            let d = self.look - self.eye;
            let len = d.length();
            if len < NEAR_LOOK {
                self.look += d.normalized() * (NEAR_LOOK - len);
            }
        }

        /// One frame of KH1's mode 1, after any entry (`0x1401E1920`): Sora's speeds
        /// (`0x1401E5D90`), the look point's spring, the stick, the recenter's blend; then Auto
        /// (the swing-behind, the wall-avoid, the pitch assist, the eye walk, the visibility
        /// re-place, the auto recenter) or Manual (the pitch assist, the eye's wall ray); R3, the
        /// distance, the yaw from the result.
        pub fn step(&mut self, inp: &Input, w: &mut dyn World) {
            let d = inp.pos - self.prev.unwrap_or(inp.pos);
            self.speed = d.length() / inp.step;
            self.speed_h = d.flat().length() / inp.step;
            self.spring_look(inp, w);
            self.stick(inp);
            self.blend(inp.step);
            let auto = inp.auto && inp.state != 0x1B;
            if auto {
                self.swing_behind(inp, w);
                self.wall_avoid(inp, w);
                self.pitch_assist(inp, w);
                self.auto_eye(inp, w);
                let h = (self.eye - self.look).horizontal_length();
                if h < auto::SQUEEZE && self.pitch < auto::SQUEEZE_PITCH && moving(inp.state, self.speed_h) {
                    self.start_recenter(inp.facing, w);
                }
            } else {
                self.pitch_assist(inp, w);
                self.manual_eye(w);
            }
            if inp.recenter && inp.can_act {
                self.start_recenter(inp.facing, w);
            }
            // Auto: the distance springs back 10 % a frame toward the default; Manual: the default.
            self.dist = if auto {
                let now = (self.eye - self.look).length();
                now + (DIST_DEFAULT - now) * auto::DIST_BACK
            } else {
                DIST_DEFAULT
            };
            self.yaw = yaw_of(self.eye, self.look);
            self.prev = Some(inp.pos);
        }

        /// The swing-behind `0x1401E6FF0`.
        fn swing_behind(&mut self, inp: &Input, w: &mut dyn World) {
            let (r1, r2, rw) = if inp.mouse { (auto::MOUSE_RAMP1, auto::MOUSE_RAMP2, auto::MOUSE_WALL_RAMP) } else { (auto::RAMP1, auto::RAMP2, auto::WALL_RAMP) };
            let sw = &mut self.swing;
            if auto::FACING_RESET <= (sw.facing - inp.facing).abs() || !moving(inp.state, self.speed_h) {
                sw.first = 0.0;
                sw.second = 0.0;
            } else {
                sw.first += inp.step;
                if r1 < sw.first {
                    sw.second += inp.step;
                    sw.first = r1;
                    if r2 < sw.second {
                        sw.second = r2;
                    }
                }
            }
            if 0.0 < self.avoid.left || sw.first <= 0.0 {
                sw.facing = inp.facing;
                return;
            }
            if w.near_wall(self.look, auto::PROBE) || w.near_wall(self.eye, auto::PROBE) {
                let sw = &mut self.swing;
                sw.wall = (sw.wall + inp.step).min(rw);
                if sw.second != 0.0 {
                    sw.second = r2;
                }
            } else {
                let sw = &mut self.swing;
                sw.wall = (sw.wall - inp.step * 0.3).max(0.0);
            }
            let sw = &mut self.swing;
            let delta = wrap_angle(inp.facing + PI - self.yaw);
            let mut g = ((sw.wall / rw) * auto::GAIN + (sw.second / r2) * auto::GAIN) * delta;
            if auto::FADE_FROM <= delta.abs() {
                g *= 1.0 - (delta.abs() - auto::FADE_FROM) / auto::FADE_OVER;
            }
            sw.facing = inp.facing;
            self.yaw += g;
        }

        /// The wall-avoid `0x1401E6C10`: a wall beside the line from the look point to the eye,
        /// and one of two ±30° rays from the eye blocked: swing (40 frames) to run along the wall.
        fn wall_avoid(&mut self, inp: &Input, w: &mut dyn World) {
            let a = &mut self.avoid;
            if 0.0 < a.left {
                self.yaw = ((1.0 - a.left / auto::AVOID_FRAMES) * PI * 0.5).sin() * a.delta + a.from;
                a.left -= inp.step;
                return;
            }
            a.left = 0.0;
            let to_look = self.look - self.eye;
            let back = self.eye - self.look;
            let mut s = back.normalized();
            s.y = 0.0;
            let mut side = s.rotate_yaw(PI / 2.0) * auto::SIDE;
            let mut hit = w.sweep(self.look + side, self.look + side + back, RAY);
            if hit.is_none() {
                side = side * -1.0;
                hit = w.sweep(self.look + side, self.look + side + back, RAY);
            }
            if let Some(at) = hit {
                let normal = w.normal();
                let c = to_look.normalized() * auto::FAN;
                let t1 = w.sweep(self.eye, self.eye + c.rotate_yaw(auto::FAN_ANGLE), RAY).is_some();
                let t2 = w.sweep(self.eye, self.eye + c.rotate_yaw(-auto::FAN_ANGLE), RAY).is_some();
                if t1 && t2 {
                    return;
                }
                if t1 != t2 && moving(inp.state, self.speed_h) {
                    let b = (at - self.eye).flat().normalized();
                    let n = normal.flat().normalized();
                    let mut ang = n.x.atan2(n.z);
                    let cross_y = b.z * n.x - b.x * n.z;
                    ang += if cross_y < 0.0 { -PI / 2.0 } else { PI / 2.0 };
                    let mut d = wrap_angle(ang) - wrap_angle(self.yaw);
                    if PI < d.abs() {
                        d += if 0.0 <= d { -2.0 * PI } else { 2.0 * PI };
                    }
                    self.avoid = Avoid { from: wrap_angle(self.yaw), delta: d, left: auto::AVOID_FRAMES };
                }
            }
            // The right stick turning the camera (KH1's logical 0x1000 / 0x2000 held) cancels it.
            if inp.stick[0] != 0.0 {
                self.avoid.left = 0.0;
            }
        }

        /// The pitch assist `0x1401E6670` (Auto and Manual alike).
        fn pitch_assist(&mut self, inp: &Input, w: &mut dyn World) {
            let slow = !moving(inp.state, self.speed);
            let mut raise = false;
            if slow {
                if inp.auto && inp.state != 0x1B {
                    if self.assist.still <= auto::STILL_FRAMES {
                        self.assist.still += inp.step;
                    } else {
                        let fwd = Vec3::from_yaw(inp.facing);
                        if w.sweep(inp.pos, inp.pos + fwd * auto::LEDGE_AHEAD, RAY).is_none() {
                            let mut from = fwd * auto::LEDGE_FROM + inp.pos;
                            from.y -= 100.0;
                            raise = w.sweep(from, from + Vec3::new(0.0, auto::LEDGE_DOWN, 0.0), RAY).is_none();
                        }
                    }
                }
            } else {
                if inp.state == 2 {
                    if w.sweep(self.look, self.look + Vec3::new(0.0, auto::AIR_DOWN, 0.0), RAY).is_none() {
                        raise = true;
                    } else if auto::FALL_FAST < inp.vel.y {
                        raise = w.sweep(inp.pos, inp.pos + inp.vel.normalized() * auto::FALL_AHEAD, RAY).is_none();
                    }
                }
                self.assist.still = 0.0;
            }
            let skip = matches!(inp.state, 4 | 5 | 8 | 0x15 | 0x18 | 0x1A);
            let a = &mut self.assist;
            if !raise || skip {
                a.drop = 0.0;
            } else {
                self.pitch = wrap_angle(self.pitch + (a.drop / auto::ASSIST_RAMP) * auto::ASSIST_STEP).min(auto::PITCH_TOP);
                a.drop = (a.drop + inp.step).min(auto::ASSIST_RAMP);
            }
            let floor = w.sweep(self.eye, self.eye + Vec3::new(0.0, auto::EYE_FLOOR, 0.0), RAY);
            match floor {
                Some(at) if !skip => {
                    let len = (at - self.eye).length();
                    if len < auto::EYE_FLOOR_NEAR {
                        let a = &mut self.assist;
                        self.pitch = wrap_angle(self.pitch + (1.0 - len / auto::EYE_FLOOR) * (a.floor / auto::ASSIST_RAMP) * auto::ASSIST_STEP).min(auto::PITCH_TOP);
                        a.floor = (a.floor + inp.step).min(auto::ASSIST_RAMP);
                    }
                }
                _ => {
                    let a = &mut self.assist;
                    if !slow {
                        a.floor -= inp.step * 0.3;
                    }
                    if a.floor < 0.0 {
                        a.floor = 0.0;
                    }
                    if a.floor <= 0.0 {
                        self.pitch += wrap_angle(PITCH_DEFAULT - self.pitch) * auto::PITCH_BACK;
                    }
                }
            }
        }

        /// The Auto branch's eye: the goal at the pitch, yaw and distance; re-placed when the
        /// distance has grown past the default + 300; walked there as a 40-unit sphere stopping at
        /// contact (KH1's 20-unit steps as one Re:CoM sweep), or jumped there from 1000 away; then
        /// re-placed when no segment from it reaches Sora (`0x1401E7780`).
        fn auto_eye(&mut self, inp: &Input, w: &mut dyn World) {
            let mut goal = self.look + offset(self.dist, self.pitch, wrap_angle(self.yaw));
            if DIST_DEFAULT + auto::REPLACE_PAST < (goal - self.look).length() {
                // `0x1401E4820` into the goal (the look point and the defaults reset); the walk
                // still starts from the eye.
                let eye = self.eye;
                self.replace(self.yaw, inp, w);
                goal = self.eye;
                self.eye = eye;
            }
            if (goal - self.eye).length() < auto::JUMP {
                self.eye = w.sweep(self.eye, goal, SPHERE).unwrap_or(goal);
            } else {
                self.eye = goal;
            }
            if !self.sees(inp, w) {
                self.replace(self.yaw, inp, w);
            }
        }

        /// `0x1401E7780`: a clear segment from the eye to the point above Sora, to it turned ±15°
        /// about the eye, or to it 50 up or down.
        fn sees(&self, inp: &Input, w: &mut dyn World) -> bool {
            let above = inp.above();
            let v = above - self.eye;
            let targets = [above, self.eye + v.rotate_yaw(auto::SEE_TURN), self.eye + v.rotate_yaw(-auto::SEE_TURN), above + Vec3::new(0.0, auto::SEE_UP, 0.0), above - Vec3::new(0.0, auto::SEE_UP, 0.0)];
            targets.iter().any(|&t| w.sweep(self.eye, t, RAY).is_none())
        }
    }

    impl Follow {
        /// KH1's mode switch `0x1401E5360` with a blend of `frames`: from the drawn view (the
        /// current blend's if one runs, else the mode's own eye / look).
        pub fn switch_blend(&mut self, frames: f32) {
            if self.blend.left <= 0.0 {
                self.blend.from_eye = self.eye;
                self.blend.from_look = self.look;
            } else {
                let (e, l) = self.view_now();
                self.blend.from_eye = e;
                self.blend.from_look = l;
            }
            self.blend.left = frames;
            self.blend.total = frames;
        }

        fn view_now(&self) -> (Vec3, Vec3) {
            let b = &self.blend;
            if b.left <= 0.0 || b.total <= 0.0 {
                return (self.eye, self.look);
            }
            let f = ((b.left / b.total) * PI * 0.5).cos();
            let mix = |from: Vec3, to: Vec3| from * (1.0 - f) + to * f;
            (mix(b.from_eye, self.eye), mix(b.from_look, self.look))
        }

        /// The task's end of frame (`0x1401E5D90`): the blend steps and the drawn eye / look.
        pub fn view(&mut self, step: f32) -> (Vec3, Vec3) {
            if 0.0 < self.blend.left {
                self.blend.left -= step;
                if 0.0 < self.blend.left {
                    return self.view_now();
                }
                self.blend.left = 0.0;
            }
            (self.eye, self.look)
        }

        /// KH1's lock-on start `0x1401E41A0`: in the lock camera already, the look point blends from
        /// where it was over 60 frames (a new target); else the switch to it with a 60 blend.
        pub fn lock_on(&mut self, already: bool) {
            self.lock.old_look = self.look;
            if already {
                self.lock.relock = lockcam::RELOCK;
            } else {
                self.switch_blend(lockcam::ENTER_BLEND);
            }
        }

        /// KH1's unlock `0x1401E4110`: the follow with a 45 blend, its entry at the drawn view's yaw.
        pub fn unlock(&mut self, inp: &Input, w: &mut dyn World) {
            self.switch_blend(lockcam::EXIT_BLEND);
            let (e, l) = self.view_now();
            self.enter(yaw_of(e, l), inp, w);
        }

        /// One frame of KH1's lock camera, mode 3 (`0x1401E2650`).
        pub fn lock_step(&mut self, inp: &Input, t: &Target, w: &mut dyn World) {
            use lockcam::*;
            let mut anchor = inp.pos + (ANCHOR + Vec3::new(0.0, self.lock.height, 0.0)).rotate_yaw(self.yaw);
            let mut v = t.point - anchor;
            let dy = v.y;
            v.y = 0.0;
            let (n, _) = v.normalize_len();
            let mut v = n * (t.radius + PAST);
            v.y = dy * -0.5;
            let past = t.point + v;
            let wv = past - anchor;
            let hh = wv.horizontal_length();
            let a = wv.y.abs();
            if hh < a + a {
                let mut y = wv.y;
                if a < hh {
                    y *= ((a + a) - hh) / a;
                }
                anchor.y += y * 0.5;
            }
            // The eye closes on the anchor: 90 % of the horizontal gap past 75, 90 % vertically.
            let mut d = anchor - self.eye;
            let dyy = d.y;
            d.y = 0.0;
            let (n, len) = d.normalize_len();
            let len = len - DEAD;
            if 0.0 < len {
                self.eye += n * (len * CLOSE);
            }
            self.eye.y += dyy * CLOSE;
            // A ray from Sora's body point to the eye.
            let body = inp.pos + Vec3::new(0.0, -BODY, 0.0);
            let blocked = w.sweep(body, self.eye, RAY);
            if let Some(h) = blocked {
                self.eye = h;
            }
            // The yaw: half the error outside a ±5° band about the line to a point ahead of
            // Sora, at most 10° a frame.
            let ahead = inp.pos + AHEAD.rotate_yaw(self.yaw);
            let a1 = wrap_angle(self.yaw - yaw_of(self.eye, ahead));
            let mut a2 = wrap_angle(yaw_of(self.eye, past) - self.yaw);
            if a1 + BAND < a2 {
                a2 -= a1 + BAND;
            } else if a2 < a1 - BAND {
                a2 += BAND - a1;
            } else {
                a2 = 0.0;
            }
            let turn = (a2 * 0.5).max(-TURN_MAX).min(TURN_MAX);
            self.yaw = wrap_angle(turn + self.yaw);
            let to = past - self.eye;
            self.look = self.eye + Vec3::new(0.0, to.y, -to.horizontal_length()).rotate_yaw(self.yaw);
            self.yaw = yaw_of(self.eye, self.look);
            // Keep Sora on screen: his feet outside the half FOV raise a counter (to 40) that pulls
            // the look point up to 80 % toward his body point; it falls 2 a frame inside 80 %.
            let half = FOV * PI / 180.0 * 0.5;
            let ang = angle_between(self.look - self.eye, inp.pos - self.eye);
            let pull = if ang.abs() <= half {
                if ang.abs() < half * 0.8 {
                    self.lock.screen -= inp.step * 2.0;
                    if self.lock.screen < 0.0 {
                        self.lock.screen = 0.0;
                    }
                }
                0.0 < self.lock.screen
            } else {
                self.lock.screen = (self.lock.screen + inp.step).min(SCREEN_MAX);
                true
            };
            if pull && 0.0 < self.lock.screen {
                let k = (self.lock.screen / SCREEN_MAX) * SCREEN_PULL;
                self.look = body * k + self.look * (1.0 - k);
            }
            if 0.0 < self.lock.relock {
                self.lock.relock -= inp.step;
                if 0.0 <= self.lock.relock {
                    let c = ((self.lock.relock / RELOCK) * PI * 0.5).cos();
                    self.look = self.look * c + self.lock.old_look * (1.0 - c);
                } else {
                    self.lock.relock = 0.0;
                }
            }
            let skip = matches!(inp.state, 4 | 5 | 8 | 0x15 | 0x18 | 0x1A);
            if !skip && blocked.is_none() {
                // `0x1401E5A20`: the floor under the eye (a ray from 10 above it, 10000 down).
                let from = self.eye + Vec3::new(0.0, -10.0, 0.0);
                let floor = w.sweep(from, self.eye + Vec3::new(0.0, FLOOR_PROBE, 0.0), RAY).unwrap_or(self.eye + Vec3::new(0.0, FLOOR_PROBE, 0.0));
                let over = inp.pos.y - floor.y;
                self.lock.height = if over <= 30.0 {
                    let up = t.point.y - self.eye.y;
                    if -200.0 <= up {
                        0.0
                    } else {
                        ((up.max(-400.0) + 200.0).abs() / 200.0) * 20.0
                    }
                } else {
                    -((over.min(150.0) - 30.0).abs() / 120.0) * 170.0
                };
            }
            self.prev = Some(inp.pos);
        }
    }

    /// The angle between two vectors (`0x1402A91F0`: acos of the normalized dot, 0 / π past ±1).
    pub fn angle_between(a: Vec3, b: Vec3) -> f32 {
        let (la, lb) = (a.length(), b.length());
        if la == 0.0 || lb == 0.0 {
            return 0.0;
        }
        let c = a.dot(b) / (la * lb);
        if 1.0 < c {
            0.0
        } else if c < -1.0 {
            PI
        } else {
            c.acos()
        }
    }
}
