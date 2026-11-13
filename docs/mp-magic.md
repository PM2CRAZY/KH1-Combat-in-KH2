# Phase 4: KH1's MP and magic inside KH2 (plan and progress)

`mp = kh1`, `magic = kh1`. Native as the earlier phases: KH1's numbers and rules in KH2's own
status fields, tables, menus and spell scripts. Research basis: the sister project's
`kh1-kh2-mp.md`, `kh1-magic-native.md`, `kh1-attack-elements.md` (KH1 Steam addresses, re-checked
here where used).

## The owner's choices (2026-11-13)

1. **The pool: KH1's bars.** Max MP and spell costs are KH1's, shown on KH2's MP bar at 100 a bar;
   KH2's level-ups and MP Ups don't raise it.
2. **Running out: KH1's.** No MP Charge; MP comes back from the hits Sora lands (and orbs), as KH1.
   Limits still work and still empty MP.
3. **Spells: all nine.** KH1's Fire, Blizzard, Thunder, Cure, Gravity, Stop and Aero, plus KH2's
   Magnet and Reflect.

4. **Max MP grows (the owner, 2026-11-13): KH1's levels + KH2's story.** KH1's start and KH1's +1
   bar at its levels; each of KH2's own MP-up rewards (bonus levels) gives +1 bar instead of its
   KH2 amount.

## What is known

**KH2** (the sister project's `kh1-kh2-mp.md` §2, Checked (decomp) there, Steam): the player's
status at `object + 0x5C0`: `+0x180` MP, `+0x184` max MP (ints), `+0x1BC` / `+0x1C0` the MP
Charge's time left / length. `0x1403D3CF0 (object, delta, flag)` changes MP: a change that would go
below 0 starts the MP Charge (`0x1403C20A0`) and isn't paid. Spell costs are the command's cost byte
(03system `cmd`: Fire 12, Blizzard 15, Thunder 18, Cure 255 "all", Reflect 10), paid through
`0x1403D7E90` / `0x1403D8B40`. Hits Sora lands give no MP (no hit-dealt caller). Orbs: the prize
table `0x1403DCA50` (`0x1405C7425` the MP of each prize). The hit record (Phase 3, t54): `0x1403CE740`.

**KH1** (Checked (decomp) in the sister notes; Checked (files) here where marked):
- MP is whole bars (stats `+0x44`, max `+0x48`) and a yellow gauge (`+0xBE`, units). The next bar
  needs (MP + 1) × 30 gauge units (`0x1402A4CB0`).
- A hit Sora lands adds attack data `+0x2C` units, × (1 + level difference × 20 %), within 1 and 250
  % of it, + 150 % with MP Haste (`0x1402BF330`). Checked (files): his plain swings 3
  (`btltbl.bin` `0x4B78` / `0x4BE8`), his finishers 6 (`0x4C58`).
- A spell's cost is its attack data `+0x30` in hundredths of a bar (`0x14028CE40`); paid by
  `0x1402A5B60` (100 and up: whole bars; under: gauge units).
- Orbs: small 2 units, large 15 (`0x1402AC080`). MP Rage: damage taken × copies, in units.
- **KH1's level-ups** (Checked (decomp) `0x1402B51B0` / `0x1402B5660`; Checked (files)): going from
  level L to L + 1 applies Sora's level table byte `L - 1` (`btltbl.bin` `+0x3AC0`, its offset at
  KH1 exe `0x140528CC0`, the file loaded at `0x142D22D40`): 1 = +3 max HP, 2 = +1 max MP (bar),
  3 = +2 AP, 4 = +2 Strength, 5 = +2 Defense, 6 = another. Max MP +1 at levels 12, 24, 36, 48 and
  60. (`0x140528D10[dream weapon]` points at the three ability tables, `+0x3B28` / `+0x3B90` /
  `+0x3BF8`.) Not found yet: Sora's starting max MP (the new-game record) and KH1's story MP ups
  (Assumption: event scripts; not needed for the owner's choice 4).

**KH2's max MP** (Checked (decomp) `0x1403C1D90`, the status build from the save's character
record): record `+6` MP, `+7` max MP (bytes), into status `+0x180` / `+0x184`. So KH2's max MP can't
pass 255: KH1's bars go on it at **10 a bar** (not the 100 the plan first said): up to 25 bars.
KH2's MP-up rewards (Checked (files), `00battle.bin` `bons`, character 1): five, rewards 23, 26,
29 (+10 each), 51 (+40), 68 (+10), so record `+7` − 100 tells how many he has (prefix sums 10, 20,
30, 70, 80; Assumption: he gets them in reward order).

