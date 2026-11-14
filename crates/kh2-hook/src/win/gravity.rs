//! KH1's Gravity damage as a KH2 attack type of ours (`magic_rows::GRAVITY_TYPE`): KH2's damage
//! function `0x1403C12F0(target status, attack, ATKP row, which)` (Checked (decomp): ATKP `+4`
//! the type, `+6` the power; status `+0` HP, `+0x1AE` the general resistance) computes it KH1's
//! way for that type, KH2's own way for every other.

use super::{hook, log};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const DAMAGE: usize = 0x3C_12F0;
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
    let o: unsafe extern "system" fn(*const i32, usize, *const u8, i32) -> i32 = std::mem::transmute(O_DAMAGE.load(Ordering::Acquire));
    o(status, attack, atkp, which)
}

pub unsafe fn install(base: usize) {
    hook::install(base + DAMAGE, &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x40], damage as *const () as usize, &O_DAMAGE, "damage 0x1403C12F0");
}
