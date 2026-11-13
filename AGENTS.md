### 2026-11-13: Stage 44 (testing branch: KH1 jump on KH2's jump)
Owner: "Sora has his KH1 high jump animation for normal jump, and KH2's High jump for high jump".

- `MOVES`: KH2's jump row (14) gets KH1 motion 8 (KH1's jump; requests 4..7 → slots 8..11) instead of 12, which is KH1's High Jump (requests 9..0xC → 12, 13, 10, 11). KH2's High Jump motion (201) is a DUMM row in `P_EX100.mset` (Checked (files)): KH1's 12 goes there once the set KH2 loads it from is found.

### 2026-11-13: Stage 43 (testing branch: KH1 motions: travel in the bones, one Euler form)
Owner: "Sora still rubber bands during his ground combo", "Sora still flips 180 during his KH1 guard with the KH1 player model".

- Travel: KH2's own motions keep it in the bones (Checked (files): A300 root z 145 / travel 150), KH2 draws the model at the object less the travel; ours took it out, so Sora was drawn behind himself and snapped forward when a swing ended. The trace (t42) showed the object moving by KH1's travel. Now the bones keep it, on both skeletons; converted KH2 motions take it off only for the transfer.
- Euler forms: every motion for KH1's skeleton starts from one reference pose's forms (KH1 battle idle, frame 0), so KH2's blends (which mix Euler numbers) can't go the long way round. Assumption: the cause of the guard's 180° flip.

### 2026-11-13: Stage 42 (testing branch: KH1 idle, walk, run, jump, fall and land motions)
Owner (t41 video): "Sora still has his KH2 walking, running, idle, keyblade idle, jumping, falling etc animation instead of his KH1 animation".

- `kh1_assets::MOVES` (movement = kh1): KH1 motions 0 / 1 (field / battle idle), 2-4 (walks), 5-7 (runs), 12 (jump), 10 (fall), 11 (land) written into KH2's rows for idle 0/2, walk 4/6/7, run 8/10/11, jump 14, fall 18, land 22. KH2's ids from the OG rewrite (rows Checked (files)); the KH1 mapping is an Assumption from lengths, poses and the sister project's request map (docs/movement.md).
- The KH1 slots are now motion set rows (id × 4 + variant), so field / battle variants can differ.

### 2026-11-13: Stage 41 (testing branch: Phase 3: KH1's guard)
Owner: "Finish phase 3". First system: the guard.

- `kh1_assets::GUARD`: KH1 motion 110 (KH1's guard, attack row 0x3D, guard window 10-36, Checked (files)) written into KH2's guard slot 173 with `defence = kh1`; the Keyblade's guard window (range 10, ATKP 161) at KH1's frames, Sora's triggers moved with it, his combo window from KH1's 22.
- The build now takes its KH1 slots by setting (`select_kh1_slots`: swings with combos = kh1, the guard with defence = kh1) and runs when any of combos, defence, model is kh1.
- Not yet: KH1's Square rule (guard on a light tilt too), the guard arc. Plan: docs/defence.md.

### 2026-11-13: Stage 40 (testing branch: Config menu: L1 / R1 to the mod page)
Owner: "make the config menu tab between main config and mod config by using r1 and l1".

- `win/config_menu.rs`: the 9th row (t39) replaced by a page of ours in KH2's own Config screen. L1 / R1 (raw pad word L1 0x400 / R1 0x800, Assumption from the sister project's notes) flips KH2's 8 rows and ours (Movement, Combos, Defence, MP, Magic, Abilities, Camera, a help row, Player Model), each KH2 / KH1, drawn and changed by KH2's own code (`0x140365A20`, `0x140364D70`).
- Leaving the screen on our page writes `kh2-kh1.ini` (applies at the next launch) and gives KH2's values and rows back before KH2's apply `0x140365460` saves them. Hooks: open `0x140365580`, step `0x140364D70`, apply `0x140365460`, text lookup `0x1403AD9A0` (prologues checked).

### 2026-11-13: Stage 39 (testing branch: Config menu: the KH1 in KH2 row)
Owner: "add the config menu", then: a 9th row in KH2's Config that opens our page; settings in `kh2-kh1.ini`.

- `win/config_menu.rs`: KH2's Config row table (`0x1405BC340`, 8 rows of 0x14, Checked (exe)) copied with a 9th row of ours; its 15 `lea` references (Checked (exe bytes)) repointed; the list's row count 9 (`0x1403656EF`), so it scrolls; the values array 16 bytes (`0x140365390`); our row's value cleared when the screen opens (hook on `0x140365580`, prologue checked). Our texts through a hook on the text lookup `0x1403AD9A0`.
- The row opens nothing yet (next version: our page).

### 2026-11-13: Stage 38 (testing branch: KH1 air swing hop and speed)
Phase 2 (owner: "Finish Phase 2"). KH1's air swings hop by their row's height (20, finisher 40) under KH1's gravity and move at a constant 4 (finisher 12) toward the aim (KH1 `0x1402A68F0`, `0x1402AF960`, Checked (decomp); heights from `btltbl.bin`, Checked (files)).

- `win/combo_tables.rs` (was `atkp.rs`): after the table loader, Sora's `ptya` air rows 9 / 10 get height 20 and speed 4, rows 7 / 8 height 40 and speed 12 (jump = min = max, speed min = max). Assumption: KH2's row heights are arc heights (the OG rewrite). Not covered: KH1's 8 on the first air hit.
- Combo length checked against both decomps: KH2's rule and count are KH1's (3 + Combo Plus, the count 1 on the first press); no change.

