# KH2's Config menu (research, for the "KH1 in KH2" button)

The owner (2026-11-12): "Have a similar config menu button as the KH2 combat in KH1 mod, where the
player can switch between both games' jumps/physics, etc". The model is the sister mod's KH1 work
(`KH2-Combat-in-KH1` `docs/kh1-config-menu.md` §6). Steam KH2. Tags as elsewhere.

## Found

- The pause menu is KH2's camp menu: `camp.2ld`, its update `0x140303CD0`, which each frame runs
  every sub-screen's update (`0x140323FA0`, `0x14034E470`, `0x1403564C0`, `0x1403201F0`,
  `0x14035B110`, `0x140360750`, `0x140365120`, `0x140366810`, `0x140368600`, `0x1403521B0`).
  Checked (decomp). Which one is Config: not yet known.
- `YS::CONFIG::WIDGET` / `EDIT2` (`0x14039EE10` / `0x14039EC20`) are a developer config (one row is
  "debug"), not the player's. Checked (decomp).
- The Config text, `msg/us/sys.bar` (the player's `msg_us_sys.bar`; KH2's text encoding: A–Z
  0x2E–0x47, a–z 0x9A–0xB3, space 0x01), Checked (files): `0x3617` Config; `0x3717` Field Camera,
  `0x3719` Navigational Map, `0x371A` Vibration, `0x371C` / `0x3723` Command Menu, `0x3722`
  Camera, `0x372C` / `0x372D` vibration help lines; `0x42F5` Camera Up/Down, `0x42F6` Camera
  Left/Right, `0x42F7` Summon Effects, `0x4337`..`0x433B` and `0x4381` vibration (a second set).
  None of these ids is a plain constant in the code or an aligned table entry in the exe's data
  (searched, Checked (exe / decomp)): the rows likely come from the camp layout or a computed id.

## The Config screen (found, Checked (decomp / exe))

- It is camp mode `0x24`. Its pieces: `0x140365580` opens it (reads the saved options into the
  row values, sets the list's row count 8: `0x1402F8F10 (list, 8)`, `0x1402F9140 (list, 8)`),
  `0x140365060` steps it (the current row's help line), `0x1403657E0` draws the current row's
  value, `0x140365460` applies (writes the row values back), `0x1403651A0` re-reads them. The list
  widget is `0x140BEECD8` (`0x1402F7FB0` its current row), the row values a byte each at the
  array `0x140BF0150` points to (allocated by `0x140365390`).
- The saved options are bits of save `+0x41A4` (OpenKH `SaveDataFinalMix`: vibration bit 0,
  navigational map 3, field camera 4, right stick 5, command menu 6, camera left/right 7, camera
  up/down 8), read and written by small functions at `0x1402E4FA0`..`0x1402E5400`.
- The row table is static, at `0x1405BC340`, 8 rows of 0x14 bytes: `+0` the option count (2, 3, or
  1 for the last), `+2` the row's label (a `sys.bar` text id with bit 0x8000: `0xB717` is `0x3717`
  "Field Camera"), `+4` up to four value labels, `+0xC` up to four help lines (one per value).
  Rows: 0 Field Camera, 1 Camera Up/Down, 2 Camera Left/Right, 3 Summon Effects (3 values), 4
  Navigational Map, 5 Vibration, 6 Command Menu, 7 (`0x371D`, one row of four values). The text is
  drawn from these ids into 32 text widgets in the camp object (`+0x4360`, 0x220 each, 8 rows × 4
  values: `0x140365330`).

## Plan (open: how the player reaches our page; see the owner's answer)

Our 8 systems (movement, combos, defence, mp, magic, abilities, camera, model) are 8 rows: the
same count as KH2's page. So our page can be KH2's own Config screen with our row table, our
values and our apply (write the settings, then switch the systems live) swapped in, and our row
and value labels as new `sys.bar` texts the build step adds. No new widgets: KH2's 32 text widgets
and its list code draw ours.

## Next

1. Which camp sub-screen is Config: find the one whose rows read the saved option values (the
   vibration value is the easiest to follow: what turns the rumble off).
2. Its row table, the row draw and the left / right handler; its help line.
3. Where KH2 keeps the Config values in the save, and whether the save has spare room for ours.
4. Then, one version each: the button, our lists, live switching (the KH1 hooks always installed,
   each checking its row; today they install only when the ini says `kh1` at start).

## The owner's choice and step 1 (t39)

The owner: a 9th row that opens our page; the settings stay in `kh2-kh1.ini`.

t39 (`win/config_menu.rs`): the row. The 15 `lea`s that reach the row table (`0x140364974` …
`0x140365C67`, Checked (exe bytes)) point at a 9-row copy (KH2's 8, then ours: one value, label
"KH1 in KH2", value "Open", a help line); the list's row count (`mov edx, 8` at `0x1403656EF`,
`0x1402F8F10` sets the list's `+0x16`) is 9, so the list scrolls (the draw `0x140365A20` reads
the table at scroll + row); the row values array (`0x140365390`, `new(8)`) is 16 bytes, and our
row's byte is set to 0 when the screen opens (`0x140365580`, hooked). Our texts come from a hook
on the text lookup `0x1403AD9A0` (ids `0x7F00..`, KH2's text encoding after OpenKH). Nothing
happens on our row yet.

## t40: L1 / R1 flip the page (the owner's change)

The owner: "make the config menu tab between main config and mod config by using r1 and l1". The
9th row (t39) is gone; `win/config_menu.rs` now:
- keeps KH2's table references on a copy of ours (the 15 `lea`s), holding KH2's 8 rows or our
  page's 9: Movement, Combos, Defence, MP, Magic, Abilities, Camera on rows 0..6, a help line on
  row 7 (KH2 draws row 7 as one value), Player Model on row 8 (the list scrolls);
- the step `0x140364D70` (hooked) flips the page on an L1 / R1 press (the pad record's raw word,
  `*(*(0x140BF31A0 + 0x48))`, PS2 bits L1 0x400 / R1 0x800: the sister project's
  `docs/kh2-pad.md`, Assumption here) and redraws with KH2's own draw `0x140365A20` and help
  `0x1403659E0`; the list's row count (`+0x16`), cursor and scroll are set for the page;
- the values array (16 bytes, `0x140365390`) holds the page's values: KH2's kept aside while ours
  are up; a change on our row 5 gives the vibration back its KH2 value;
- the apply `0x140365460` (hooked): if our page is up, our values go into `kh2-kh1.ini` (read at
  the next launch) and KH2's values and rows come back before KH2's apply saves them;
- the open `0x140365580` (hooked) starts on KH2's page with our values read from the ini.
