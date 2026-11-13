//! KH1's follow camera in KH2's own field camera (`camera = kh1`).
//!
//! KH2 (Checked (decomp), Steam; `docs/camera.md`): the field camera's update `0x140166100` runs
//! its mode (`camera + 0x48`); mode 0 is the follow `0x140165110 (camera)`; then `0x140165700`
//! springs the drawn look `+0x08` toward its goal `+0x84` (previous goal `+0x94`), the drawn eye `+0x18`
//! toward `+0x64` (previous goal `+0x74`), checks the eye against the walls for mode 0, and hands the
//! view to the renderer. The subject is `camera + 0x50` (Sora).
//!
//! With `camera = kh1`, mode 0 on Sora is KH1's follow camera (`crate::kh1cam::kh1::Follow`, from
//! the owner's KH1-in-ReCom, every number from the KH1 Steam decomp): KH2's follow function runs
//! it instead of its own and writes its eye and look as KH2's goals and drawn view (previous goals
//! the same, so KH2's spring keeps them where KH1 put them). Its walls are KH2's own segment test
//! `0x140171210 (segment, hit, face, object)` (Checked (decomp): the start and end vec4s, the hit
//! vec4). Its inputs are KH2's: Sora's position `+0x70`, facing (`0x1403B56E0`), velocity `+0x20`,
//! in the air (`0x1403BA9A0`), the pad (`0x14039B780(0)`: the right stick's unit direction
//! `+0x20` / `+0x24` and tilt `+0x2C`, the reset bit 17 of `+8`, the follow's own reads), KH2's
//! Field Camera option (`0x1402E5310`, Auto) and its camera reverse options (`0x1402E5330` /
//! `0x1402E5350`), the frame step `0x140717480`, keyboard and mouse `0x1404E5C50`.
//! Runs once a frame while KH2's follow camera does.
//!
//! KH2's lock-on camera (mode 2, `0x140176C70`; Checked (decomp)) is KH1's lock camera the same
//! way. KH2's lock-on start `0x140176910 (target)` sets mode 2 and keeps the target record at
//! `0x140ABE540` (the enemy's handle, its target part, a special target's handle); KH2's
//! `0x1403BE000 (record, out)` gives the target's point (`out + 0x18`) and its part's radius
//! (`out + 4`). KH1's lock-on start (a 60-frame blend in, a 60-frame re-lock on a new target) and
//! unlock (a 45-frame blend out) come from the first frame in mode 2, a change of target and the
//! first frame back in mode 0.
//!
//! KH1's FOV (53°, `crate::kh1cam::kh2_fov`) is KH2's FOV goal `+0x38` (copied to `+0xA4` after
//! the mode) and its drawn FOV `+0x28` while KH1's camera has the view. KH2's own follow and first
//! person put `+0x38` back from the camera's base FOV `+0xE0` each frame (Checked (decomp):
//! `0x140165110`, `0x140166100` mode 1), so `camera = kh2` keeps KH2's 1.5.

use super::hook;
use super::log;
use super::sora::is_sora;
use crate::kh1cam::kh1::{self, Follow, Input, World};
use crate::kh1cam::math::Vec3;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

const FOLLOW: usize = 0x16_5110;
const SEGMENT: usize = 0x17_1210;
const FACING: usize = 0x3B_56E0;
const IN_AIR: usize = 0x3B_A9A0;
const PAD: usize = 0x39_B780;
const OPT_AUTO: usize = 0x2E_5310;
const OPT_REV_H: usize = 0x2E_5330;
const OPT_REV_V: usize = 0x2E_5350;
const KBM: usize = 0x4E_5C50;
const STEP: usize = 0x71_7480;
const HANDLE: usize = 0x4A_D270;
const LOCK: usize = 0x17_6C70;
const LOCK_TARGET: usize = 0x3B_E000;
const LOCK_RECORD: usize = 0xAB_E540;