### 2026-11-13: Stage 37 (testing branch: KH1 swing power on ATKP)
Owner's choice (Phase 2 damage): KH2's formula, each KH1 swing the nearest KH2 power, KH1's reactions.

- `win/atkp.rs` (combos = kh1): after KH2's table loader `0x1403F4C30` (prologue `48 89 5C 24 08`), the swing rows in the loaded ATKP table (`0x142AE57B0`, Checked (decomp)): 126, 127, 128, 131, 132 power 100; 129, 130, 133, 134 power 200 (KH1: plain swings power 0, finishers 4, ×2.0 on KH1's curve; Checked (files) `btltbl.bin`). Reactions and knockback stay KH2's (they match KH1's kinds; KH1 has no knockback field).

### 2026-11-13: Stage 36 (testing branch: KH1 hit, combo and trail windows)
Owner: "Finish Phase 2, and add the config menu". Phase 2 step: KH1's swing timing.

- `kh1_data::mset`: a motion's events (`+0x3C`): hit windows, combo window, trail, foot effects, sounds. Checked (files) against the sister project's reading.
- `kh2_formats::motion::MotionTriggers::write`: byte for byte with KH2's 123 trigger lists (Checked (files)).
- `kh1_assets::swing_triggers`: in each swing slot the Keyblade's hit window (range 10, ATKP kept) and trail (12) at KH1's frames, Sora's combo window (11) at KH1's (none on the finishers), every other trigger moved with the hit. `W_EX010.mset` is written again with these (16 bytes shorter than KH2's; the build refuses a longer one).
- Assumption: KH2's range 11 acts as KH1's combo window. Details: docs/combos.md.

### 2026-11-13: Stage 35 (testing branch: Remove the Keyblade-switch diagnostic)
Owner: "no crash!" (t34). The diagnostic hook on `0x140407DE0` (t31/t32, `win/itemcheck.rs`) is removed; the crash report (`win/crash.rs`) stays.

### 2026-11-13: Stage 34 (testing branch: KH1 model: motion set under KH2's size)
Owner: "bro the game is still crashing when switching keyblades lets fix this now", then with `model = kh2` (combos kh1): "no crash". So the motion set's size (3.4 MB with the KH1 model, KH2's 2.6 MB) is the cause. Assumption: an area heap (default `0x142A0F7B0` = 0x280000, Checked (decomp)) that a bigger file overruns.

