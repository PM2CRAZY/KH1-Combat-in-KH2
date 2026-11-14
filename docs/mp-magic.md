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

## t72: KH1's Fire in KH1's own order, from the Keyblade's middle, with its flash

Read again from the owner's `xm_fire01..03.mag` with KH1-in-ReCom's script disassembler (its
`tools/kh1-ai-script.py`, the syscall table read from the KH1 Steam exe), Checked (files):

- **Where the ball starts.** KH1 takes a point on the Keyblade's bone (48) at a fraction: Fire 0.5
  (the middle; `sys 19(owner, bone, scale × 0.5)`), Blizzard 1 (the tip). KH2 gives the same points
  as weapon anchors `0x4007` (middle) and `0x4003` (tip) to `trap_obj_apply_bone_matrix` (trap table 1
  entry 83, `0x14042F6F0` → `0x1403B50F0`'s anchor cases), Checked (exe + decomp). t70 started the
  ball at the cast effect's place (the tip).
- **The flash.** At the release KH1 plays its effect 4 at that point, turned to Sora's yaw
  (`sys 74`, `sys 55(…, (0, yaw, 0))`). Now element 104, placed by the script.
- **The ball's frame, in KH1's order** (`0x7A`): life > 0, else gone; turn toward a live target (≤ 3°);
  start the 20-frame fade once life < 20; move 40; place the effect and attack; life − 1; wait a frame;
  a hit ends it. So the first frame shows the ball already 40 along (t70 showed it at the start), the
  fade starts on the 12th frame, and the ball no longer checks its effect being alive (KH1 doesn't).
- **The end:** KH1 `sys 97` kills the effect (`0x1402BB900` → `0x1401EC470`, Checked (decomp)): KH2's
  `trap_effect_kill`, as before.
- The three levels' scripts are the same (Checked (files): Fire 40 / 3 / 20 / 500 in all three).
- Still not: KH1's re-pick of a new target within 500 of the ball when the first is gone (KH2's
  `trap_target_search` searches from the caster by mode, `0x1404303A0` → `0x1403BE2B0`; a native
  match is still to be found), KH1's hit stop (`sys 186`; KH2's attacks have their own).
- The spell code is now shared (`kh1_assets::spells`: one cast, one release point, then each spell's
  shots); Blizzard's is written but not built yet (next version).

## t73: KH1's Blizzard inside KH2's own Blizzard (`magic = kh1`, step 5.3)

`magic/BLIZZARD_1..3.mag` are built as Fire's are (t70): KH2's own file, KH2's script for the Drive
Forms, KH1's Blizzard for base Sora (sub < 10). From the owner's `xm_blizzard01..03.mag`, read with
KH1-in-ReCom's disassembler, Checked (files):

- **Cast:** KH1's effect 0 on the Keyblade, held while the cast motion plays (t69's KH1 motion and
  release at 28).
- **Release** (`0x6DC`): the Keyblade's tip (bone 48 at 1: KH2 anchor `0x4003`), KH1's flash (effect
  4) there turned by π from Sora's facing; then **seven shards** (`0x4A`): six on a cone, KH1's local
  (0.383, 0, 0.924) turned 60° about his facing for each, each starting its sideways part × 50 off the
  axis (19.15); then one straight ahead from the tip. Shards 0 and 3 (level) and the straight one
  play KH1's effect 1, the four others effect 5. Turned to the world by his yaw with
  `trap_vector_roty` (`0x14041CFE0` → `0x1403B7680`, a turn about y, Checked (decomp)).
