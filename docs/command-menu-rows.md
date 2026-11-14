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
