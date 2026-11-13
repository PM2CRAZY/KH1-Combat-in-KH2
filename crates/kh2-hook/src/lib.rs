//! `kh2-hook`: Kingdom Hearts 1's combat inside Kingdom Hearts II Final Mix (PC, KH 1.5+2.5 ReMIX,
//! Steam build first).
//!
//! Panacea (OpenKH's mod loader) loads every DLL in `<mods>\kh2\dll\` into KH2 only and calls
//! `OnInit`. Everything we add goes into KH2's own systems (`docs/design-philosophy-instructions.md`).
//!
//! Now (t2): load, write `kh2-kh1.log`, tell the Steam and Epic builds apart, read `kh2-kh1.ini`
//! (one KH2 / KH1 toggle per system), the byte-checked hook installer, and the debug trace of
//! Sora (`win/trace.rs`, read only); with `movement = kh1`, KH1's jump and fall arc on Sora
//! (`win/arc.rs`); with `combos = kh1`, KH1's combo motions built into the mod folder on launch
//! (`kh1_assets`).

#![allow(clippy::missing_safety_doc)]

pub mod kh1;
pub mod settings;

/// The mod's version as the user sees it (the log): the stage of the latest `AGENTS.md` entry,
/// bumped with every change to the source (a test checks). `t` for the testing branch.
pub const VERSION: &str = "t53";

#[cfg(all(windows, target_arch = "x86_64"))]
mod win;

#[cfg(all(windows, target_arch = "x86_64"))]
pub use win::*;
