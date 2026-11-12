# Phase 2: KH1's ground and air combos inside KH2 (plan)

`combos = kh1`. Native: KH2's own combo picker plays KH1's combo from KH1's rows, KH1's motions,
KH1's hit data and windows. Steam addresses. Tags as elsewhere.

## KH2's side (Checked (decomp) unless tagged)

- The picker `0x140402C50` walks Sora's `ptya` set in order (rows of 0x44 bytes: id, kind, `sub`,
  combo offset, flags, motion, next motion, jump / jump max / jump min, speed window, near / far,
  low / high, facing window, blend, distance adjust, ability, score; the OG rewrite's reading of the
  files, Checked (files) there) and takes the first valid row (a bit in its valid mask `+0x60`) whose
  motion isn't the one playing (`actor +0x180`), else the first valid row's `sub`; `0x140402D20`
  runs it, with a fallback (row 5 ground, 9 air). Called from the attack press `0x1403A76B0`.
- `ptya` is `00battle.bin`'s, kept at `0x142AE57B8` (`0x1403EB780`), a set per moveset (1 = Sora,
  read by `0x1403EB760` with the object's moveset `+0x4E`).
- Combo length: 3 ground / 3 air, Combo Plus / Air Combo Plus +1, Negative Combo -1, finishers 1 +
  Finishing Plus (the OG rewrite; to re-check on Steam).

## KH1's side (the sister project's notes, Steam; to re-check in the KH1 decomp per version)

- The Attack step `0x1402A7510` counts the press (combo window, the swing connected or Combo Master),
  then the ground picker `0x1402A6EE0` / air picker `0x1402A6C70`, and runs the row `0x1402A68F0`.
- Ground: marks rows in a flag array (66 rows): the finisher (8) at the stats' `+0xD4` (the combo
  length); ability finishers 3 / 4 / 5 by ability bits 27..29 (30% each, random `0x14031BA30`), 7 by
  bit 26, 6 by bit 3 with the finisher; the dash at a target over 160 away (0 with ability bit 1,
  else 1); 9 / 10 by the target's height; 11 / 13 / 14 by the targets around and ahead; 2 at a drop
  ahead. Walks the rows in order, takes the first marked one whose request isn't playing; none: 13.
- Air: 16 its finisher at `+0xD5`, 15 a finisher with ability bit 25, 17 a target more than 100 above,
  18 its plain swing; walked from 15; none: 18.
- Rows: Sora's 66 rows of 0x14 in the battle data block (`0x142D22D40 + 0xA9F0`): request, trail,
  effect, attack data offset, lunge, sound / voice bytes. Attack data 0x70 bytes (`btltbl.bin`).

The two pickers already work alike (first marked row in table order, not the one playing): KH1's
rows go in as Sora's own `ptya` rows, and what KH1 tests that `ptya` can't say (the 30% chances, the
ability bits, KH1's combo length) goes into KH2's row check as a wrapper.

## Versions (each needs the previous)

1. **Asset build, motions**: KH1's Sora motions (KH1's `xa_ex_0010.mset`, read from the KH1 install)
   retargeted onto KH2 Sora's skeleton, written as new motions into a copy of KH2's `P_EX100.mset`
   in the mod folder (built once on first launch; Panacea serves it in place of the game's).
2. **KH1's rows in `ptya`**: Sora's set gets KH1's rows (its motions from step 1) after KH2's; at
   `combos = kh1` the picker sees only KH1's (the valid mask), in KH1's order.
3. **KH1's row tests**: the 30% ability finishers, the ability bits, KH1's combo length (`+0xD4` /
   `+0xD5`), the dash and height rows, in KH2's row check.
4. **KH1's hit data**: KH1's attack data as `atkp` rows (power, knockback, re-hit), and KH1's hit
   windows as KH2 motion triggers on the motions of step 1; the Keyblade's hit shape.
5. **KH1's combo windows and lunge**: when the next press is taken, how far a swing lunges.
6. Trails, effects, sounds of KH1's swings (with Phase 7's effect and sound work).

## Step 2 (t13): KH1's swings on Sora's own `ptya` rows

Checked (decomp): Sora's set is his `+0xD50` (PARTY constructor `0x1403D5840`, from `ptya` by his
moveset), read by the selector `0x1404028D0`; the row check `0x140402750` (motion set, ability
`+0x40` equipped, flag bit 0 air, flag bit 2 the finisher test `0x1403D7780 (actor, combo offset)`:
count `+0xD74` + offset ≥ length `+0x200` / `+0x201`; else `0x1403D7850`: count < length).
Checked (files), the owner's `00battle.bin`: Sora's set (moveset 1) has 37 rows; the plain ones
(ability 0): ground swings ids 4 (152, kind 2), 5 (151, kind 3, the fallback), 6 (153, kind 1,
180..300 away); finishers 2 (154) and 3 (155); the air swing 10 (182) and fallback 9 (181); air
finishers 7 (184) and 8 (183); the jump approaches 0 (182) and 1 (181).
KH1's rows (`btltbl.bin +0xA9F0`, 0x14 each, Checked (files)): 13 `0xC8` (plain), 11 `0xC9`,
14 `0xCA`, 8 `0xCB` (finisher); air 18 `0xCC` (plain), 17 `0xCD`, 16 `0xCE` (finisher), 15 `0xD1`
(ability finisher); dashes 0 / 1; lunges 0 on the ground, 20 / 40 in the air.

t13 (`win/combos.rs`): after KH2's table loader `0x1403F4C30`, the plain rows' motions become
KH1's (210..216); the ability rows, windows and order stay KH2's. Only when this launch's build
made the motions. Open (Assumptions): KH1 request `0xC8 + n` → KH1 motion `98 + n`; KH1's target
tests vs KH2's windows; KH1's combo length (stats `+0xD4` / `+0xD5`, runtime: a KH1 combo trace
reads it); the jump approaches still KH2's.
