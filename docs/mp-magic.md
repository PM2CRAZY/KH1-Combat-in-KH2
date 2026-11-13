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

## t64: magic rows with no KH2 cost keep it

The t62 log showed the cost patch giving cmd 2 (a magic row, exec 2, arg 2, KH2 cost 0) KH1's
Thunder cost of 100. A row with no cost in KH2 isn't a paid cast (Assumption: it is a menu or
script entry, not the spell itself), so it keeps its 0; only rows KH2 charges for get KH1's cost.
Checked (files: the t62 log's own list of rows).

## t66: KH1's orbs, MP Rage and KH2's MP gains (step 3)

The owner (2026-11-13): "finish phase 4".

Every MP gain reaches KH2's MP change `0x1403D3CF0`; t60 read every gain as KH1 gauge units.
Now (base Sora, `mp = kh1`) it depends on where the gain comes from:
- **Hits** (t61's hit record): KH1's units, as before.
- **MP orbs:** KH2's pick-up `0x1403DCA50 (?, object, prize)` reads the prize's kind (`+0x14`) in
  the row table `0x1405C7420` (8 bytes a row: `+5` MP; rows 5 / 6 the small / large MP orb, 1 / 3
  MP; Checked (decomp + exe)). Hooked (prologue 48 89 5C 24 08): rows 5 / 6 give KH1's orbs, 2 / 15
  gauge units (KH1 `0x1402AC080` kinds 5 / 6 → `0x1402A4AE0`; Checked (decomp)).
- **MP Rage:** KH2's damage-taken handlers `0x1403D5600` / `0x1403D5E50` (Checked (decomp)) add
  `lost / max HP × rate` (`+0x22C`; rate per copy `*(0x142AE5760) + 0x150`, ability 0x19C, from
  `0x140401730`'s ability block at status `+0x1D0`; Checked (decomp)). Hooked (prologues 48 89 5C 24
  08): KH2's gain held back, KH1's given: HP lost × copies in gauge units (KH1 `0x14029BBD0`,
  copies from `+0x184` bits 1..3; Checked (decomp)). Copies = rate / rate per copy.
- **Everything else** (items such as Ether, scripts): KH2's own amount in KH2 MP at 10 a bar
  (`kh1mp::Mp::add_kh2`, tested): KH2's Ether (30) is 3 bars. Assumption: KH1's Ether is the
  same 3 (KH1's item values not read yet).

Not done in step 3: KH2's MP Haste / Hastera / Hastega (abilities 0x19D / 0x1A5 / 0x1A6, KH2's
charge speed `+0x20C`) have nothing to speed up without the MP Charge; KH1's MP Haste is +150 % gauge
per hit (`0x1402BF330`, `btltbl` `+0x95`). Question to the owner. Not checked in game.

## t67: KH2's MP Haste, Hastera and Hastega as KH1's MP Haste, stacked

The owner (2026-11-13, answer): KH2's three "stack" as KH1's MP Haste.

- KH1 (Checked (decomp) `0x1402BF330`): a hit's units = the attack's `+0x2C` × (1 + level
  difference × 20 % + 150 % with MP Haste), at least 1, at most 250 % of `+0x2C`.
- KH2 (Checked (decomp) `0x140401730`, `0x1404016B0`; Checked (files) `03system.bin` `sstm`): MP
  Haste (0x19D), Hastera (0x1A5) and Hastega (0x1A6) add 0.25 / 0.5 / 0.75 (sstm `+0x120` / `+0x124`
  / `+0x128`) to the charge rate, status `+0x20C` (1.0 bare). So they are 1 / 2 / 3 steps of 0.25.
- `kh1mp::hit_units(base, steps)` (tested): each step another 150 %; KH1's 250 % cap rises 150 % a
  step past the first, or a second step would add nothing (KH1's own cap stops at one MP Haste).
  With one step it is KH1's exactly (3 → 7, finishers 6 → 15). The cap's rise is our reading of
  "stack" (Assumption on the owner's meaning).
- KH2's MP Rage per copy (sstm `+0x150`) is 75 (Checked (files)), so t66's copies count is exact.

## KH1's spells in KH2: findings and plan (2026-11-13)

The owner's answers (2026-11-13): spells with **KH1's effects and sounds** (ported into KH2's own
effect engine), KH1's behaviour; MP Haste family stacked (t67).

### Finding: KH2's effect engine already speaks KH1's effect programs

- KH1 keeps every effect as a DPX set of PPP programs (`.mdls` section 1, `.wpn` section 0,
  `.mag` section 1; set header u32 `0x82`, PPP blob header `0x96`; KH1-in-ReCom's
  `recom-m13-kh1-fx.md`, Checked (decomp + files) there).
- KH2's spell file (`magic/FIRE_1.mag`, a BAR: `fire` PAX, type 18, and `fire` BDX, type 3,
  twice) holds a PAX whose DPX part is the same layout: a set header `0x82` at PAX `+0x4A0` and a
  PPP blob `0x96` at `+0x620` (Checked (files): the owner's `FIRE_1.mag`).
- The PPP op tables: KH1's exe names 235 ops, KH2's 242; **KH2's first 235 are KH1's, in the same
  order** (Checked (exe): both Steam exes' op name strings), with the same 7 additions Re:CoM has.
- So KH1's spell effect programs can go into KH2's own spell PAX as they are, played by KH2's
  own effect engine (native: no player of ours). Still to check: KH2's PAX element table (which
  element plays which DPX effect, its bone, flags and sound: OpenKH `Pax.cs`, the sister's
  `kh2-formats/src/pax.rs`), KH1's program sounds (`+0xFC`) against KH2's element sounds, KH1's
  attach codes (`0x8003`, `0x18001`, `0x30`) against KH2's bones / anchors, the 30 Hz tick in both,
  and KH1's HD textures for these sets (tagged by offset in the `.mag`).

### What a KH1 spell needs in KH2 (one spell per version, Fire first)

1. **The cast:** KH1's cast motions (slots 54 / 57 ground, 79 / 82 air; Blizzard 55 / 80, Thunder
   and Cure 56 / 81) on KH2's MAGC cast rows (Fire 56 / 58, Blizzard 59 / 61, Thunder 62 / 64, Cure
   65 / 67), with KH2's magic-cast trigger at KH1's release (Fire: the wind-up's end, 28 frames;
   KH1-in-ReCom `recom-m7-magic.md`, decoded from `xm_fire01.mag`, Checked (files) there).
2. **The script:** KH2 runs a spell from its BDX script (trigger 20 at the cast, 21 at the
   release; the sister's `magic-model.md`). KH1's spell is its `.bd` script in KH1's own script
   machine, a different bytecode (`fire01.bd`: KH1's section 0; Checked (files)). Native: a KH2
   BDX script of ours, written by the build step, doing what KH1's does with KH2's own traps
   (Fire: from the Keyblade's middle, 40 a frame, 30 frames, ≤ 3° a frame toward the target,
   re-picks within 500, radius 50, one hit; KH1-in-ReCom's decoding). Needs: a BDX writer (the
   sister's `kh2-bdx` reads and runs BDX; the traps are documented in its `kh2-magic`).
3. **The effects:** KH1's `xm_fire01.mag` section 1 (its DPX set) as the PAX's DPX, with PAX
   elements for KH1's ids (Fire: 0 on the casting bone, 1 the ball, 2 / 3 the hit sparks, 4 at a
   point).
4. **The numbers:** KH1's damage (Fire 20 / 28 / 36 + max MP, KH1's curve) on the spell's ATKP rows
   (the owner chose KH2's formula with KH1's powers for the swings: the same for spells unless he
   says otherwise), KH1's MP at the release (t60's cost already KH1's).
5. Then Blizzard, Thunder, Cure; then Gravity, Stop, Aero as new KH2 spells (new MAGC and `cmd`
   rows, menu entries, `magic/` files of ours); Magnet and Reflect stay KH2's.

KH1's spell files are in the owner's `local/re/kh1mag/` (`xm_*.mag`, all 19), KH2's in
`local/re/` (`FIRE_1.mag` … `REFLECT_3.mag`).

## t69: KH1's cast motions and release frames on KH2's cast rows (`magic = kh1`, step 5.1)

The first part of KH1's spells: how Sora casts. `kh1_assets::CASTS` (Checked (files) for both
games' motions and KH2's triggers; KH1's release frames from KH1-in-ReCom's decoding of the
owner's `xm_*.mag`, Checked (files) there):

| KH2 row (motion) | KH1 motion | KH2 cast event → KH1's | KH1 free |
|---|---|---|---|
| 224 (56, Fire) | 54 (wind-up, 28) then 57 (60): 88 frames | 22 → 28 | 74 |
| 232 (58, Fire air) | 79 (28) then 82 (52): 80 | 28 → 28 | 74 |
| 236 (59, Blizzard) | 55 (86) | 16 → 28 | 68 |
| 244 (61, Blizzard air) | 80 (78) | 36 → 28 | 68 |
| 248 (62, Thunder; Cure's 65 is its duplicate) | 56 (68) | Thunder 18 → 30, Cure 18 → 36 | none (68) |
| 256 (64, Thunder air; Cure's 67) | 81 (68) | 30 → 30, Cure → 36 | none (68) |

- KH2's spell script gets its release from the cast event (frame trigger 17, its parameter the
  MAGC motion; the sister's `magic-model.md`), so KH2's own spells now leave at KH1's frame.
- Sora's other triggers are moved piecewise (`swing_triggers::cast`): KH2's cast frame onto KH1's
  release, KH2's free point (its open Reaction Command range) onto KH1's "may be left", the end
  onto KH1's end.
- The Keyblade keeps KH2's own motion on these rows (W_EX010's size; Assumption: it rides the
  palm the same way through a cast).
- Built on both models; built again when the Config's Magic row is switched (t65's rebuild).
- Not yet: KH1's air cast hop (30) and air speed × 0.5 (`0x1402A3600`); KH1's Fire fast path
  (Fire again during the release, or from a swing's combo window); the spells themselves (KH2's
  Fire / Blizzard / Thunder / Cure scripts and effects still run): the next steps of the plan above.

## t70: KH1's Fire inside KH2's own Fire (`magic = kh1`, step 5.2)

The build step writes `magic/FIRE_1.mag` … `FIRE_3.mag` into the mod folder from the player's own
files: **KH2's own spell file with KH1's Fire added to it** (`kh1_assets::spells::fire_mag`). KH2
loads, runs, draws and collides it as its own spell; there is no spell player of ours.

- **One file for both.** The Drive Forms cast the same files (sub 10 and up; the sister's
  `drive-model.md`, and `00battle.bin`'s ATKP 502 rows, Checked (files)), so KH2's script,
  elements and effects stay, unchanged and where they were; KH1's are added after them.
- **The script** (`kh2_bdx::asm`, a BDX writer of ours): KH2's own `fire` code word for word, then
  ours. The trigger table points at two small stubs: trigger 20 (the cast) takes base Sora's casts
  (sub < 10) to KH1's Fire and the Drive Forms' to KH2's own entry, and keeps the choice in a work
  word past both scripts' work; trigger 21 (the release) follows it. KH1's Fire as before: from the
  Keyblade, 40 a frame, 30 frames, at most 3° a frame toward the target, radius 50, fades its last
  20, ends on its first hit (KH1-in-ReCom's decoding of `xm_fire01.mag`, Checked (files) there).
  None of our float pushes sits where KH2 rewrites one for 60 fps (`kh2_bdx::vm::pc_patch`,
  checked by a test). The test machine runs base Sora's cast in the merged file and gets the same
  flight as our script alone.
- **The effects:** one DPX holding KH2's 8 effects and KH1's 10, and both DPDs. KH2 looks an effect
  up by its number (`0x1402BC9B0`, the sister's `kh2-effects.md` §3, Checked there), so KH1's are
  renumbered +200 (200..204, 300..304; KH2's are 0..7). KH2's DPX load relocates every DPD of
  the list (`0x1402BDDE0` → `0x1402C21E0` for each), Checked (decomp). A particle data's shapes,
  models and vertex sets go into KH2's number-indexed tables as it becomes active
  (`0x1402C1610`, Checked (decomp)), the same as for two separate effect files.
- **The elements:** KH2's 13, then ours: 100 the cast on the Keyblade (weapon tip `0x4003`), 101
  the ball, 103 the hit, 105 / 106 wall and clash, the hit ones in hit-effect group 3.
- **The hit effect** (`kh2-hook`, `combo_tables::patch_spells`): base Sora's Fire rows (ATKP 502,
  subs 0..5, those with a hit effect) take `effect_on_hit` 3 at KH2's table load, so KH2's hit
  picks KH1's spark; put back when Magic is switched to KH2 in the Config menu.
- **HD textures:** KH2's own for its DPD, then KH1's, as `remastered/magic/FIRE_n.mag/-i.dds`
  (Panacea takes a mod folder's list when it has as many as the file's textures). Without KH1's
  HD textures none are written. Assumption: Panacea then gives the SD textures for the whole file.
- **Sounds:** KH2 stand-ins on the elements (116 cast, 118 hit) until KH1's sounds are a KH2 sound
  bank. KH1's own particle sounds (`+0xFC`) are cleared.
- Magic switched to KH2: the files are removed; KH2 loads its own at its next load of a spell
  file. Assumption: KH2 loads a spell's file again on a room or world change (an already loaded
  one plays until then).
- Not yet: KH1's re-pick of a new target within 500 when the first is gone; KH1's damage (KH2's
  Fire rows' powers still); KH1's sounds; Blizzard, Thunder, Cure next.

## t71: Fire's attack radius call fixed

`trap_attack_set_radius` takes 3 arguments (attack, radius, height; 0 = the radius): KH2's trap
table 2 entry 11 (`0x140756B60 + 11 × 16`: `0x1404322F0`, flags 3) and its body pass the three to
`0x140437C00`, Checked (exe + decomp); KH2's own Blizzard pushes three. t70's Fire pushed four, so
the game would have read the radius as the attack. The test machine now fails any script that
leaves a value on a stack between frames.
