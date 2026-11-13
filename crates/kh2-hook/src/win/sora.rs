//! Which object is Sora, and whether he is base Sora (not in a Drive Form). Shared by the
//! movement systems.
//!
//! - Sora: KH2 keeps him at `0x142AE9A28` (`0x140405030` stores, `0x140405180` reads). Checked
//!   (decomp).
//! - Base Sora: his movement block's `prty` row pointer `+0x148` (`0x14040A480` stores it with the
//!   row it loads) is `prty` row 1, Sora's own. A Drive Form loads its own row (the owner,
//!   2026-11-12: Drive Forms keep KH2's jump, answer 2a). Checked (decomp) for the field;
//!   Assumption: every form loads a row other than 1 (the log says when an object is skipped).

use std::sync::atomic::{AtomicUsize, Ordering};

const SORA_PTR: usize = 0x2AE_9A28;
const PRTY: usize = 0x2AE_5780;

static BASE: AtomicUsize = AtomicUsize::new(0);

pub fn set_base(base: usize) {
    BASE.store(base, Ordering::Relaxed);
}

pub fn base() -> usize {
    BASE.load(Ordering::Relaxed)
}

pub fn is_sora(actor: usize) -> bool {
    actor != 0 && unsafe { *((base() + SORA_PTR) as *const usize) } == actor
}

/// `prty` row `n`'s address (0 if there is none).
pub fn prty_row(n: usize) -> usize {
    unsafe {
        let table = *((base() + PRTY) as *const usize);
        if table == 0 || n >= *(table as *const i32) as usize {
            return 0;
        }
        let off = *((table + 4 * (n + 1)) as *const i32) as usize;
        if off == 0 { 0 } else { table + off }
    }
}

/// Sora, on his own `prty` row (not a Drive Form).
pub fn is_base_sora(actor: usize) -> bool {
    is_sora(actor) && {
        let row = prty_row(1);
        row != 0 && unsafe { *((actor + 0x148) as *const usize) } == row
    }
}

/// Sora's object now (0 when there is none).
pub fn current() -> usize {
    if base() == 0 {
        return 0;
    }
    unsafe { *((base() + SORA_PTR) as *const usize) }
}
