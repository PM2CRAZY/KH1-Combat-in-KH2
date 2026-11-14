# KH2's command menu: room for 12 rows a submenu (owner's choice (a1), 2026-11-14)

The owner wants Magic to list nine spells in KH1's order (Fire, Blizzard, Thunder, Cure, Gravity,
Stop, Aero, then KH2's Magnet and Reflect) and chose to make KH2's own command menu bigger: every
submenu gets room for 12 rows. This file maps what has to change. Steam addresses; tags as in the
other docs.

## What holds a submenu (Checked (decomp + exe))

- **`YS::PLAYER_COMMAND`**, one global instance at `0x142A10620` (static init `0x140024560`: ctor
  `0x1403B1C70`, then the `YS::FIELD_COMMAND` vtable). No other instance: the ctor's only caller is
  that init; `0x1403A79A0` / `0x1403A79F0` are its destructors.
- **19 submenus inline**, `0x80` bytes each, at `+0x28` (`+0x28 + 0x80 × i`, `i < 0x13`). A submenu:
  - `+0x00` kind (`u32`; 0 and 6 are roots; kinds in mask `0x7E420` are pick-a-target lists),
  - `+0x08` owner (the `PLAYER_COMMAND`),
  - `+0x10` row count,
  - `+0x14` **8 rows of 12 bytes**: `+0` `u16` flags (bit 0: a shortcut's row), `+2` `u16` command id,
    `+5` `u8` state (1, 5, 6: greyed kinds), `+6` `u8` (target class, `0x1405C4ED8` mask index),
  - `+0x74` cursor row (`i32`), `+0x78` cursor command id (`u32`, −1 none).
  - So a 9th row lands on the cursor. The row appenders never check the count: `0x1403B0F20`
    (one row), `0x1403B1840` (a 0-ended `u16` list), `0x1403B19B0` (a list with counts).
  - Row ctors loop 8: `0x1403B1BB0`, and the inner loop of `0x1403B1C70`.
- **The submenu stack**: `+0x9A8` points at the top entry of a stack of submenu pointers that starts at
  `+0xAF0` (push `0x1403B2800`: `*top = this + 0x28 + 0x80 × i`; depth = (`top` − `this` − `0xAF0`) / 8 + 1).
- **Submenu by number** from a submenu's owner: `owner + 0x80 × i`, then `+0x38` (count) and `+0x41`
  (row 0's state, stepping 12) in `0x1403B0F20`, `0x1403B1840`, `0x1403B19B0`; `this + 0x28 + 0x80 × i` in
  `0x1403B2800`, `0x1403F2470` (two), `0x1403F3AF0`; fixed ones: `this + 0x28` (submenu 0) in
  `0x1403F2470`, `0x1403F2600`, `0x1403F29E0`, `0x1403F3300`; submenu 11 (`+0x5A8`, row `0x1B7`) in
  `0x1403B26B0`; Magic (submenu 1, `+0xA8`) in `0x1403F29E0` / `0x1403F3460`.
- **The Magic list** `+0xB68`: room for 6 command ids and a 0 (`0x1403F27D0`, `0x1403F2910` from
  the six magic elements), handed to `0x1403B1840` at `0x1403F2B8F` / `0x1403F3479`. `+0xB76` is the
  next list (`0x1403E6D10`).
- No instruction addresses the submenus absolutely (the only RIP references into the object are its
  base, 7 `lea`s, and fields past `+0xAB0`): every access goes through a register. Checked (exe scan).

## What draws them (Checked (decomp); details still to map)

- The HUD's command object (`DAT_140ABC508`) keeps **4 levels × 9 sprite slots** at `+0x20`
  (level × 9 + slot; the title and 8 rows), per-level `i32`s at `+0x164` and `+0x174`, cursor `+0x188`
  / `+0x18C`. Built level by level from the stack by `0x14015E800` → `0x14015B6D0(hud, submenu,
  level, …)`; cleared by `0x14015CBE0` (9 a level). Sprites from a pool of `0x34` `dk::COMMAND_ONE`
  (`DAT_140ABC4F0`, made in `0x14015C660`; 52 = 4 × 13).

## Step 1: every site (t77; Checked (exe + decomp))

Found with a may-analysis over the whole Steam exe (every register and stack slot that can hold
the object or a submenu pointer, from: the object's address, its vtable methods, `[x+0xDD0]` and
`[x+0xDC0]` of the player (`0x1403A7A40` / `0x1403AA850` store the object there), `+0x398` of the
command HUD, the global `0x142AE80A0` (`0x1403FD830`), a submenu's owner `+0x08`, and the
submenu stack), then each hit read in the decomp. Code nothing calls (no call, jump, pointer or
RIP reference anywhere in the exe) is left alone: `0x1403B1080`, `0x1403B1590`, `0x1403B1BB0`,
`0x1403B1E90` (only in the ctor's own vtable, replaced at once), `0x1403F2680`, `0x1403F26F0`,
`0x1403F3000`–`0x1403F3A30` except `0x1403F3860`, `0x140394E70`, `0x1404045E0`. The ctor
`0x1403B1C70` runs once, before the move.

- **58 cursor accesses** (`+0x74` / `+0x78` on a submenu pointer) in `0x1403B1140`, `0x1403B11E0`,
  `0x1403B1B70`, `0x1403B1B80`, `0x1403B21D0`, `0x1403F22E0` (also the submenu under the top,
  `[top-8]`), `0x1403F2470`, `0x1403F3AF0` and the HUD's `0x14015B6D0`, `0x14015BA00`. The full list
  with bytes is `CURSOR_SITES` in `crates/kh2-hook/src/menu_rows.rs`.
- **32 addresses with a 32-bit displacement** (`object + 0x28 + 0x80 n (+ field)`): in
  `0x1403B26B0` (11), `0x1403B2A90` (14), `0x1403F27D0` (6), `0x1403F29E0` (1 Magic, 2, 3, 4, 5, 7, 8,
  9, 10, 12; also submenu 4's and 8's counts `+0x238` / `+0x438` and 8's row states `+0x441`),
  `0x1403F3860` (3, 6, 7's count `+0x3B8`), `0x1403947A0` (10), `0x140403F10` (14). `NUMBER_SITES`.
- **17 short sites**: submenu 0 as `lea r,[obj+0x28]` (7 in `0x1403F29E0`, 1 in `0x1403F27D0`), the
  reset loop over all 19 (`0x1403F28C0`: start and the `0x80` step), "submenu `n`" as `n << 7`
  (`0x1403F3AF0`, `0x1403F2470`, the two pushes `0x1403B1F30` / `0x1403B2800`) and from a submenu's
  owner (`0x1403B0F20`, `0x1403B1840`, `0x1403B19B0`, which read `+0x38` count and `+0x41` row
  states of the command's own submenu to grey it out). `SHORT_SITES`.
- Not submenus, though near: the 7 shortcut-like entries at `+0x9B0` (stride `0x24`;
  `0x1403B25C0`, `0x1403B2660`, `0x1403B2850`), the 8 pointers at `+0xB80` filled in step with
  submenu 4's rows (`0x1403F29E0`, read `0x1403F22E0`), the 2 entries of `0x14` at `+0xB98` filled
  in step with submenu 8's rows. These stay as KH2 has them: submenus 4 and 8 never get more rows
  than KH2 gives them (only Magic grows).
- No code works out a submenu's number from its address (no shift by 7 back), and the stack's
  depth is computed from `+0xAF0` (unchanged).

## Step 2: the move (t77)

Our memory (`0x2000` bytes, within 2 GB of KH2's code): 19 slots of `0xC0`; slot `n`'s header at
`+0x18` (8 mod 16, as KH2's), cursor row at header `-0x0C`, cursor command at `-0x08`, 12 rows at
`+0x14`; then the stubs. Once KH2 has built the object (or, when we load before KH2's static
init, right after the ctor that init calls: `0x140024571` is pointed at a wrapper), each submenu
is copied over (header, 8 rows, cursor; rows 8–11 as KH2's ctor makes a row) and the submenu stack's
pointers moved, then all 107 sites are written. Every site's bytes are checked first: if any
differs, nothing is changed. The stubs were run in an emulator against KH2's own code for all 19
submenus (Checked (exe, emulated)); in-game: Assumption until tested.

## Step 3: the HUD draws up to 16 sprites a level (t78; Checked (exe + decomp), emulated)

- **What KH2 has.** The command HUD `dk::COMMAND_DRAW` (vtable `0x1405B1C28`; one object, made
  by `0x14015E410` with `0x670` bytes, pointer at `0x140ABC508`; its menu object at `+0x398`)
  keeps 4 menu levels × 9 sprite slots (`+0x20 + 8 (9 level + k)`) and a flag byte for each
  (`+0x140 + 9 level + k`); `0x140ABC4F8` (4 ints) counts the slots used this frame per level, zeroed
  each frame by `0x14015E800`. Every sprite of a level takes the next slot (`0x14015D140`): the
  title (`0x14015B6D0`), the rows, the cursor row (`0x14015BA00`), and extras: level 0 adds up to 5
  (`0x14015BC30`, `0x14015D670`, `0x14015DF90`), a deeper level 1 (`0x14015BF80`). So a submenu of 12
  rows needs at least 14 slots. Sprites come from a pool of 52 `dk::COMMAND_ONE` (`0x14015C660`,
  free count `+0xC570`); a slot keeps its sprite until its level shrinks (`0x14015AD20`,
  `0x14015AED0`, `0x14015AF50`, `0x14015CBE0` free them).
- **Rows past 8.** A row's place comes from the count: rows at `y = 0x19A − 0x16 × count + 0x16 i`,
  the title at `0x184 − 0x16 × count` (`0x14015B6D0`), the frame at `0x179 − 0x16 × count`
  (`0x14015D670`): the menu grows upward by itself. 12 rows put the title at `y = 0x7C`.
- **The change.** The object is made `0x8B0` bytes; slots move to `+0x670` (16 a level) and flags to
  `+0x870`; KH2's code addresses them there: `level × 9` → `imul r, r, 16` (7 places; each checked
  that the flags it changes are written again before they are read), the flags' `disp32` `0x140`
  → `0x870` (7), the first-slot sums and loop counts in the four free functions, the level step in
  `0x14015E800` (`add rcx,0x48` → `sub rcx,-0x80`), the clear `0x14015C5D0` rewritten (KH2's arrays
  and ours, the per-level ints to −1), the object's size, and three short sites through stubs
  (`0x14015D1B2` / `0x14015D302` read and store a slot, `0x14015EA5F` a level's first slot). If the
  HUD already exists when we load, nothing at all is changed.
- **Checked (emulated)**: the frees for every level range and random counts, the level-0 compare
  free, the clear, against KH2's own code. The pool (52) is enough: only the drawn levels hold
  sprites (root ≤ 11, a 12-row submenu 14). Assumption until tested in game.

## Step 4a: KH2's magic elements 6 → 9, in KH1's order (t79; Checked (exe + decomp), emulated)

- **What KH2 has.** One entry of `0x50` per magic element at `0x142A23A80`, 6 of them (Fire 0,
  Blizzard 1, Thunder 2, Cure 3, Magnet 4, Reflect 5): `+0x00` the loaded spell file, `+0x08` /
  `+0x10` its parts, `+0x18` the effect set (ctor `0x1402C4050`), `+0x48` the MAGC row of the
  level Sora has. Made by the static init `0x140025690` (dtor `0x140577600`); filled by
  `0x1403C5DC0` (the MAGC row per element: rows with that element id, the highest level the
  element's item count reaches) and `0x1403C5A90` (loads each element's file); read by the cast
  `0x1403C5170` (element by command id), `0x1403C5900` (element → command), `0x1403C5940` /
  `0x1403C5C50`; freed by `0x1403C5440` / `0x1403C5750`; `0x1403C5CE0` preloads.
  Element → item: `0x1403C4840`, a table of 6 at `0x1405C5D18` (21, 22, 23, 24, 87, 88); the
  item's count (`item +0x12` = its slot in `0x1409ACE30`) is the level.
- **Menus.** The command menu's Magic list (`+0xB68`, room for 6 and a 0; `0x1403F27D0`, handed
  to `0x1403B1840` at `0x1403F2B8F`) and the pause menu's Magic page (`0x14035B440`, items of the
  6 elements into a stack array, then its 18-entry output) both go by element number.
- **The change.** The 9 entries, the item table and the Magic list (room for 9 and the 0) are in
  our memory; every reference (12 RIP-relative, 2 image-relative, the two ends) and every count
  of 6 (11) is changed; the command menu's and the pause menu's loops take element
  `ORDER[i]` = Fire, Blizzard, Thunder, Cure, **Gravity (6), Stop (7), Aero (8)**, Magnet, Reflect
  (three stubs). Items 84, 85, 86 for the new elements (unused ids in KH2's `item` table, Checked
  (files)). `0x1403F2910` and `0x1403F3460` (no caller) are left alone.
- **Checked (emulated)**: the command menu's list loop and the pause menu's first loop on the
  patched code (KH1's order, an element without a spell skipped).
- Nothing changes on screen until the new elements have MAGC, item and command rows (step 4b).

## Step 4b: Gravity, Stop and Aero in KH2's tables and text (t80; Checked (files + decomp))

- **Commands** (`kh2-hook::magic_rows`): after KH2's table loader, its `cmd` table (pointer
  `0x142A161E8`, `bsearch` by id) is copied with 9 rows: 797–799 Gravity / Gravira / Graviga,
  800–802 Stop / Stopra / Stopga, 803–805 Aero / Aerora / Aeroga, each Thunder's row of that level
  (51, 123, 124) with element 6 / 7 / 8 in `+4`, our name in `+8`, KH2's cost 18 (Assumption: no
  KH2 cost exists for these; with `mp = kh1` KH1's 100 / 200 / 200 go in, `kh1mp::KH1_SPELL_COSTS`).
- **Items**: `item` (pointer `0x142A25370`) copied with items 84 / 85 / 86 (kind `0x12`, element,
  names, the level-1 command, count slots 75 / 76 / 77), on Fire's row. Both copies only once
  step 4a's patches are all in.
- **Names** (`kh1-assets::spell_names`): the build writes `msg/us/sys.bar` (with `magic = kh1`):
  KH2's English system text with ours at ids `0x5700`–`0x570E` (KH2's letters, Checked (files):
  `A` `0x2E`, `a` `0x9A`, space `0x01`, `.` `0x4F`; the items' "Use from the Magic Command." line
  copied from KH2's). Other languages: not yet (they'd show no name).
- Still to come for the spells to be cast: their MAGC rows (`00battle.bin` `magc`, pointer
  `0x142A23C68`: element, level, file name, item, command, motions) and their spell files with
  KH1's behaviour, one spell a version.

## Step 4c: Gravity (t81)

**Built** (owner's answers: the Drive Forms cast KH1's version, "a"; every enemy takes KH1's full
share, "(c) Everyone full"):
- `kh1_assets::spells` `Kind::Gravity`: `magic/GRAVITY_1..3.mag` built on KH2's
  `BLIZZARD_n.mag` (its PAX and script stay in the file unused: the trigger stubs send every sub
  to ours, `kh1_only`), KH1's `xm_gravity0n.mag` effects added (elements 100 charge, 108 hand,
  102 marker, 101 sphere, 104 / 105 impact). Script: at the cast the target and the offset, a
  marker thread (28 frames, 240 above the point); at the release the hand effect and the sphere
  from 240 above, 230 down over N, scale 0.01 → S (`trap_effect_set_scale(effect, &vec)`,
  `0x14041E000`, Checked (decomp)); at N the impact effect (4 if a 100 ray finds the floor, else
  5), every enemy part within 170 × S (KH2's target searcher, nearest first, one strike each) struck
  once with ATKP 2066; the sphere × 1.02 a frame for 20, then killed. Test VM: the marker, the
  start and the impact frame (28 + N), the strikes (in reach, one per enemy, not the dead), the
  scales, the no-target point (`kh1_gravity_falls_and_crushes_as_kh1s`).
- `kh2-hook::magic_rows`: ATKP 2066 (Blizzard's 248 rows, every sub, type 7, power 40 / 55 / 70 by
  level, no element, hit group 3); MAGC rows for element 6 (Blizzard's base rows, `GRAVITY_n`,
  item 84, commands 797..799), only when the files are in the mod folder.
- `win/gravity.rs`: KH2's damage function `0x1403C12F0` (prologue `40 53 48 83 EC 40`) computes
  type 7 as KH1's: current HP × (power + 5 × max MP bars)%, at least 1; KH2's general resistance 0
  (scripted invulnerability) still stops it (Assumption).
- Debug `new_spells = 1..3` in `kh2-kh1.ini` gives Gravity at that level (magic = kh1) until the
  unlocks (step 5).
- Not yet: KH1's slow on enemies the falling sphere touches (time × 0.8; KH2 has no per-enemy
  time scale found yet), KH1's sounds, the struck part's bodies (KH2 part centres stand in).

### Findings

- **Owner (2026-11-14, "a for now")**: the Drive Forms that cast magic cast Gravity, Stop and Aero
  as base Sora does (KH1's version), so our spell files run KH1's script for every sub.
- **KH1's Gravity** (KH1-in-ReCom `recom-m7-magic.md` §7.8, Checked (files + decomp) there; the
  owner's `xm_gravity01..03.mag` disassembled here agree: entry `func_8cc(…, S, N)` with
  (1, 60) / (1.5, 70) / (1.6, 60), `func_3ea`'s 170 / −230 / 0.01 / 80 / 20 / 1.02 / 28):
  cast on Blizzard's motions (KH1 55 / 80, already on KH2's Blizzard rows 59 / 61 with the release
  at 28, t69); charge effect 0 on the Keyblade; a marker (effect 2) 240 above the point for 28
  frames (point: the target's part if alive, else the caster + 200 along his facing, the offset
  taken at the cast); release: hand effect 1; the sphere (effect 3) from 240 above the point, 230
  down over N frames, scale 0.01 → S; each frame enemies touching 170 × scale are slowed (time ×
  0.8: not in KH2 yet); impact: one attack struck on every enemy touching 170 × S at the centre,
  floor-contact effect 4 (near the floor) or 5, then a 20-frame fade growing × 1.02 a frame, then
  killed. MP 100.
- **Damage** (KH1 `0x1402BFA80`, element 8): current HP × (power + 5 × max MP bars)% × resist[8],
  power 40 / 55 / 70; no Defence, no cap; at least 1. In KH2: KH2's damage function
  `0x1403C12F0(target status, attacker, ATKP row, …)` (Checked (decomp): ATKP `+4` type (5, 6 the
  heals), `+5` stat source, `+6` power, `+9` element; status `+0` HP, `+4` max HP, `+0x1A8 + e`
  element resistances, `+0x1AE` general) gets a new ATKP type for Gravity, computed KH1's way by a
  wrapper on that function. KH2's enemies have no Gravity resistance: owner question (bosses).

## The plan (one step a version)

1. (done, t77) Map every read and write of a submenu's tail (`+0x74`, `+0x78`) and every "submenu by number"
   site (the list above is the start), and the HUD's per-level slots and row placement. In this file.
2. (done, t77) Move the 19 submenus out of the object into a block with **12 rows each and the cursor fields
   just before the header** (`−0x0C` cursor row, `−0x08` cursor command): a tail access keeps its
   8-bit displacement (only the byte changes: `0x74` → `0xF4`, `0x78` → `0xF8`), rows stay at
   `+0x14 + 12 × i`, a slot is `0xC0` bytes. Every "submenu by number" site moved to the block;
   KH2's ctor is left as it is (it runs once, before the move). The old inline area stays unused (a write there that was missed harms
   nothing). The HUD still takes 8 until step 3, so Magic keeps 8 until then.
3. (done, t78) The HUD's levels to 16 slots (title + 12 rows + extras); rows past 8 place themselves.
4. KH2's six magic elements to nine (`0x142A23A80` arrays and their loops: `0x1403C5170`, `0x1403C5440`,
   `0x1403C5750`, `0x1403C5900`, `0x1403C5940`, `0x1403C5A90`, `0x1403C5C50`, `0x1403C5CE0`,
   `0x1403C5DC0`, `0x140025690`; the Magic list `+0xB68`), new MAGC / item / command rows, the
   spells, the menu order.
5. The unlocks. **Approved by the owner (2026-11-14)**, each as the second bonus of KH2's reward
   row (every Sora row has `BonusItem2 = 0`): Gravity at Phil's Training (OC1), Oogie Boogie (HT1),
   Hades (OC2); Stop at the Cornerstone of Light (TR), Pete (TR), Luxord; Aero at Dark Thorn (BC1),
   Storm Rider (LoD2), Transport to Remembrance.