static BASE: AtomicUsize = AtomicUsize::new(0);
static O_FOLLOW: AtomicUsize = AtomicUsize::new(0);
static O_LOCK: AtomicUsize = AtomicUsize::new(0);
/// KH1's lock camera has the view (mode 2), and the target record it locked on.
static LOCKED: Mutex<Option<[u32; 3]>> = Mutex::new(None);
static LOCK_LOGGED: AtomicBool = AtomicBool::new(false);
static CAM: Mutex<Option<Follow>> = Mutex::new(None);
/// When we last stepped, to know when the camera comes back to us (another mode or an event
/// had it): more than a few frames without us is a fresh entry.
static LAST: Mutex<Option<std::time::Instant>> = Mutex::new(None);
const FRESH_AFTER: std::time::Duration = std::time::Duration::from_millis(100);
static LOGGED: AtomicBool = AtomicBool::new(false);

unsafe fn f<T: Copy>(off: usize) -> T {
    std::mem::transmute_copy(&(BASE.load(Ordering::Relaxed) + off))
}

unsafe fn v4(at: usize) -> Vec3 {
    let p = at as *const f32;
    Vec3::new(*p, *p.add(1), *p.add(2))
}

unsafe fn put4(at: usize, v: Vec3) {
    let p = at as *mut f32;
    *p = v.x;
    *p.add(1) = v.y;
    *p.add(2) = v.z;
    *p.add(3) = 1.0;
}

/// KH2's background segment test as KH1's camera collision.
struct Kh2World;

impl World for Kh2World {
    fn sweep(&mut self, from: Vec3, to: Vec3, radius: f32) -> Option<Vec3> {
        unsafe {
            let seg: [f32; 8] = [from.x, from.y, from.z, 1.0, to.x, to.y, to.z, 1.0];
            let mut hit = [0.0f32; 4];
            let mut face = [0u16; 4];
            let mut obj = 0usize;
            let test: unsafe extern "system" fn(*const f32, *mut f32, *mut u16, *mut usize) -> u8 = f(SEGMENT);
            if test(seg.as_ptr(), hit.as_mut_ptr(), face.as_mut_ptr(), &mut obj) == 0 {
                return None;
            }
            // Back from the wall by the sphere's radius (Assumption: a ray stands for KH1's sweep).
            let h = Vec3::new(hit[0], hit[1], hit[2]);
            let (dir, len) = (h - from).normalize_len();
            Some(from + dir * (len - radius).max(0.0))
        }
    }

    /// KH1's flat wall probe `0x140196300` as eight flat rays of `radius` from `at` (Assumption:
    /// eight rays stand for KH1's circle, as in KH1-in-ReCom).
    fn near_wall(&mut self, at: Vec3, radius: f32) -> bool {
        (0..8).any(|k| {
            let a = k as f32 * std::f32::consts::FRAC_PI_4;
            self.sweep(at, at + Vec3::new(a.sin() * radius, 0.0, a.cos() * radius), 0.0).is_some()
        })
    }
}