- `convert::write_motion`, new skeleton: two IK helpers (root, travel) instead of the template's 11-53; travel curves on helper 1. Fixes travel pointing at helper 52 in templates with fewer helpers.
- Rotation tolerance per bone from the farthest joint it carries (0.12 units).
- `fit_hermite`: a pass removing keys the neighbours make needless; tangents on a 1/1024 grid, so most share one value (the fit checks with the rounded tangents).
- KH1 model motion set 3.43 MB to 2.39 MB (KH2's 2.61 MB); every joint within 0.67 units of the written pose. Checked (files).

### 2026-11-13: Stage 33 (testing branch: KH1 swings in KH2's own swing slots)
Owner: "game crashed take the log" (t32): the Keyblade-switch crash. It does not happen with `combos = kh2` (owner, t31 build). The object with the overwritten collision list is at the same heap address each run (t31, t32 logs), so the cause is in the files we write for the combo, not the model.

- `kh1_assets::SWINGS`: KH1's motions are written into KH2's swing slots (151 ← 98, 152 ← 99, 153 ← 100, 154/155 ← 101, 181 ← 102, 182 ← 103, 183/184 ← 104, the t12 row mapping), each keeping its slot's own triggers, instead of being added at 210..216.
- `W_EX010.mset` is no longer written (a build removes one left by an older build): the Keyblade plays KH2's own motion of the same id. The `ptya` row patch and the table loader hook (`win/combos.rs`) are gone; nothing is hooked for combos.
- With `model = kh1`, the same slots hold KH1's motions on KH1's skeleton (`combos = kh1`) or KH2's swings converted (`combos = kh2`).
- Assumption: the crash came from the longer Keyblade motion set (loaded again on a switch) overrunning its room. Not checked in the decomp.

### 2026-11-13: Stage 32 (testing branch: Keyblade-switch crash: name the object)
Owner: "crashed" (t31), then with `combos = kh2`, `model = kh2`: "no crash". So it comes from the KH1 combo (the KH1 model is not needed: the crash came before it).

t31's log: the walk `0x140407DE0` runs over every object (`0x140407760` loops `0x1403BF4F0`, Checked (decomp)); it crashed on an object that is not Sora, whose collision list (`+0xAB8`) holds no collision data (count over 256, bytes like mesh data): memory reused after the list it pointed at was freed. Assumption: the old Keyblade, still with an attack on.

- `win/itemcheck.rs`: the log line also gives the object's objentry id (`+0xC`) and its own list (`+0xAB0`) next to the one in use.

### 2026-11-13: Stage 31 (testing branch: Keyblade-switch crash: log the item id)
Owner: "still crashes, pull the log yourself never ask me again". t30's log: the fault came as a stack overflow inside the crash report, so the id was lost. The collision lists of `P_EX100.mdlx` and nine Keyblade models (`W_EX010`, `_10` … `_60`, `_A0`, `_U0`) have no entries of types 5 or 14, and no motion set entry holds a collision list. Checked (files). So the list `0x140407DE0` walks (object `+0xAB8`) is set at run time.

- `win/itemcheck.rs`: hooks `0x140407DE0` (prologue `40 55 41 56 48 81 EC C8 00 00 00`, Checked (exe)). Before the original runs it walks the same entries and logs any item id the item table lacks: the object, whether it is Sora, the entry's bytes, `+0xCF0`, `+0x9F0` and the object's vtable. Once per case. Temporary diagnostic; runs on attack frames only. The crash itself is unchanged.

### 2026-11-13: Stage 30 (testing branch: Crash report: the faulting function's stack words)
Owner: "you can open the camp menu, switching keyblades crashes the game". t29's report: a read of 0x2 at 0x1403B68E0 (in 0x1403B68B0, which looks an item id up in the item table by bsearch and reads the row without checking it was found), called from 0x140407EBC (in 0x140407DE0, which walks Sora's collision entries of types 5 and 14 while an attack is on and looks up each one's item: entry +2 for type 5, else Sora +0xCF0). Checked (decomp). Same place as the keyblade-switch crash before the KH1 model. The id itself isn't in the report.

- `win/crash.rs`: also logs the first 16 stack words raw. 0x1403B68B0 saves the caller's rbx, which holds the item id, at rsp+0x30. Checked (exe bytes: `40 53 48 83 EC 30`, the caller passes `mov ecx, ebx`).

### 2026-11-13: Stage 29 (testing branch: KH1 model: a smaller motion set)
Owner: "entering the camp menu crashes the game again" (t27). Suspect: the converted motion set was 9.8 MB against KH2's own 2.6 MB, and the camp menu crashed before on Sora's motion file (t14). Assumption until the t28 crash report says where it faults.

- `convert::write_motion`: Hermite keys fitted to the curve (`fit_hermite`: the samples' slope as each key's tangent, a key added where the curve misses a frame worst) in place of straight lines; a key's in and out tangent stored once; equal values and tangents shared.
- For a new skeleton (KH1 Sora), channels at the bone's bind value are left out of the starting pose, as KH2's own motions do (A300 sets 196 of 2,052; the rest play the bind). Rotation tolerance 0.002 to 0.004 rad.
- 79 motions: 9.8 MB to 3.9 MB. Read back, every joint within 0.91 units of the pose written. Checked (files).

### 2026-11-13: Stage 28 (testing branch: Crash report in the log)
Owner: "entering the camp menu crashes the game again" (t27, model = kh1). No address to go on, so the log now names crashes.

- `win/crash.rs`: a vectored exception handler that, on an access violation, illegal instruction, divide by zero or stack overflow, logs the code, the faulting address (as a Steam exe address), what was read or written, eight registers, and the first 16 exe return addresses on the stack. First 3 faults only; it only reads and logs, then KH2's own handling continues. Nothing per frame.

### 2026-11-13: Stage 27 (testing branch: KH1 player model: model, motions and the Config row)
Owner: "Lets see if we can have KH1 sora's player model be placed in KH2", then "B" (KH1's own 293-bone skeleton, every KH2 motion converted onto it, a Config row).

- `kh2_formats::mdlx_write`: KH2 model (BAR 4) and texture (BAR 7) writers in the layout of KH2 Sora's own `obj/P_EX100.mdlx`. Checked (files): Sora's mesh written back reads to the same triangles; textures read back through our reader.
- `kh1_assets::player_model::build`: KH1 Sora (293 bones, 9 meshes, 7 textures) as `P_EX100.mdlx`; KH2's skeleton data and collision list kept, their bone numbers moved to KH1's bone at the same place (`BONE_MAP`, each within 12 units after height scaling, Checked (files)). KH1's HD textures as `remastered/obj/P_EX100.mdlx/-0..-6.dds` (Assumption: OpenKH serves them).
- `player_model::build_mset`: all 79 KH2 Sora motions posed by KH2's evaluator, moved onto KH1's bones by `kh1_data::retarget`, written for 293 bones (read back within 0.62 units, Checked (files)); KH1's swings 98-104 at slots 210-216 from `kh1_data::rig` (KH1's own bone pass, matched to KH1 in t24).
- Config: `model = kh1 / kh2` in kh2-kh1.ini; `kh1` builds the model, the converted motion set, `W_EX010.mset` and the HD textures on launch; `kh2` removes them.
- Not yet: Sora's motions outside `P_EX100.mset` (Reaction Commands' captures, Limits, world variants), cutscenes, the Keyblade's attach bone (unknown). Details: docs/kh1-player-model.md.

### 2026-11-13: Stage 26 (testing branch: KH1 player model, step 1: read KH1 Sora's mesh)
Owner: "Lets see if we can have KH1 sora's player model be placed in KH2", then "B" (KH1's own skeleton; KH2's motions converted onto it).

