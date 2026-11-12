//! Which KH2 exe we are in, from its PE link stamp (`docs/sources.md`, Checked (exe)).
//! New addresses are Steam-only for now: everything that patches the game checks
//! `Build::Steam` first.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Build {
    Steam,
    Epic,
    Unknown,
}

/// Steam `KINGDOM HEARTS II FINAL MIX.exe` (SHA-256 9002b2de…).
pub const STEAM_STAMP: u32 = 0x669E_384A;
/// Epic `KINGDOM HEARTS II FINAL MIX.exe` (SHA-256 7db77a7a…).
pub const EPIC_STAMP: u32 = 0x669E_44D8;

/// The PE header's `TimeDateStamp` of the module at `base`.
pub unsafe fn stamp(base: usize) -> u32 {
    let pe = base + *((base + 0x3C) as *const u32) as usize;
    *((pe + 8) as *const u32)
}

pub unsafe fn detect(base: usize) -> Build {
    match stamp(base) {
        STEAM_STAMP => Build::Steam,
        EPIC_STAMP => Build::Epic,
        _ => Build::Unknown,
    }
}
