//! KH2's magic-cast trigger (motion frame trigger 17, `0x14040DD70` case 0x11) hands Sora's spell
//! record (`+0xDA0`, set at the cast by `0x1403A9DD0`) to `0x1403C6150`, which reads the record's
//! caster and spell handles. KH2 frees a record once its spell's code has finished
//! (`0x1403C55F0`, each frame, unlinking it from the live list `0x142A23A70` → `+0xC` next) and
//! never clears Sora's `+0xDA0`: a spell of ours whose code ended before its release frame left a
//! freed record there (the owner's Aero crash, t94: `0x1403C6161` reading `0x6C7`). Our spells now
//! hold their cast thread to the release (`kh1_assets::spells`); this guard runs KH2's function only
//! for a record still in KH2's live list, and logs the first one it skips. Checked (decomp).

use super::{hook, log};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const RELEASE: usize = 0x3C_6150;
const LIVE: usize = 0x2A2_3A70;
const HANDLE: usize = 0x4A_D270;
static BASE: AtomicUsize = AtomicUsize::new(0);
static O_RELEASE: AtomicUsize = AtomicUsize::new(0);
static LOGGED: AtomicBool = AtomicBool::new(false);

pub unsafe fn install(base: usize) {
    BASE.store(base, Ordering::Relaxed);
    // 40 53 push rbx; 48 83 EC 20 sub rsp, 0x20.
    hook::install(base + RELEASE, &[0x40, 0x53, 0x48, 0x83, 0xEC, 0x20], release as *const () as usize, &O_RELEASE, "magic release 0x1403C6150");
}

unsafe extern "C" fn release(record: usize) {
    let o: unsafe extern "C" fn(usize) = std::mem::transmute(O_RELEASE.load(Ordering::Acquire));
    let base = BASE.load(Ordering::Relaxed);
    let resolve: unsafe extern "C" fn(u32) -> usize = std::mem::transmute(base + HANDLE);
    let mut r = *((base + LIVE) as *const usize);
    let mut n = 0;
    while r != 0 && r != usize::MAX && n < 256 {
        if r == record {
            return o(record);
        }
        r = resolve(*((r + 0xC) as *const u32));
        n += 1;
    }
    if !LOGGED.swap(true, Ordering::Relaxed) {
        log!("magic: a cast's release found its spell already finished (record {record:#x}); skipped as KH2 would for no spell");
    }
}
