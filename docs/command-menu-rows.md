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

## The plan (one step a version)

1. (done, t77) Map every read and write of a submenu's tail (`+0x74`, `+0x78`) and every "submenu by number"
   site (the list above is the start), and the HUD's per-level slots and row placement. In this file.
2. (done, t77) Move the 19 submenus out of the object into a block with **12 rows each and the cursor fields
   just before the header** (`−0x0C` cursor row, `−0x08` cursor command): a tail access keeps its
   8-bit displacement (only the byte changes: `0x74` → `0xF4`, `0x78` → `0xF8`), rows stay at
   `+0x14 + 12 × i`, a slot is `0xC0` bytes. Every "submenu by number" site moved to the block;
   KH2's ctor is left as it is (it runs once, before the move). The old inline area stays unused (a write there that was missed harms
   nothing). The HUD still takes 8 until step 3, so Magic keeps 8 until then.
3. The HUD's levels to 13 slots (title + 12 rows), and the rows' placement past 8.
4. KH2's six magic elements to nine (`0x142A23A80` arrays and their loops: `0x1403C5170`, `0x1403C5440`,
   `0x1403C5750`, `0x1403C5900`, `0x1403C5940`, `0x1403C5A90`, `0x1403C5C50`, `0x1403C5CE0`,
   `0x1403C5DC0`, `0x140025690`; the Magic list `+0xB68`), new MAGC / item / command rows, the
   spells, the menu order.
5. The unlocks. **Approved by the owner (2026-11-14)**, each as the second bonus of KH2's reward
   row (every Sora row has `BonusItem2 = 0`): Gravity at Phil's Training (OC1), Oogie Boogie (HT1),
   Hades (OC2); Stop at the Cornerstone of Light (TR), Pete (TR), Luxord; Aero at Dark Thorn (BC1),
   Storm Rider (LoD2), Transport to Remembrance.
