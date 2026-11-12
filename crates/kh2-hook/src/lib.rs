//! `kh2-hook`: Kingdom Hearts 1's combat inside Kingdom Hearts II Final Mix (PC, KH 1.5+2.5 ReMIX,
//! Steam build first).
//!
//! Panacea (OpenKH's mod loader) loads every DLL in `<mods>\kh2\dll\` into KH2 only and calls
//! `OnInit`. Everything we add goes into KH2's own systems (`docs/design-philosophy-instructions.md`).
//!
//! Stage 1 (now): load, write `kh2-kh1.log`, note the game build. No hooks, nothing in the game
//! is changed.

#![allow(clippy::missing_safety_doc)]

/// The mod's version as the user sees it (the log): the stage of the latest `AGENTS.md` entry,
/// bumped with every change to the source (a test checks). `t` for the testing branch.
pub const VERSION: &str = "t1";

#[cfg(all(windows, target_arch = "x86_64"))]
mod win;

#[cfg(all(windows, target_arch = "x86_64"))]
pub use win::*;