unsafe fn input(cam: usize, sora: usize) -> Input {
    let pad: unsafe extern "system" fn(i32) -> usize = f(PAD);
    let p = pad(0);
    let (dx, dy, len) = (*((p + 0x20) as *const f32), *((p + 0x24) as *const f32), *((p + 0x2C) as *const f32));
    let facing_of: unsafe extern "system" fn(usize, *mut [f32; 4]) -> *const [f32; 4] = f(FACING);
    let mut buf = [0.0f32; 4];
    let fv = facing_of(sora, &mut buf);
    let facing = if fv.is_null() { 0.0 } else { (*fv)[0].atan2((*fv)[2]) };
    let in_air: unsafe extern "system" fn(usize) -> u8 = f(IN_AIR);
    let opt = |o: usize| -> bool {
        let g: unsafe extern "system" fn() -> u8 = f(o);
        g() & 1 != 0
    };
    let kbm: unsafe extern "system" fn() -> i32 = f(KBM);
    // The follow's own "may turn" test: the subject's handle object's vtable +0x78.
    let subj = *((cam + 0x50) as *const usize);
    let handle: unsafe extern "system" fn(u32) -> usize = f(HANDLE);
    let o = if subj != 0 { handle(*(subj as *const u32)) } else { 0 };
    let can_act = o != 0 && {
        let vt = *(o as *const usize);
        let g: unsafe extern "system" fn(usize, usize) -> u8 = std::mem::transmute(*((vt + 0x78) as *const usize));
        g(o, subj) != 0
    };
    Input {
        pos: v4(sora + 0x70),
        facing,
        stick: [dx * len, dy * len],
        mouse: kbm() != 0,
        can_act,
        recenter: (*((p + 8) as *const u32) >> 17) & 1 != 0,
        // KH1's Normal is (+1 left / right, −1 up / down); KH2's reverse options flip them
        // (Assumption: KH2's Normal turns as KH1's).
        inv: [if opt(OPT_REV_H) { -1.0 } else { 1.0 }, if opt(OPT_REV_V) { 1.0 } else { -1.0 }],
        step: *((BASE.load(Ordering::Relaxed) + STEP) as *const f32),
        auto: opt(OPT_AUTO),
        state: if in_air(sora) != 0 { 2 } else { 0 },
        vel: v4(sora + 0x20),
    }
}

unsafe extern "system" fn follow(cam: usize) {
    let o: unsafe extern "system" fn(usize) = std::mem::transmute(O_FOLLOW.load(Ordering::Acquire));
    let sora = *((cam + 0x50) as *const usize);
    if cam == 0 || sora == 0 || !is_sora(sora) {
        return o(cam);
    }
    let now = std::time::Instant::now();
    let fresh = LAST.lock().map(|mut l| l.replace(now).is_none_or(|t| now.duration_since(t) > FRESH_AFTER)).unwrap_or(true);
    let inp = input(cam, sora);
    let mut w = Kh2World;
    let Ok(mut g) = CAM.lock() else { return o(cam) };
    let c = g.get_or_insert_with(Follow::default);
    // KH2's cut (`+0x41` = 1: a new room or an event's end) and a return to the follow: KH1's
    // re-place behind Sora; else the entry at the drawn view's yaw.
    let unlocked = LOCKED.lock().map(|mut l| l.take().is_some()).unwrap_or(false);
    if *((cam + 0x41) as *const u8) == 1 {
        *((cam + 0x41) as *mut u8) = 0;
        c.behind(&inp, &mut w);
    } else if unlocked && !fresh {
        // KH1's unlock `0x1401E4110`: the follow with a 45-frame blend.
        c.unlock(&inp, &mut w);
    } else if fresh {
        let (e, l) = (v4(cam + 0x18), v4(cam + 0x08));
        c.enter(kh1::yaw_of(e, l), &inp, &mut w);
    }
    c.step(&inp, &mut w);
    put_view(cam, c, inp.step);
    if !LOGGED.swap(true, Ordering::Relaxed) {
        log!("camera kh1: KH1's follow camera on KH2's mode 0 (first eye {:?}, look {:?}; KH2's FOV {}, KH1's {})", (c.eye.x, c.eye.y, c.eye.z), (c.look.x, c.look.y, c.look.z), KH2_FOV_SEEN.lock().map(|g| *g).unwrap_or(0.0), crate::kh1cam::kh2_fov(kh1::FOV));
    }
}

static KH2_FOV_SEEN: Mutex<f32> = Mutex::new(0.0);

