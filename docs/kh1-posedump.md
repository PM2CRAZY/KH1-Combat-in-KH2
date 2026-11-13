# kh1-posedump: KH1's own poses as ground truth

A test tool, **not part of the mod** (`crates/kh1-posedump`, built by CI into
`tools/kh1-posedump/kh1-posedump.dll`). It runs inside KH1 and records Sora's bone matrices
right after KH1's own skeleton pass. The offline KH1 reader (`kh1_data::rig`), which the KH1-to-KH2
motion converter uses, is checked against those recordings frame for frame.

Why (owner, t17): KH1's swings on KH2 Sora fold him forward ("stretched like CRAZY"). The KH1
clip shows KH1 keeps his upper body upright. The motion writer is right (the t17 round trip), and
KH1's helper math matches the decomp, so the bone pass in `rig.rs` is the suspect, and reading
the decomp alone left it ambiguous.

## Use

1. Turn the KH2-combat-in-KH1 mod off for the recording: it redraws Sora after KH1's pass, but
   the tool records straight after KH1's pass, before any mod changes it. Turning it off keeps KH1
   vanilla in every other way too.
2. Enable the OpenKH mod `PM2CRAZY/KH1-PoseDump` for KH1 (its `mod.yml` copies
   `kh1-posedump.dll` to `dll/`) and Build. A DLL put straight into `mod\kh1\dll\` is lost:
   OpenKH rebuilds `mod\kh1` from the enabled mods (it removed ours, owner's PC, t19).
3. In KH1, press F10, do the ground combo and the air combo (and anything else to check), then
   press F10 again.
4. The recording is `kh1-posedump.bin` beside `KINGDOM HEARTS FINAL MIX.exe`; the log
   `kh1-posedump.log` says whether both hooks went in and how many frames were written.

## What it hooks (Steam KH1, link stamp `0x669E37CE`)

- `0x140391C70 (world matrix, instance, time f32, motion)`: samples the motion and runs the bone
  pass `0x1403900B0`. Prologue `4C 8B DC 53 56 41 54`. Checked (decomp, exe bytes). Recorded as
  frame tag 2.
- `0x140392560 (world matrix, instance, flags)`: the ground-IK pass after it (Sora's feet).
  Prologue `4C 8B DC 55 53 41 54 41 57`. Checked (decomp, exe bytes). Recorded as tag 3.
- Instance: `**instance` the bone count (Sora 293); `+8` the motion; `+0x10` the time; `+0x1B0`,
  `+0x1B4`, `+0x1B8` handles of the sampled channels, world matrices and local matrices (0x40 bytes
  a bone). A handle is `table[(h & 0x7FFFFFFF) >> 25] | (h & 0x1FFFFFF)`, the table at
  `0x142EE3980` (`0x14038AF40`). Checked (decomp).
- The sister project's notes give `0x140391E20` / `0x140392710` and the table at `0x142EE4730`:
  `0x1B0` / `0xDB0` off from this exe. Its bytes there are not these functions' prologues
  (Checked: exe bytes), so those notes are for another build of KH1.

## File

`KH1POSE1`, then records (little-endian):

- Motion, once per motion: `1u32`, motion pointer `u64`, the motion's first 0x40 bytes, bone
  count `u32`, its joint words (one `u32` a bone). These identify the motion in `xa_ex_0010.mset`.
- Frame: `2u32` or `3u32`, instance `u64`, motion `u64`, time `f32`, bone count `u32`, the world
  matrix argument (16 `f32`), the world matrices (bones × 16 `f32`), the local matrices (the same).
  Matrices are as KH1 keeps them: row-vector, which is glam's column-major memory. KH1's world is
  y-down.

## Not the cause of KH1's ability-menu crash (owner, t19)

KH1 crashed after the ability menu (`CrashDump.dmp`, 05:28): an access violation at exe offset
`0x2EBF10` (a null read). `ReFormAbilityToggle.dll`, another mod, is on the stack. Our tool hadn't
loaded (no `kh1-posedump.log`; OpenKH had removed it), so it wasn't involved. Checked (dump).
