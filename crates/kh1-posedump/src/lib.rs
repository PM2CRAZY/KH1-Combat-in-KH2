//! `kh1-posedump`: a test tool, not part of the mod. Loaded into KH1 (`KINGDOM HEARTS FINAL
//! MIX.exe`, Steam), it records Sora's bone matrices right after KH1's own skeleton pass, so the
//! offline KH1 reader (`kh1_data::rig`) can be checked against KH1 frame for frame. F10 starts
//! and stops a recording into `kh1-posedump.bin` beside the exe (format below); a log goes to
//! `kh1-posedump.log`.
//!
//! The pass (Steam, link stamp `0x669E37CE`): `0x140391C70 (world matrix, instance, time f32,
//! motion)` samples the motion and runs the bone pass `0x1403900B0`. Checked (decomp, exe bytes).
//! The instance: `**instance` the bone count (u16); `+0x1B0` / `+0x1B4` / `+0x1B8` handles of the
//! sampled channels / world matrices / local matrices (0x40 bytes a bone). A handle `h` is
//! `table[(h & 0x7FFFFFFF) >> 25] | (h & 0x1FFFFFF)`, the table at `0x142EE3980` (`0x14038AF40`).
//! The motion: `+0x14` the joint words' handle (one u32 a bone). Checked (decomp).
//!
//! Sora's pass with ground IK, `0x140392560 (world matrix, instance, flags)`, is recorded too
//! (its motion and time read from the instance's `+8` / `+0x10`). Checked (decomp, exe bytes).
//!
//! File: `KH1POSE1`, then records. Motion (once each): `1u32, motion u64, 0x40 header bytes,
//! bones u32, joint words`. Frame: `2u32` (the plain pass) or `3u32` (the ground-IK pass), instance u64, motion u64, time f32, bones u32, the
//! world matrix argument (16 f32), world matrices (bones × 16 f32), local matrices (the same)`.
//! Matrices as KH1 keeps them (row-vector, glam column-major memory; KH1's world is y-down).

#[cfg(all(windows, target_arch = "x86_64"))]
mod win;

#[cfg(all(windows, target_arch = "x86_64"))]
pub use win::*;