/// KH1's drawn view as KH2's goals and drawn view; the goals' previous values (`+0x74`, `+0x94`)
/// the same, as KH2's own snap `0x140164D70` leaves them, so KH2's spring `0x140165BD0` keeps the
/// view where KH1 put it. KH1's FOV the same way (`+0x38` goal, `+0xA4` / `+0xB4`, `+0x28`).
unsafe fn put_view(cam: usize, c: &mut Follow, step: f32) {
    let (eye, look) = c.view(step);
    for at in [0x64usize, 0x74, 0x18] {
        put4(cam + at, eye);
    }
    for at in [0x84usize, 0x94, 0x08] {
        put4(cam + at, look);
    }
    let fov = crate::kh1cam::kh2_fov(kh1::FOV);
    let seen = *((cam + 0x38) as *const f32);
    if seen != fov {
        if let Ok(mut g) = KH2_FOV_SEEN.lock() {
            *g = seen;
        }
    }
    for at in [0x38usize, 0xA4, 0xB4, 0x28] {
        *((cam + at) as *mut f32) = fov;
    }
    // KH2's own yaw fields follow the view (its lock-on and resets start from them).
    let yaw = kh1::yaw_of(eye, look);
    *((cam + 0x5C) as *mut f32) = yaw;
    *((cam + 0x60) as *mut f32) = yaw;
}

/// KH2's lock target: its point and its part's radius (`0x1403BE000`).
unsafe fn lock_target() -> (kh1::Target, [u32; 3]) {
    #[repr(C, align(16))]
    struct Out([u32; 12]);
    let mut out = Out([0; 12]);
    let rec = BASE.load(Ordering::Relaxed) + LOCK_RECORD;
    let get: unsafe extern "system" fn(usize, *mut u32) = f(LOCK_TARGET);
    get(rec, out.0.as_mut_ptr());
    let r = rec as *const u32;
    let p = out.0.as_ptr() as *const f32;
    (kh1::Target { point: Vec3::new(*p.add(6), *p.add(7), *p.add(8)), radius: (*p.add(1)).max(0.0) }, [*r, *r.add(1), *r.add(2)])
}

unsafe extern "system" fn lock(cam: usize) {
    let o: unsafe extern "system" fn(usize) = std::mem::transmute(O_LOCK.load(Ordering::Acquire));
    let sora = if cam == 0 { 0 } else { *((cam + 0x50) as *const usize) };
    if sora == 0 || !is_sora(sora) {
        return o(cam);
    }
    let now = std::time::Instant::now();
    let fresh = LAST.lock().map(|mut l| l.replace(now).is_none_or(|t| now.duration_since(t) > FRESH_AFTER)).unwrap_or(true);
    let inp = input(cam, sora);
    let mut w = Kh2World;
    let (t, rec) = lock_target();
    if t.point == Vec3::ZERO {
        // No target point (KH2's record gives none): KH2's own lock camera this frame.
        return o(cam);
    }
    let Ok(mut g) = CAM.lock() else { return o(cam) };
    let c = g.get_or_insert_with(Follow::default);
    let Ok(mut locked) = LOCKED.lock() else { return o(cam) };
    if fresh {
        // Back from another mode or an event: KH1's follow at the drawn view first.
        let (e, l) = (v4(cam + 0x18), v4(cam + 0x08));
        c.enter(kh1::yaw_of(e, l), &inp, &mut w);
        *locked = None;
    }
    match *locked {
        // KH1's lock-on start `0x1401E41A0`: a new target while locked re-locks over 60 frames.
        Some(was) if was == rec => {}
        Some(_) => c.lock_on(true),
        None => c.lock_on(false),
    }
    *locked = Some(rec);
    c.lock_step(&inp, &t, &mut w);
    put_view(cam, c, inp.step);
    if !LOCK_LOGGED.swap(true, Ordering::Relaxed) {
        log!("camera kh1: KH1's lock camera on KH2's mode 2 (target {:?}, radius {})", (t.point.x, t.point.y, t.point.z), t.radius);
    }
}

pub unsafe fn install(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    if hook::install(base + FOLLOW, &[0x48, 0x89, 0x5C, 0x24, 0x08], follow as *const () as usize, &O_FOLLOW, "follow camera 0x140165110").is_some() {
        log!("camera kh1: KH1's follow camera with camera = kh1");
    }
    if hook::install(base + LOCK, &[0x48, 0x89, 0x5C, 0x24, 0x08], lock as *const () as usize, &O_LOCK, "lock-on camera 0x140176C70").is_some() {
        log!("camera kh1: KH1's lock camera with camera = kh1");
    }
}
