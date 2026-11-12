# The asset build step (plan, not started)

Goal (the owner's direction): KH1's motions, effects and sounds are made once from the player's own
extracted KH1 and KH2 files into the mod folder, and KH2 loads them as its own files. Nothing of
the games' goes into the repo. Rebuilt only when the mod version or the source files change.

## What KH2 loads for Sora (from the player's extracted files, `local\re`)

`P_EX100.mset` / `.mdlx` (Sora's motions and model; the forms are `P_EX100_<FORM>`), `W_EX010.mset`
(the Keyblade's motions, with the hitbox windows), the spells' `.mag` files, and the tables in
`00battle.bin` / `03system.bin` (`docs/kh2-tables.md`). Checked (files). KH2 Final Mix also ships
`P_EX100_KH1F.mdlx` / `.mset` and `W_EX010_KH1F.mset`, Sora's KH1-outfit form: a lead worth reading
before building motions (Assumption: what its motions are).

## What KH1 has

KH1's Sora is `PLAYER_0002` / `PL0002` (`.MDL`, `.ANN` motions, `.EET` effects, `.SES` sounds) in the
player's KH1 extraction (`local\re\kh1`, `kh1-extract`). Checked (files).

## Steps (each a version, planned with the owner first)

1. Read KH1's `PL0002.ANN` motions and write them as KH2 motion entries (KH2's ANB format) in a
   KH2 `.mset` the game loads in place of Sora's, keeping every KH2 motion KH2 still needs (forms,
   reactions, Limits, events).
2. KH1's per-hit data as `atkp` rows and KH1's combo order as `ptya` rows, written into a copy of
   `00battle.bin` in the mod folder.
3. KH1's effects (`.EET`) as KH2 effect programs, and KH1's sounds (`.SES`) as a KH2 sound bank.

## Decided (the owner, 2026-11-12)

- The build runs in the DLL on first launch (answer 1a), reading the KH1 install directly (2a).
- The sister project's format readers are reused ("feel free to copy the code"): `kh2-formats`,
  `kh2-anim`, `kh1-data`, copied from `KH2-Combat-in-KH1` testing (t10). Its
  `bake::continuous_angles_take_the_short_way` test fails there too (its KH2→KH1 bake, unused
  here): marked ignored, not fixed.
- The built files go into the OpenKH mod folder (`openkh\mod\kh2\`, which Panacea maps into
  KH2's packages through `patch-package-map.txt`: Checked (files), the DLL's own line
  `dll/kh2-kh1.dll $$$$ kh2_first/original/dll/kh2-kh1.dll`).

## The readers on the owner's install (t10, `kh1-assets` tool, Checked (files))

Both games' packages are in `Image\dt\`. KH1 Sora's `xa_ex_0010.mset`: 118 motions, 7,440 frames
in all; `xa_ex_0010.mdls`: 293 bones. KH2 Sora's `obj/P_EX100.mset`: 839 entries, 2.6 MB;
`obj/P_EX100.mdlx`: 228 bones. Read in 0.15 s.

## Next: the motion converter

KH1 motion → KH2 motion: KH1's pose each frame (`kh1_data::mset::Motion::apply` on KH1's
skeleton), retargeted onto KH2 Sora's 228 bones (the sister project's `kh1_data::retarget` joint
map, run the other way), written as a KH2 motion entry (KH2's ANB) appended to `P_EX100.mset`.
To read first: how KH2 plays a motion entry (interpolated or raw, `kh2_formats::anim` /
`motion`) and which form costs KH2 least to play.
