//! KH1's Gravity damage and Stop as KH2 attack types of ours (`magic_rows::GRAVITY_TYPE`): KH2's damage
//! function `0x1403C12F0(target status, attack, ATKP row, which)` (Checked (decomp): ATKP `+4`
//! the type, `+6` the power; status `+0` HP, `+0x1AE` the general resistance) computes it KH1's
//! way for that type, KH2's own way for every other.

use super::{hook, log};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const DAMAGE: usize = 0x3C_12F0;
/// Handle → object (`0x1404AD270`) and KH2's stop of an object (`0x1403B6730(object, id,
/// frames)`, the time in `xmm2`: `0x1403B4383`; Checked (decomp + exe)).
const FROM_HANDLE: usize = 0x4A_D270;
const STOP_OBJECT: usize = 0x3B_6730;
static BASE: AtomicUsize = AtomicUsize::new(0);
static STOP_LOGGED: AtomicBool = AtomicBool::new(false);
static O_DAMAGE: AtomicUsize = AtomicUsize::new(0);
static LOGGED: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn damage(status: *const i32, attack: usize, atkp: *const u8, which: i32) -> i32 {
    if !atkp.is_null() && *atkp.add(4) == crate::magic_rows::GRAVITY_TYPE {
        let hp = *status;
        let power = u16::from_le_bytes([*atkp.add(6), *atkp.add(7)]) as i32;
        let bars = super::mp::max_bars();
        let general = *(status as *const u8).add(0x1AE);
        let d = crate::magic_rows::gravity_damage(hp, power, bars, general);
        if !LOGGED.swap(true, Ordering::Relaxed) {
            log!("magic: KH1's Gravity: {d} of HP {hp} (power {power} + 5 x {bars} bars %, resistance {general})");
        }
        return d;
    }
    if !atkp.is_null() && *atkp.add(4) == crate::magic_rows::STOP_TYPE {
        // KH1's Stop: KH2's own stop on the struck object (status `+0x268` its handle).
        let base = BASE.load(Ordering::Relaxed);
        let obj: extern "system" fn(u32) -> usize = std::mem::transmute(base + FROM_HANDLE);
        let o = obj(*status.add(0x9A) as u32);
        if o != 0 {
            let frames = crate::magic_rows::stop_frames(super::mp::max_bars());
            let stop: extern "system" fn(usize, i32, f32) = std::mem::transmute(base + STOP_OBJECT);
            stop(o, crate::magic_rows::STOP_ID as i32, frames);
            if !STOP_LOGGED.swap(true, Ordering::Relaxed) {
                log!("magic: KH1's Stop on {o:#x} for {frames} frames (KH2's stop kind {:#x})", crate::magic_rows::STOP_ID);
            }
        }
        return 0;
    }
    let o: unsafe extern "system" fn(*const i32, usize, *const u8, i32) -> i32 = std::mem::transmute(O_DAMAGE.load(Ordering::Acquire));
    o(status, attack, atkp, which)
}

pub unsafe fn install(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    hook::install(base + DAMAGE, &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x40], damage as *const () as usize, &O_DAMAGE, "damage 0x1403C12F0");
}
