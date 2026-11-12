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

## KH1's whole skeleton pass, offline (t11, `kh1_data::rig`)

KH1 Sora is mostly driven by 11 helpers (root, spine, legs and arms: aims and two-bone IK), so
the sister reader's keyed channels alone don't pose him. `rig::pose` runs KH1's pass as the KH1
Steam decomp shows it (motion start `0x14038F550`: bind, then the constant channels; helpers;
the bone pass `0x1403900B0` by joint word; the solver `0x14038FC20`), Checked (decomp) for the
steps. Left out offline: the ground lift of a foot target and the head's look-at (they need the
game's world). The multiply order is taken from what the solver needs to mean (Assumption; the
tests check the IK reaches its target with its lengths kept).

On the owner's files (`kh1-assets pose`, Checked (files)): the idle (motion 0) stands with the
legs on their foot helpers, about 145 tall; bones keep their lengths to 0.15 except bone 3,
which KH1 places at helper P6 (the spine's aim), so it stretches by design (up to 21 in 99..101).
Motions 98..101 travel forward with the travel helper H10 (98: 0 → 90 in 30 frames), the feet
helpers with them. Not compared with KH1's own pose in game yet.

## The motion converter (t12, `kh1_assets::convert`)

- KH1's pose each frame (`rig`), its travel (helper H10) taken off the body, the body onto KH2
  Sora's 228 bones (the sister project's `kh1_data::inverse`), each bone's local Euler / offset,
  written as a KH2 interpolated motion on KH2's A300 (motion 151) as the template: its reserved
  block, IK helper table (53), joint table (281), bounding box and root block kept; an initial
  pose of every bone's first-frame values; linear keys where a channel moves (Douglas–Peucker,
  0.002 rad / 0.05); no constraints or expressions (baked). KH2 keeps a motion's travel in its
  last IK helper (joint 52; A300's z curve ends at 150, Checked (files)): KH1's travel goes there.
  Interpolated, not raw: KH2's player motions are all interpolated, and a raw one would be
  ~0.9 MB a motion (228 matrices a frame).
- Checked (files), the owner's files (`tests/local_convert.rs`): the written motion read back and
  played by KH2's evaluator (`kh2_anim::evaluate`) matches the retargeted pose within 0.4 at every
  joint; the travel matches. Stick figures of the KH2 pose look as KH1's swing (a forward lunge).
- `build_mset`: KH2 Sora's `P_EX100.mset` with KH1's combo motions 98..104 added as KH2 motions
  210..216 (slot = motion × 4, the battle slot; the other three empty, as KH2's attacks). Its own
  839 slots are byte-identical in the rebuilt BAR (Checked (files)). Triggers: A300's for now
  (KH1's own windows are Phase 2 step 4). 3.4 MB, built in 0.5 s on the owner's PC.
- The DLL (t12): with `combos = kh1`, `kh1_assets::build_into` from Panacea's `OnInit` writes
  `mod\kh2\obj\P_EX100.mset` and a stamp (`kh2-kh1-build.txt`: the mod's version and KH1's file
  entries), again only when the stamp differs; with `combos = kh2` it removes them. Assumptions:
  Panacea serves a loose file in the mod folder in place of the game's; `OnInit` runs before KH2
  loads Sora. Nothing plays motions 210..216 yet: Phase 2 step 2 (KH1's rows in `ptya`).
