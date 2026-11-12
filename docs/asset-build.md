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

## Open choices (asked of the owner)

- Where the build runs: in the DLL on first launch (no extra program; the first launch takes
  longer), or a separate builder program the player runs once.
- Which KH1 to read: the player's KH1 install directly (KH1's own archives), or an extraction they
  made first.