- **One hit per enemy per cast** (KH1's shared hit bit): one attack (`attack_new`, ATKP 248, KH2's
  shard row by the sub) copied for each shard with `trap_attack_dup` (table 2 entry 31,
  `0x1404324D0` → `0x1404372E0`: a new attack with the source's owner and group, Checked (decomp)),
  so a shard reaching an enemy another shard hit flies on through it.
- **A shard's frame** (`0x238`), KH1's order: while its effect lives and life (26) remains: x and z
  by its direction × speed (KH1's vector unnormalised: 0.383² + 0.924² = 1.0005); then a ray
  straight down by 50 + its own vertical step (`trap_bg_ground_pos(from, out, depth)`, table 1 entry
  141, `0x14042E8B0` → `0x1403F3DD0`, Checked (decomp)): a floor → 50 above it, none → its own
  step; place the effect and attack; wait a frame; a hit ends it; life − 1. Speed 20 / 30 / 40 and
  radius 70 / 90 / 120 by the file's level.
- **The end:** KH1 `sys1 3` sets the effect's first flag byte (`0x140394DE0`): read as "end the loop"
  (KH2's `trap_effect_loop_end`), Assumption.
- **Hit effects:** KH1's body spark (effect 2, set mask `0x81`) and wall spark (3, `0x102`) as group-3
  elements; `kh2-hook` gives ATKP 248's base rows `effect_on_hit` 3 with Fire's (t70).
- Sounds: KH2's Blizzard's stand in (120 cast, 121 each shard, 122 hit, 124 / 125 wall / clash).
- Not yet: KH1's damage (KH2's shard rows' powers), KH1's sounds.

## t74: KH1's Thunder inside KH2's own Thunder (`magic = kh1`, step 5.4)

`magic/THUNDER_1..3.mag` built as Fire's and Blizzard's (KH2's script for the Drive Forms). From the
owner's `xm_thunder01..03.mag` (the same in all three), read with KH1-in-ReCom's disassembler,
Checked (files); KH1-in-ReCom's `recom-m7-magic.md` §7.6 (Checked (files + decomp) there):

- **Cast:** KH1's effect 0 on the Keyblade; **KH1's tint** (`sys 52` → `0x1402BC190`: a scene colour
  1 → 0.5 over 20 frames) as one of KH2's lights: `trap_light_create(1)` (copies the scene's,
  `0x1403F6F80`), `trap_light_set_color(light, 0x40, 0x40, 0x40, 20)` (`0x1404309C0` → `0x1403F76E0`: a
  0 → 1 blend over the frames to that colour; 0x80 = 1, KH2's default), `trap_light_fadeout(light,
  20)` (`0x140430A20` → `0x1403F76A0`). Checked (decomp). Back over 20 frames 40 after the release, or
  at once when the cast is cut before it. Assumption: KH2's light colours the scene as KH1's tint does
  (to see in game).
- **Release** (frame 30): KH1's flash (effect 3) at the Keyblade's tip; the strike thread; the tint's
  thread.
- **The area:** a cylinder radius 600, ±500, round the target's part if it lives, else Sora. The
  marker (effect 1) on the floor under the centre (a ray of 500, `trap_bg_ground_pos`), plus, without
  a target, 2 m along the camera's level forward (`trap_camera_at` − `trap_camera_eye`, KH1's sys1 6 /
  5).
- **The list:** every enemy, nearest first from the target (else Sora), up to 30 (KH1's selection
  sort over `sys 65`). Built with KH2's own target searcher (table 6: `trap_target_searcher_init`
  `0x14053DCE0` → `0x14053FB60`, `trap_target_seracher_search` `0x14053DD20` → `0x14053FB80`: every
  battle target's parts by distance, the nearest not in its history, then added; range `+0x14`
  opened, as KH1 lists every enemy; Checked (decomp)), one entry per enemy. Assumption (small): KH2
  measures to the nearest part, KH1 to the enemy's position.
- **The strikes:** down the list, an enemy still in the cylinder is struck straight on (KH1 `sys 160`,
  no collision) with KH2's `trap_attack_strike(attack, object record, 0, 0)` (`0x1404328E0`, the
  record's object read at `+4`, as Cure's call; Checked (decomp)) on ATKP 250 (KH2's Thunder burst
  row by the sub), KH1's bolt (effect 2) where it struck, then 8 frames; one outside is passed at
  once; the first strike on the release frame. Assumption: "in the cylinder" by the struck part's
  centre (KH1 tests the enemy's hurt bodies against it, `0x1402B6AF0`): an enemy at the very edge may
  differ.
- `kh2-hook`: ATKP 250's base rows take `effect_on_hit` 3 (no group-3 element in Thunder's file, so
  KH2's own spark doesn't play over KH1's bolt).
- Sounds: KH2's Thunder's stand in (126 cast, 127 bolt).
- Not yet: KH1's damage; KH1's hit stop (`sys 186(2)`).

## t75: KH1's Cure inside KH2's own Cure (`magic = kh1`, step 5.5)

`magic/CURE_1..3.mag` built as the others (KH2's script for the Drive Forms). From the owner's
`xm_cure01..03.mag` (the same in all three), Checked (files); KH1-in-ReCom §7.7:

- **Cast:** KH1's charge (effect 0) on the Keyblade; release at 36 (t69).
- **Release** (`0x56`): KH1 strikes its heal straight onto the command's member, else the caster.
  KH2's Magic menu picks no member, so **Sora only** (KH2's own Cure heals every party member near
  him: base Sora no longer does; the Drive Forms still run KH2's). KH2's ATKP 708 (kind 6, heals by
  Magic) struck with `trap_attack_strike` on Sora's own record, as KH2's Cure calls it.
- **The heal effect** (effect 1) held at a fixed height over his feet and following him while it
  plays (KH1: bone 8's height at the release; KH2's body centre `0x4001` stands in for that bone,
  Assumption).
- Sounds: KH2's Cure's stand in (129 cast, 130 heal).
- Not yet: KH1's heal amount (KH1: power 15 / 27 / 36 + max MP; KH2's rows heal by Magic); the
  member choice (KH1 asks which member; KH2's menu has no such step: an open design question).

## t85: KH1's held hits under Stop (owner, 2026-11-14: "Make these all 1:1")

KH1 (Checked (decomp), KH1-in-ReCom `recom-m7-magic.md` "M7 f3"): a hit on a stopped actor is
built as a record (damage worked out then) and queued on it; a record joining a queue that isn't
empty gets a delay of 6..11 frames (`6 − (int)(r × −6)`, KH1's `0x14031BA30` random). While stopped
the actor skips its hit pass; from the frame after Stop each frame pops: a head with a delay loses one
and ends the pop, a head with none is applied and the next looked at. KH1's pool: 128 records for
everyone. Stop records themselves are handled while stopped. A record on a downed actor is dropped
(`0x1402B7B90`).

KH2 (Checked (decomp)): a hit is delivered at once: a record from KH2's pool (`0x1403CE740`, 20 of
0x40 at `0x142A24530`, free list `0x142A24A30`, count `0x142A24A80`; damage at `+0x28`), the struck
object's hit slot (`vtable + 0xC0`: generic `0x1403C2B30` → `0x1403D60C0` → HP `0x1403D3BA0`, then the
reaction slot), then the record back (`0x1403CE950`). Record: `+0x10` the attacker, `+0x14` the struck
object, `+0x1C` the attack, `+0x20` the ATKP row (handles), `+0x27` the push kind. Handles are packed
pointers (`0x1404AD240` / `0x1404AD270`), not checked for life.

Built (`win/held_hits.rs`, `held_hits.rs`; no hook on a function, six call sites):
- KH2's six deliveries (`call [r9+0xC0]` at `0x1401CACAB`, `0x1403D0879`, `0x1403D1930`,
  `0x14043295D`, `0x140538B47`, `0x14053AADF`, Checked (exe)) call ours: a hit on an object whose stop
  block runs KH1's Stop kind (0x20), other than a Stop strike, is copied to KH1's queue (KH1's delay
  rule) and KH2's record goes back to its pool as usual; anything else goes to the slot as before.
- Each frame, only while hits are kept: for an object no longer stopped, KH1's pop; each hit to land
  takes a record from KH2's pool and goes through the object's own hit slot, then back (KH2's own
  pattern). An object gone from KH2's live objects (`0x1403BF4F0`) loses its hits; a downed one (HP 0)
  drops them, as KH1.
- A push away from the attack (kind 5 asks the attack object `+0x1C`, `0x1403CEAB0`): its centre is
  kept at the hit; if the attack is gone from KH2's live attacks (`0x142A24B40`, next `+0xC4`) at the
  landing, a stand-in attack of ours answers with that centre.
- **Assumptions:** our per-frame step runs after KH2's objects tick their stop blocks, so the first
  held hit lands the frame after Stop (KH1's); KH2's hit sparks and attacker-side events stay at the
  contact (KH1's spark timing not read); the random's law only (6..11 evenly), not KH1's sequence.
- **Tests:** KH1's delay law and pool, the landing frames (`held_hits` tests).

## t86: KH1's Gravity on the enemy's height: the squash under the sphere and the hit's flatten

KH1 has no slow under the falling sphere (Checked (files + decomp), KH1-in-ReCom "M7 f2"; the
earlier "slow" was a research-note Assumption): each fall frame the sphere's thread marks every
enemy touching it (`+0x88 |= 4`), and the actor update eases its height (`+0x480`, the world matrix's
y scale, its only reader) toward 0.8 with a 4-frame wobble, back +0.05 a frame without the mark. Its
hit (reaction kind 8, `0x1402B7DE0`) flattens: 68 frames on the ground (75 in the air), squashed over
7 frames to 0.1, held, then the get-up's ease ×0.3 a frame; a flinch knocks it out of it.

KH2 (Checked (decomp)): the object's scale is `+0x30` (a vector, `+0x34` the height), read when the
world matrix is built (`0x1403CCB90` → `0x1401AA7D0`); its stop block is ticked once a frame by the
object update (`0x14040A830`, only caller `0x1403BFDCD`).

Built:
- `stop` table: two more kinds of ours, `0x22` (under the sphere, no flags) and `0x23` (flattened,
  flag 1: KH2 skips the enemy's think and motion while it runs, as KH1's flattened actor does
  nothing).
- The Gravity script, each fall frame (KH1's `L0055c`, not the impact run): every enemy whose part
  centre is within 170 × the sphere's scale of its centre gets kind `0x22` for one frame through
  KH2's `trap_obj_stop` (table 6 entry 6 → `0x1403B6730`).
- The Gravity hit, through the hit deliveries of t85 (so a hit held by Stop flattens when it lands):
  KH2's stop with kind `0x23` for 68 frames, or 75 when KH2 says the enemy is in the air
  (`0x1403BA9A0`). Any other hit with a reaction (`0x1403CE980` group ≠ 0) on a flattened enemy ends
  the kind (knocked out).
- `win/squash.rs` wraps KH2's stop tick (prologue checked): an object with kind `0x22` or `0x23`, or
  one still coming back, takes KH1's height step (`squash.rs`, tested frame by frame against KH1's
  numbers) on `+0x34` = its own height × KH1's `+0x480`, before KH2's tick; nothing while KH1's Stop
  holds it (KH1 skips that update while stopped). Back at full height its own `+0x34` is put back.
- **Assumptions:** KH2's "can't be stopped" (`+0x9B8` bit 2, `0x1403B6730`) stands for KH1's per-actor
  reaction rules and the param-block exemption: those enemies neither squash nor flatten; the enemy's
  part centre for KH1's bodies (as the impact); KH2 enemies have no KH1 get-up motion (0x67), so its
  ease runs until 0.95, then the +0.05 spring; a stop block holds 4 kinds (KH2's own limit).
- Per frame: a 4-slot read per object tick; the height step only for squashed or flattened enemies.
