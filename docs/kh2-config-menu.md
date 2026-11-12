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

## Next

1. Which camp sub-screen is Config: find the one whose rows read the saved option values (the
   vibration value is the easiest to follow: what turns the rumble off).
2. Its row table, the row draw and the left / right handler; its help line.
3. Where KH2 keeps the Config values in the save, and whether the save has spare room for ours.
4. Then, one version each: the button, our lists, live switching (the KH1 hooks always installed,
   each checking its row; today they install only when the ini says `kh1` at start).