## Plan (one system per version)

1. **KH1's pool on KH2's MP fields.** KH2's MP = bars × 10 + the yellow gauge as a share of the
   next bar; max MP = KH1's max bars × 10 (KH1's per-level max: its level-up table, to be read;
   question if KH2's levels can't map onto it). MP Charge never starts for base Sora (the "below
   0" branch of `0x1403D3CF0` refuses instead), and the magic menu greys a spell he can't pay for.
2. **KH1's MP from hits.** At the hit record (`0x1403CE740`), a hit by Sora on an enemy adds KH1's
   units for that attack (KH1's swing / finisher / spell values by KH2 attack kind), turned into
   KH2's MP by the scale above.
3. **KH1's orbs and MP Rage.** The prize table's MP values and MP Rage on KH1's units.
4. **KH1's spell costs.** The `cmd` costs (in KH2 MP, 100 a bar) for the spells.
5. **KH1's spells, one per version:** Fire, Blizzard, Thunder, Cure as KH1's in KH2's magic system
   (KH2's spell scripts and effects; KH1's projectile, timing, damage and effects), then Gravity,
   Stop and Aero as new spells (new magic rows, commands, menu entries), Magnet and Reflect kept
   KH2's.

## t60: KH1's pool on KH2's MP (step 1)

`win/mp.rs` (logic in `kh1mp.rs`, tested), base Sora, `mp = kh1`:
- **Pool:** after KH2's status build `0x1403C1D90`, Sora's max MP is KH1's bars × 10: KH1's start,
  +1 at levels 12 / 24 / 36 / 48 / 60 (record `+0xF`), +1 per KH2 MP-up reward (record `+7`). His
  MP shows the bars and the yellow gauge's share of the next one; the gauge itself is kept in the
  status block's MP Charge time `+0x1BC` (KH1 has no charge; its length `+0x1C0` stays 0).
- **Costs:** KH1's (Checked (files), attack data `+0x30`: Fire 30, Blizzard 30 units, Thunder 100,
  Cure 100 = one bar) written into KH2's `cmd` rows (execute 2, arguments 0..3) after the table
  loader (`cmd` at `0x142A161E8`, rows of 0x30, cost `+0x1C`; Checked (decomp) / (files)). Magnet
  and Reflect keep their bytes (30, 10), read as KH1 units. 255 ("all": KH2's Limits) empties MP
  and needs one bar.
- **Paying:** the MP change `0x1403D3CF0`: a cost goes through KH1's check and payment (units
  break a bar into `bars × 30` units); what he can't pay isn't paid and starts nothing; the
  result is handed to KH2's own function as a change from his MP now, so KH2 never starts the MP
  Charge for him. A gain is KH1's gauge units (orbs, MP Rage, items as units for now: step 3).
- **Menu:** `0x1403D88E0` greys (5) a command KH1's check refuses (KH2's 2 meant "will start the
  charge").
- **Scripts' charge:** `0x1403D4A20` (a script's MP Charge, the other caller of the charge start)
  empties his MP instead.
- Assumptions: KH1's start is 2 bars (not yet read from KH1's new-game record); KH2's MP-up
  rewards come in reward order. Hits don't refill yet (step 2). Not checked in game.

## t61: KH1's MP from hits (step 2)

KH2 makes a hit record for every hit (`0x1403CE740 (attack, ATKP, target, ..)`, from
`0x1403D12C0`; the attack's owner its handle `+0x10`, the record's `+0x28` the damage; Checked
(decomp)). `win/mp.rs` hooks it: a hit base Sora lands that does damage adds KH1's units through
the MP change (t60): 6 for his finishers (ATKP 129 / 130 / 133 / 134, KH1's finishers in Phase 2),
0 for a magic-stat attack (ATKP `+5` = 1; KH1's spells give none: their attack data `+0x2C` is 0,
Checked (files)), else 3 (KH1's plain swings, `+0x2C`, Checked (files)). KH1's level scaling (× (1 +
level difference × 20 %), within 1 unit and 250 %) isn't applied: KH2's enemies have no KH1 level
(Assumption, ×1). MP Haste's + 150 % is step 3's. Not checked in game.

## Camera (the owner, 2026-11-13: "bring in KH1's Camera, and have the setting ... in the config")

The Config page already has the Camera row (`camera = kh2 / kh1`, t40); it does nothing yet. KH1's
field and lock-on camera go in as a KH2 camera mode, from the sister project's `kh1-camera.md`
(KH1's camera, Checked (decomp) there) and `kh1-kh2-camera.md`. Research next.
