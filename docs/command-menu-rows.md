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

## Step 1: the sites (from the Steam exe, linear disassembly, each function read in the decomp)

### Tail accesses (`+0x74` cursor row, `+0x78` cursor command) on a submenu pointer

- `0x1403B1080` (2): `0x1403B10F0`, `0x1403B10F3`
- `0x1403B1140` (9): `0x1403B1140`, `0x1403B1154`, `0x1403B1185`, `0x1403B1192`, `0x1403B11A0`, `0x1403B11AA`, `0x1403B11B2`, `0x1403B11BC`, `0x1403B11C3`
- `0x1403B11E0` (30): `0x1403B1260`, `0x1403B1263`, `0x1403B12AC`, `0x1403B12ED`, `0x1403B1341`, `0x1403B1352`, `0x1403B1383`, `0x1403B1391`, `0x1403B139D`, `0x1403B13A4`, `0x1403B13A7`, `0x1403B13AC`, `0x1403B13AF`, `0x1403B1421`, `0x1403B1424`, `0x1403B1462`, `0x1403B1475`, `0x1403B1486`, `0x1403B149D`, `0x1403B14AB`, `0x1403B14BD`, `0x1403B14C0`, `0x1403B14DA`, `0x1403B14EC`, `0x1403B14FD`, `0x1403B150C`, `0x1403B150F`, `0x1403B151C`, `0x1403B1529`, `0x1403B153D`
- `0x1403B1590` (2): `0x1403B15AA`, `0x1403B15FC`
- `0x1403B1B70` (2): `0x1403B1B70`, `0x1403B1B77`
- `0x1403B1B80` (4): `0x1403B1B85`, `0x1403B1B94`, `0x1403B1B98`, `0x1403B1B9F`
- `0x1403B1E90` (1): `0x1403B1EA7`
- `0x1403B21D0` (1): `0x1403B21E3`
- `0x1403F22E0` (7): `0x1403F232E`, `0x1403F2360`, `0x1403F2376`, `0x1403F23A6`, `0x1403F23C0`, `0x1403F2415`, `0x1403F2427`
- `0x1403F2470` (1): `0x1403F25AB`
- `0x1403F2680` (1): `0x1403F26B8`
- `0x1403F3AF0` (1): `0x1403F3B1B`
- `0x14015B6D0` (2): `0x14015B833`, `0x14015B977`
- `0x14015BA00` (1): `0x14015BA19`

64 sites. Excluded after reading: `0x1403B2340`, `0x14015E800`, `0x1403FD690` (virtual calls `[r+0x78]`), `0x1404016B0`, `0x140401730` (float fields of another object), `0x1403FD8D0` (word fields of another list), every `rsp`-relative one.

### A submenu at a fixed number (`this + 0x28 + 0x80 × n`)