- Plan in docs/kh1-player-model.md (5 steps).
- Step 1: `kh1_data::mdls::parse_mesh` reads KH1 Sora's meshes and textures (layout after OpenKH's `OpenKh.Kh1.Mdls`, Apache-2.0, credited; our own code). `xa_ex_0010.mdls`: 9 groups, 5,182 vertices, 2,828 triangles, 7 textures; all single-weight.
- Checked: files (the mesh drawn offline on KH1's idle and swing 98 is whole). No change in game yet.

### 2026-11-13: Stage 25 (testing branch: KH1 swings: the lean on KH2's proportions)
Owner: "B" (KH1's timing, swing path, travel and hit frames, with the lean and spread scaled to KH2's proportions; not 1:1).

- Measured over every frame (test `dump_kh2_vs_kh1_extremes`): KH2's own attacks lean the trunk median 35° / 95th percentile 60° (max ~63°); KH1's on KH2 46° / 68° (max 102°, swing 1). Thigh lift was already within KH2's range. Checked: files.
- `convert::soften_lean` maps the lean (46→35, 68→60, 102→65), tilting pelvis and upper body about the hips. The legs keep their turns and the lowest foot stays put. After: lean 35° / 60°, hip bend 97° / 135° (KH2: 97° / 132°).
- Not checked in game.

### 2026-11-13: Stage 24 (testing branch: KH1 reader 1:1 with KH1's own poses)
Owner (t23, video): "still very funky looking".

Against the kh1-posedump recording (test `kh1_truth`) the KH1 reader now matches KH1 within 0.1–0.2 units on every bone and frame of the idle and swings 98, 99, 101–104 (was 7–55):
- KH1 key tangents are per 1/256 frame (`0x14038F820`); they were read 256× too small, flattening every curve.
- A one-bone aim (word mode 1) places the next bone at its bind offset (`0x1403900B0`).
- A child's offset is scaled by its parent's accumulated scale.
Checked: trace (the recording), decomp. Not checked in game.

### 2026-11-13: Stage 23 (testing branch: KH1 swings: the pelvis measured from KH1's own zero)
Owner (t22): "The T Pose looks normal. the rest of the combo looks how it did before".

- KH2 Sora's real mesh drawn offline on the converted poses showed the game's torn polygons. The pelvis took KH1 bone 0's turn from its bind rotation, but KH1 drives bone 0 by copying helper 3's rotation (word bits 0-1 = 2), whose zero is the body's facing: 109° off. Now measured from the helper's rest frame (`Inverse::set_torso_rest`). Checked: decomp `0x1403900B0`, offline mesh (the converted idle stands right; swing 98 holds together).
- The rest-pose test is off: the first ground swing is KH1's again.
- Remaining: edges stretch up to 15–22 at the deepest lunge frames, around the hips (KH2's own swing: 13). Not checked in game.

### 2026-11-13: Stage 22 (testing branch: test: KH2's rest pose in the first ground swing)
Test build. KH2's own `.mdlx` rest pose (bind), held still, plays as the first ground swing. If KH2's mesh looks clean on it (a T-pose), the rest's bone frames are the ones the mesh follows and the stretching is elsewhere; if the mesh twists, the transfer must turn bones from frames KH2's own motions use. Checked: nothing yet (the owner's look decides).

### 2026-11-13: Stage 21 (testing branch: KH1 swings: Euler forms like KH2's; KH1 ground truth)
Owner: KH1's swings on KH2 Sora are "stretched like CRAZY"; recorded KH1's own poses with kh1-posedump.

- Recording compared (test `kh1_truth`): KH1 itself puts 98's hips at 59 and lunges low mid-swing, as our KH1 reader does (within 0.7 idle, 7–30 most swing frames). Checked: trace.
- Converter: each bone's first frame takes the Euler form nearest A300's own, so KH2's motion blends (which mix the numbers) don't turn bones the long way round. Assumption: part of the stretching; not checked in game.

### 2026-11-13: Stage 20 (testing branch: kh1-posedump: F10 on its own thread, hooks logged)
Owner: "i pressed f10 twice" (nothing recorded). The tool read F10 only inside the hooked skeleton passes. It now watches F10 every 30 ms on its own thread, and logs each hook's first call and every 3600th, so the log shows whether KH1's passes reach the hooks. Checked: nothing new (a test tool).

### 2026-11-13: Stage 19 (testing branch: first ground swing back to KH1's)
Owner (t17): the round-tripped A300 "looks exactly like KH2's base attack". The test is done: KH1 motion 98's slot holds KH1's first ground swing again (`ROUND_TRIP_FIRST_SWING = false`). Checked: files.

### 2026-11-13: Stage 18 (testing branch: kh1-posedump: KH1's own poses as ground truth)
Owner: "Yes i will run it, and please feel free to use any overlapping code from the sister mod if it speeds things up, lets get Phase 2 finished".

- New test tool `crates/kh1-posedump` (not part of the mod; CI builds `tools/kh1-posedump/kh1-posedump.dll`). It hooks KH1's skeleton pass `0x140391C70` (prologue `4C 8B DC 53 56 41 54`) and the ground-IK pass `0x140392560` (`4C 8B DC 55 53 41 54 41 57`). F10 records Sora's world and local bone matrices and his motion's joint words to `kh1-posedump.bin`.
- Checked: decomp (instance `+0x1B0/+0x1B4/+0x1B8`, handle table `0x142EE3980`), exe bytes. The sister notes' `0x140391E20` / `0x2EE4730` are for another KH1 build (bytes differ). Notes: docs/kh1-posedump.md.

### 2026-11-12: Stage 17 (testing branch: keyblade motions for KH1 swings, round-trip test)
Owner (t16, video): "the swings are still off. Also, the game crashed when switching keyblades".

- The keyblade motion set `obj/W_EX010.mset` mirrors Sora's 839 slots (checked, files) and had nothing at 210–216: the keyblade vanished in KH1 swings, and switching keyblades crashed (link assumed). The build now writes it too, with each new slot a copy of the replaced KH2 swing's keyblade motion. Removed with combos = kh2.
- Test build: the first ground swing's slot holds KH2's own A300 run through the converter, to tell in game whether the converter or the KH1 pose transfer is at fault.
- Non-root bones take the pose's offset again (same for KH1 swings; KH2's own motions need it).
- Checked: files (keyblade set built and checked on the PC: KH2's 839 entries unchanged, A300..A333 copies at 840..867). Not checked: in game.

### 2026-11-12: Stage 16 (testing branch: KH1 swings: curves KH2 actually plays)
Owner (t15): "he's stretching the same exact way. also the game crashed". The t14 video shows Sora holding one bent-over pose through every KH1 swing while only his position moves.

- A channel now gets either a starting value or a curve, never both. KH2's own motions never give one channel both (A300: 196 starting values, 161 curves, no overlap; checked, files). Holding the starting value is the likely cause of the frozen, bent pose (Assumption from the video).
- Keys are Hermite (type 2), as all 1,145 of A300's are, with tangents equal to the slopes either side, so the curve is exactly our lines. KH2's tangents are value per key-time unit (checked: A300's tangents over its slopes, median 1.0000, 716 keys).
- Not checked: in game. The crash left an empty CrashDump.dmp, so no address yet.

### 2026-11-12: Stage 15 (testing branch: KH1 swings: the key-time count)
Owner (video, t14): "he's still stretching like crazy".

- Header `+0x10` of an interpolated motion is the length of its key-time table (checked: all 89 of KH2's own Sora motions). The writer kept A300's 27 while ours use 48–80 times; KH2 most likely searched only the first 27, so most keys landed on the wrong times. Now set to our table's length.
- Rotations of every bone now checked to play back (test `rotations_play_back`, worst 0.013 rad), not only joint positions.
- Checked: files. Not checked: in game.

### 2026-11-12: Stage 14 (testing branch: camp-menu crash and stretched swings)
Owner: "soras animations are really bad he stretches like crazy. the game also crashed" / "the game crashes when entering the camp menu".

- Crash: the build rewrote `P_EX100.mset` with a private copy of every entry's data but kept KH2's `duplicate = 1` marks, so KH2 never prepared those copies and the camp menu read an unprepared offset as a pointer (`0x1402FEB20`, `call [rax+0x30]` after `0x1403E6230`, read of `0x4EC7`). Now `convert::extend_bar` keeps KH2's file byte for byte (moved down by the longer table) and appends our motions; our DUMM slots copy KH2's. Checked (crash dump, files); that KH2 skips duplicate entries is an Assumption that fits the dump.
- Stretch: A300's joint table has four two-bone IK chains and matrix-to-rotation flags whose targets come from A300's inverse curves; ours have none, so KH2 pulled the limbs to rest targets. Our motions now clear every joint flag and carry no external effectors. Non-root bones keep KH2's bind offsets. Checked (files; test `kh2_bone_lengths_hold`). In-game look not yet checked.

### 2026-11-12: Stage 13 (testing branch: combos kh1: Sora's plain combo rows play KH1's swings through KH2's own picker)
- The owner (chat, 2026-11-12): "nothing crashes pull the log and finish phase 2". The t12 log: the build ran (7 KH1 motions, 80 ms) and the game played on with the bigger motion set. Phase 2 step 2. Checked (decomp): Sora's `ptya` set is his `+0xD50` (PARTY constructor `0x1403D5840`), read by the selector `0x1404028D0`, checked by `0x140402750` (ability, air bit, combo stage: count `+0xD74` against length `+0x200` / `+0x201`), walked by `0x140402C50` (first valid row not playing; fallback ids 5 / 9); KH1's pickers `0x1402A6EE0` / `0x1402A6C70` work alike (first marked row in order not playing; finisher at the count reaching the length; fallback 13 / 18). Checked (files): Sora's 37 KH2 rows (`00battle.bin`) and KH1's 66 (`btltbl.bin +0xA9F0`). Change (`combos = kh1`): `win/combos.rs` hooks KH2's table loader `0x1403F4C30` (first bytes checked); after it, the plain rows' motions in Sora's set become KH1's (ids 4 / 5 / 6 → 211 / 210 / 212, finishers 2 / 3 → 213, air 9 / 10 → 214 / 215, air finishers 7 / 8 → 216), each only if it still holds KH2's motion, and only when this launch's build made motions 210..216. Ability rows, windows and order stay KH2's (KH2's ability moves join KH1's combo). Assumptions: KH1 request `0xC8 + n` plays motion `98 + n`; Panacea serves the built motion set (the first swing shows it: motion 210 in the trace); KH1's target tests near KH2's windows; KH1's combo length; A300's hit and combo windows on every KH1 swing for now. Not checked in game. Per frame: nothing (load time).

### 2026-11-12: Stage 12 (testing branch: the build step: KH1's combo motions as KH2 motions, built into the mod folder)
- The owner (chat, 2026-11-12): "okay actually work on Phase 2 now". Phase 2 step 1 of `docs/combos.md`. Change: `kh1_assets::convert`: a KH1 motion as a KH2 interpolated motion: KH1's pose each frame (`kh1_data::rig`, t11), its travel (helper H10) off the body, the body onto KH2 Sora's 228 bones (`kh1_data::inverse`, the sister project's), local Eulers / offsets with reduced linear keys, written on KH2's A300 as the template (its IK helpers, joint table, bounding box and root block kept; no constraints), KH1's travel on KH2's travel helper (joint 52, A300's 150 on it, Checked (files)). `build_mset`: KH2's `P_EX100.mset` with KH1's ground combo 98..101 and air combo 102..104 added as KH2 motions 210..216 (battle slot; A300's triggers until KH1's windows are built). Checked (files) on the owner's install: played back by KH2's evaluator within 0.4 of the retargeted pose at every joint, travel matched (`tests/local_convert.rs`); KH2's 839 slots byte-identical in the rebuilt BAR; 3.4 MB, 0.5 s. The DLL: with `combos = kh1`, Panacea's `OnInit` builds `mod\\kh2\\obj\\P_EX100.mset` (and `kh2-kh1-build.txt`, its stamp) when the stamp differs; with `combos = kh2` it removes them. Assumptions: Panacea serves the loose file in place of the game's; `OnInit` runs before KH2 loads Sora. Nothing plays these motions yet (step 2). Per frame: nothing (launch only).

### 2026-11-12: Stage 11 (testing branch: the build step: KH1's whole skeleton pass offline (helpers, aims, two-bone IK))
- The owner (chat, 2026-11-12): "okay actually work on Phase 2 now". Phase 2 step 1 (`docs/combos.md`): KH1's swings as KH2 motions needs KH1's pose at every frame, and KH1 Sora is driven by 11 helpers through aims and two-bone IK (the sister project's `docs/kh1-anim-slots.md` §1, re-read in the KH1 Steam decomp here). Change: `kh1_data::mset` reads the rest of a motion (joint words `+0x14`, the helper table `+0x30`, helper channels `+0x24`, constant channels `+0x38`, Checked (decomp) `0x14038F550`); new `kh1_data::rig::pose` is KH1's skeleton pass offline (bind + constants, helpers, the bone pass `0x1403900B0` by joint word: FK / aim / copy, one-bone aim, two-bone IK `0x14038FC20` with its bend flag, pole and twists); ground lift and look-at left out (they need the game's world). Tests: the IK end on its target, lengths kept; the aim. `kh1-assets pose <folder> <motion>` dumps a motion's poses; run on the owner's files: the idle stands on its foot helpers (about 145 tall); 98..101 travel forward with H10, feet helpers with them; bones rigid but bone 3 (placed at helper P6 by design). Assumption: KH1's multiply order (taken from what the solver needs to mean). The DLL is unchanged. Per frame: nothing.

### 2026-11-12: Stage 10 (testing branch: the build step's readers: both Soras' motions and skeletons from the player's install)
- The owner (chat, 2026-11-12): "Move onto phase 2"; answers: "feel free to copy the code" (the sister project's format readers), "Yes, use the mod folder", "Join KH1's combo" (KH2's equipped ability moves join KH1's combo, for a later version). Phase 2 step 1 of `docs/combos.md` begins with the asset build (`docs/asset-build.md`). Change: `crates/kh2-formats`, `crates/kh2-anim`, `crates/kh1-data` copied from `KH2-Combat-in-KH1` testing (HD archives and cipher, BAR, KH2 motions and skeletons, KH1 packages, motion sets, models, retargeting); its failing `bake::continuous_angles_take_the_short_way` (failing there too, its KH2→KH1 bake, unused here) marked ignored. New `crates/kh1-assets`: KH2's packages (`kh2_first..sixth`, the backup copy first), and `scan`, which reads KH1 Sora's `xa_ex_0010.mset` / `.mdls` and KH2 Sora's `obj/P_EX100.mset` / `.mdlx`; the `kh1-assets` tool prints it. Checked (files), the owner's install through the tool: KH1 118 motions (7,440 frames), 293 bones; KH2 839 motion-set entries, 228 bones. The DLL is unchanged (it doesn't build anything yet). Per frame: nothing.

### 2026-11-12: Stage 9 (testing branch: movement kh1: the early-release hop is KH1's (6.06 high, 5 frames up))
- The owner (chat, 2026-11-12): "pull the kh2 trace, finish phase 1 and 0". The t8 trace, Checked (trace): jump 190, High Jump 290, Valor's own 235 / 585 / run 12 (t5..t7 as meant); KH1's steering +0.8 a frame to 8, Valor's KH2 easing (t8 as meant); walking off a ledge already KH1's (from rest, KH1's gravity, sideways 8); the early-release cut in KH1's window with KH1's fall after it, but its hop rose 5.10 (KH2's), where KH1's is 6.06 (`T = 5, h = -6.059` in the KH1 traces; KH1 `0x1402A7B30` → `0x1402AF8F0 (actor, 5.0)`, Checked (decomp)). KH2's hop height is `0x142A243D8` (set at runtime), handed on by `0x1403CB680`, Checked (decomp). Change: `win/arc.rs`'s launch wrapper: on base Sora a launch of exactly that height gets KH1's hop (time to apex 5, height `g/2 · 25`, `kh1::hop_height`, tested). Assumption: Sora's hop passes through `0x1403CBC30` / `0x1403CBB00` (a log line the first time says so). Not checked in game. Per frame: nothing new.

### 2026-11-12: Stage 8 (testing branch: movement kh1: KH1's air steering on base Sora)
- The owner (chat, 2026-11-12): "Finish phase 0 and do finish phase 1". Checked (trace, the owner's KH1 traces): KH1's air steering adds 0.1 × the stick's speed a frame (+0.8 at full tilt) up to the run speed 8 and keeps a faster speed without adding to it; let go, ×0.95 a 30 Hz frame. Checked (decomp): KH1 `0x1402AF6A0` (`v += dir × accel × speed`, length kept to max(old, run)); KH2 `0x1403B7AD0 (actor, &out, step, keep, drag)`: held, Sora's `+0x20` eases toward `+0x10` × `+0x1C` by keep; let go, × drag; out = velocity × step; his plain air moves pass 0.9 / 0.95 (`0x1403CB360`) once a 30 Hz step. The drag is the same in both (0.95 a 2-frame step). Change (`movement = kh1`): `win/airsteer.rs` wraps `0x1403B7AD0` (first bytes `48 8B C4 48 89 58 08` checked): on base Sora's plain call (0.9 / 0.95) with the stick held, his `+0x20` and the step's move are KH1's (`kh1::air_steer`, once per 60 Hz frame of the step, tested against the trace). Other values (KH2's own moves) and Drive Forms keep KH2's. Assumption: `+0x10` is a unit direction. Not checked in game. `docs/movement.md`. Per frame: one compare per air-control call; the steer only while Sora steers in the air.

### 2026-11-12: Stage 7 (testing branch: movement kh1: any KH2 High Jump level is KH1's High Jump (290) on base Sora)
- The owner (chat, 2026-11-12), answering "KH2's High Jump at movement = kh1: (a) any KH2 level gives KH1's 290, or (b) each KH2 level keeps its own KH2 height": "1. A". Checked (decomp): KH2 asks the jump height of `0x1403B59B0 (actor, &motion)` and `0x1403B5A40 (actor)`: Sora's `+0x130` (his `prty` row's, KH1's 190 since t5), or with a High Jump level (`0x140401680`) that level's `fmab` row height (`0x1403E92D0`) and the flip motion 201. Checked (trace): KH1's High Jump is 290 (`h = -290`, `T = 34.592`). Change: `win/highjump.rs` wraps both (first bytes checked): for base Sora (`sora::is_base_sora`, t6) a height other than his `+0x130` (a High Jump's) becomes 290. `fmab` isn't changed: the Drive Forms read it too (answer 2a). Not changed: the High Jump's motion (KH2's flip 201; KH1's own High Jump motion waits for the animations). Assumption: these two are every way KH2 reads the jump height (both callers lists read: `0x1404222E0`, `0x140422630`, `0x1403D78D0`, `0x1404027F0`). Not checked in game. Per frame: nothing (on a jump).

### 2026-11-12: Stage 6 (testing branch: movement kh1: Drive Forms keep KH2's jump and fall)
- The owner (chat, 2026-11-12), answering "Drive Forms: (a) keep KH2's jump, or (b) use KH1's jump": "2. A". Change: `win/sora.rs` (shared by the movement systems): Sora is `0x142AE9A28`'s object, base Sora when his movement block's `prty` row pointer `+0x148` (stored by `0x14040A480`, Checked (decomp)) is `prty` row 1, Sora's own. `win/arc.rs`: KH1's gravity and fall cap only on base Sora; a Sora built on another row keeps KH2's fall cap (and logs it). `win/prty.rs` reads the row through it (no change: only row 1 is written). Assumption: every Drive Form loads a row other than 1 (the log line on a Sora built on another row shows it; the forms not checked in game). `docs/movement.md`. Per frame: one more compare per arc call.

### 2026-11-12: Stage 5 (testing branch: movement kh1: KH1's jump height and turn rate in Sora's prty row)
- The owner (chat, 2026-11-12): "Both traces are done" (KH2 t4 trace, three KH1 traces). Read, Checked (trace): KH1 Sora's jump height 190 (290 with High Jump: `h = -190, T = 28.0` / `-290, 34.592`), turn rate π/16 (0.196 a frame at most), air acceleration 0.1 and drag 0.95 (+0.8 a frame from rest; ×√0.95 a frame let go), run 8 / walk 2 as KH2's. KH2 Sora's row 1: walk 2, run 8, jump 185, turn 0.262 (t3 trace). t4 Checked (trace): the launch's apex time 27.6 at 185 (KH1's gravity), the fall 0.4846 a frame² (KH1's); KH2's physics steps at 30 Hz at 60 fps. Change (`movement = kh1`, Steam): `win/prty.rs` hooks KH2's pref setter `0x1403E9360` (first bytes checked) and, after it, writes KH1's jump 190 and turn π/16 into Sora's `prty` row 1 (only if it holds KH2's 2 / 8 / 185 / 0.262), so KH2's own `0x14040A480` gives them to Sora. `kh1.rs`: the trace's numbers (tested against the traces). `win/arc.rs`: its first-use log line now says launch and fall separately (t4 printed only the first). Not yet: KH1's air steering (accel 0.1 / drag 0.95, a wrapper on KH2's `0x1403B7AD0`), the walk-off fall, KH2's High Jump at KH1 (a question to the owner). `docs/movement.md`. Per frame: nothing (load time).

### 2026-11-12: Stage 4 (testing branch: movement kh1: KH1's gravity and fall cap on Sora's jump and fall arc)
- The owner (chat, 2026-11-12): "Finish phase 0 and do finish phase 1". Phase 1, first system: the arc. Checked (decomp + exe, Steam KH1): KH1's arc is KH2's curve (the same rise, time to apex and fall formulas, Checked (decomp) both) with gravity 0.48469388 (KH1 `0x1402AFDA0`, `0x1402AF6A0`, RVA `0x3E9D14`) and a fall cap of 20 a frame (RVA `0x42F968`); KH2's is 20/49 (RVA `0x5CBD84`, one constant every object reads, so left alone) and Sora's `+0x138`. Both games' frame step counts 60 Hz frames (KH2 `0x140717480`, KH1 `0x14233FBDC`, Checked (decomp)): no conversion. Change (`movement = kh1`, Steam only): `win/arc.rs` wraps KH2's own functions, first bytes checked, all or nothing: Sora's constructor `0x140405030` (his `+0x138` = 20), the launches `0x1403CBC30` / `0x1403CBB00` (the time to the apex again with KH1's gravity), the steps `0x1403CB100` / `0x1403CB360` / `0x1403CB500` (past the apex, the frame's drop again with KH1's gravity, capped by `+0x138`). Any object but Sora, and `movement = kh2`, run KH2's functions untouched. `kh1.rs`: the constants and KH1's time to the apex (tested). Not yet: KH1's jump height, air acceleration and drag (KH1 fills them at runtime: a KH1 trace reads them), the walk-off fall (`0x1403B4780`). Assumptions: these five functions are Sora's arc in every jump and fall (the log says when each first runs); in a Drive Form Sora is the same object, so the forms get KH1's arc too (a question to the owner). Not checked in game. `docs/movement.md`. Per frame: a pointer compare per arc call; the recompute only while Sora falls.

### 2026-11-12: Stage 3 (testing branch: the trace: Sora each frame, read only, to settle KH2's tick rate and Sora's fields)
- The owner (chat, 2026-11-12): "finish phase 0". Phase 0 steps 3 and 4 of `docs/ROADMAP.md`. Change: `win/trace.rs`: with `trace = on` in `kh2-kh1.ini`, F10 starts and stops a recording of Sora, one row a game frame (frame, clock, Sora's pointer, `+0x70` and `+0x670` positions, the `prty` movement block `+0x128..+0x134`, `+0x180`, `+0xDE0`), written to `kh2-kh1-trace-N.csv` when it stops (cap 18,000 rows). It runs from Panacea's `OnFrame` export and reads memory with `ReadProcessMemory` (a bad pointer gives an empty cell, never a crash): no hook, nothing in the game changed. Off (the default), one flag check a frame. Sora: KH2's `YS::SORA` constructor `0x140405030` stores him at `0x142AE9A28`, Checked (decomp + exe RTTI); the movement block layout Checked (decomp) (`0x14040A480`); the other fields are Assumptions the trace checks (`docs/kh2-sora.md`). An ini from an older version gets this version's new rows added. Assumption: Panacea calls `OnFrame` in KH2 as in KH1. Per frame: one flag check (off).

### 2026-11-12: Stage 2 (testing branch: the foundation: the Steam build check, byte-checked hooks, a KH2 / KH1 setting per system)
- The owner (chat, 2026-11-12): "Let's put KH1 combat in KH2, feel free to start, give a roadmap and begin working on it". Phase 0 step 2 of `docs/ROADMAP.md`. Change: `win/build.rs` tells Steam from Epic by the exe's PE link stamp (Steam `0x669E384A`, Epic `0x669E44D8`, Checked (exe), `docs/sources.md`); on any build but Steam the mod logs that and changes nothing. `settings.rs`: `kh2-kh1.ini` next to the game (written on first launch), one `kh2` / `kh1` row per system (movement, combos, defence, mp, magic, abilities, camera), all `kh2` by default; read once at start and logged. No row does anything yet: each starts working with its system's version. `win/hook.rs`: the hook installer (adapted from the sister project's): refuses to install unless the function's first bytes match what we expect (at least 5); also a checked byte patch and a checked call redirect. No hook is installed in this version: nothing in the game changes. Also `docs/kh2-tables.md` (KH2's table loader `0x1403F4C30` and where it keeps `atkp`, `ptya`, `plrp`, `pref`'s parts, Checked (decomp)). Assumption: KH2's tick rate on Steam (open, next). Per frame: nothing.

### 2026-11-12: Stage 1 (testing branch: the repo's start: the loader, the build and the tester workflow)
- The owner (chat, 2026-11-12): "lets bring KH1's combat into KH2"; asked to set up the testing branch by adapting the sister project's scaffold (its tester scripts, Windows build and doc layout; none of its combat code or data). Change: `crates/kh2-hook`, a 64-bit DLL (`kh2-kh1.dll`) that Panacea loads from `<mods>\kh2\dll\`. In `KINGDOM HEARTS II FINAL MIX.exe` only, it writes `kh2-kh1.log` next to the game (the version, the build stamp, KH2's base and the exe's PE link stamp) and does nothing else: no hooks, no memory changed. In any other exe it does nothing. Assumptions: Panacea loads a KH2 mod's `dll/` folder and calls `OnInit` as it does for KH1 (from the sister project's notes; not checked for KH2); the PE link stamp tells the Steam and Epic builds apart (not checked: no KH2 exe in this session). Also: `.github/workflows/build-testing.yml` (builds and commits `kh2-kh1.dll` on every source push to testing), `mod.yml`, `tools/testers/` (`waiting.py`, `threads.py`, `version.py`, adapted from the sister project), and the docs (`docs/design-philosophy-instructions.md`, `engineering-rules.md`, `systems-to-port.md`, `tester-requests.md`, `tester-requests-archive.md`). Per frame: nothing.