- `0x1403B1C70`: `0x1403B1CA1` lea n=0 (4 B)
- `0x1403B1F30`: `0x1403B1F83` add n=0 (4 B)
- `0x1403B26B0`: `0x1403B26D5` add n=11 (7 B), `0x1403B26ED` lea n=11 (7 B)
- `0x1403B2800`: `0x1403B2820` add n=0 (4 B)
- `0x1403B2A90`: `0x1403B2B02` lea n=14 (7 B)
- `0x1403F2470`: `0x1403F25A4` add n=0 (4 B)
- `0x1403F2680`: `0x1403F2699` lea n=0 (4 B)
- `0x1403F27D0`: `0x1403F27F2` lea n=0 (4 B), `0x1403F27FB` lea n=6 (7 B)
- `0x1403F28C0`: `0x1403F28D2` lea n=0 (4 B)
- `0x1403F29E0`: `0x1403F29FF` add n=10 (7 B), `0x1403F2A2A` add n=4 (7 B), `0x1403F2AA4` lea n=4 (7 B), `0x1403F2AC5` lea n=8 (7 B), `0x1403F2B4D` lea n=8 (7 B), `0x1403F2B83` lea n=1 (7 B), `0x1403F2B96` lea n=1 (7 B), `0x1403F2BA2` lea n=2 (7 B), `0x1403F2C9C` lea n=3 (7 B), `0x1403F2D33` lea n=3 (7 B), `0x1403F2D5C` lea n=9 (7 B), `0x1403F2D84` lea n=7 (7 B), `0x1403F2D97` lea n=7 (7 B), `0x1403F2DAB` lea n=0 (4 B), `0x1403F2DCB` lea n=0 (4 B), `0x1403F2DED` lea n=0 (4 B), `0x1403F2E0E` lea n=12 (7 B), `0x1403F2E21` lea n=12 (7 B), `0x1403F2E4C` lea n=0 (4 B), `0x1403F2E62` lea n=0 (4 B), `0x1403F2E8C` lea n=0 (4 B), `0x1403F2EAB` lea n=0 (4 B), `0x1403F2F0A` lea n=5 (7 B), `0x1403F2FAF` lea n=5 (7 B)
- `0x1403F3110`: `0x1403F3127` add n=3 (7 B), `0x1403F31E0` lea n=3 (7 B)
- `0x1403F3300`: `0x1403F3314` lea n=2 (7 B)
- `0x1403F3460`: `0x1403F346D` add n=1 (7 B), `0x1403F3480` lea n=1 (7 B)
- `0x1403F34A0`: `0x1403F34C1` add n=5 (7 B), `0x1403F355F` lea n=5 (7 B)
- `0x1403F35B0`: `0x1403F35D9` lea n=7 (7 B), `0x1403F35EC` lea n=7 (7 B)
- `0x1403F36C0`: `0x1403F36D7` lea n=0 (4 B), `0x1403F36FB` lea n=0 (4 B), `0x1403F3728` lea n=0 (4 B), `0x1403F374E` lea n=12 (7 B), `0x1403F3761` lea n=12 (7 B), `0x1403F378C` lea n=0 (4 B), `0x1403F37A2` lea n=0 (4 B), `0x1403F37CC` lea n=0 (4 B), `0x1403F37EB` lea n=0 (4 B)
- `0x1403F3AF0`: `0x1403F3B0A` add n=0 (4 B)
- `0x140407B40`: `0x140407C79` lea n=2 (7 B)

55 sites; those of 4 bytes (`lea r,[r+0x28]`, n = 0) can't take a 32-bit displacement in place.


Also to change:
- "Submenu by number" (`owner + 0x80 × i` then `+0x38` / `+0x41`, or `+ 0x28` and the push): `0x1403B1002`,
  `0x1403B193A`, `0x1403B1ACD` (each `shl rax, 7; add rax, [rbx+8]`), `0x1403B1F7F`, `0x1403B281C`,
  `0x1403F259D`, `0x1403F26AE`, `0x1403F3B14`.
- The row ctors' 8: `0x1403B1BB0`, `0x1403B1C70`.
- The HUD: `0x14015B6D0`, `0x14015BA00`, `0x14015E800`, `0x14015CBE0` (9 slots a level), and the row placement
  (not yet read).

**How the sites move** (the plan for step 2, Assumption until built): the block of 12-row submenus
sits within 2 GB of the exe, so a `this + K` with a 32-bit displacement takes the block's place as
a new displacement in place (one instruction, same length; `this` is always the one global). The
4-byte `lea r, [r+0x28]` / `add r, 0x28` sites (submenu 0), the eight "by number" sites and the HUD's
need small jumps out and back. A tail access changes only its displacement byte.

## The plan (one step a version)

1. Map every read and write of a submenu's tail (`+0x74`, `+0x78`) and every "submenu by number"
   site (the list above is the start), and the HUD's per-level slots and row placement. In this file.
2. Move the 19 submenus out of the object into a block with **12 rows each and the cursor fields
   just before the header** (`−0x0C` cursor row, `−0x08` cursor command): a tail access keeps its
   8-bit displacement (only the byte changes: `0x74` → `0xF4`, `0x78` → `0xF8`), rows stay at
   `+0x14 + 12 × i`, a slot is `0xB0` bytes. Every "submenu by number" site moved to the block;
   the row ctors loop 12. The old inline area stays unused (a write there that was missed harms
   nothing). The HUD still takes 8 until step 3, so Magic keeps 8 until then.
3. The HUD's levels to 13 slots (title + 12 rows), and the rows' placement past 8.
4. KH2's six magic elements to nine (`0x142A23A80` arrays and their loops: `0x1403C5170`, `0x1403C5440`,
   `0x1403C5750`, `0x1403C5900`, `0x1403C5940`, `0x1403C5A90`, `0x1403C5C50`, `0x1403C5CE0`,
   `0x1403C5DC0`, `0x140025690`; the Magic list `+0xB68`), new MAGC / item / command rows, the
   spells, the menu order.
5. The unlocks, after the owner approves the list.
