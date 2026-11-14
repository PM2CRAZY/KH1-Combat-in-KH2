# Damage = KH1 (t89)

The owner (2026-11-14): "Make these all 1:1", then, for spell damage, "(a) KH1's damage, new toggle":
a Config row `damage` (KH2 / KH1). His answers to the three open points: Sora's Strength is his
**KH2 Strength stat** (a); enemies keep **KH2's HP** (b); damage Sora **takes stays KH2's** for now (a).

## KH1's formula (Checked (decomp), `0x1402BFA80`; KH1-in-ReCom's `kh1-core::damage`, tested there)

power + stat (Strength, or max MP × 1 / 2 / 3 / 5 by the data's stat source); a critical roll
(crit × 0.05 × the weapon's crit × 0.05, then + the bonus); element 8 (Gravity): current HP × power % ×
resist, at least the floor (1 under 2); elements 9 / 11 / 12: the power as it is (Cure's heal); else
KH1's curve at (power − Defence), the data's min / max, the record's floor / cap, × the element's
resistance, at least 1. Ported in `kh2-hook::kh1dmg` (tested on KH1-in-ReCom's examples).

## Sora's attacks (Checked (files): the owner's `btltbl.bin`)

| KH2 ATKP (base Sora, sub < 10) | KH1 row | stat | element | power |
|---|---|---|---|---|
| 126, 127, 128, 131, 132 swings | `0x4B78` / `0x4BE8` | Strength | 1 | 0 |
| 129, 130, 133, 134 finishers | `0x4C58` | Strength | 1 | 4 (crit 4, +4) |
| 502 Fire | `0x5F28..` | max MP | 2 | 20 / 28 / 36 |
| 248 Blizzard | `0x6078..` | max MP | 3 | 22 / 27 / 34 |
| 250 Thunder | `0x61C8..` | max MP × 2 | 4 | 16 / 20 / 26 |
| 708 Cure (heal) | `0x6318..` | max MP | 11 | 15 / 27 / 36 |
| 2066 Gravity | `0x6468..` | max MP × 5 | 8 | 40 / 55 / 70 |
| 2069 Aero's wind | `0x5CF8` | max MP × 2 | 0 | 8 |

Min / max 0 on every one. The weapon crit is the Kingdom Key's 20 (KH2's Keyblades have no KH1 row:
Assumption). Every other attack (Drive Forms, Limits, Reaction Commands, KH2's ability moves) keeps
KH2's damage.

## Enemies' KH1 records (`kh1_assets::kh1_records`)

Read at start from the player's KH1 files (KH1-in-ReCom's reader, ported): each ordinary KH1
Heartless's AI script (`.mdls` part 5) holds 0x6C-byte records (r0 HP, r1 / r2 Strength / Defence
indices into btltbl `0xC8A8`, r3 / r4 floor / cap, r5 level, r10.. resistances %). 22 models, every LV1 /
LV2 record (and the Shadow's per-world ones).

A KH2 enemy is told by its `enmp` row (the status's `+0x270` handle): `+0` its id, `+2` its level (0:
the world's battle level, from `lvpm`). KH2's Shadow / Soldier / Large Body (ids 1000 / 1001 / 1002,
models `M_EX010` / `020` / `050`; Assumption on which enemies these are) take their KH1 twin's record
nearest their level, with its resistances. Every other enemy takes the stand-in: the ordinary KH1
record nearest its level (fixed-damage records left out; ties to the lower Defence), with KH1's
ordinary 100 % resistances. **Note:** the owner's option said "a stand-in by world"; KH2's worlds
mostly have no KH1 world, so the stand-in goes by level (KH1's records carry one): to confirm.

KH2's invulnerability (status `+0x1AE` = 0) still makes a hit do nothing. KH2's own extras (difficulty,
ENMP min / max damage) don't apply to KH1's number.

## Not yet

- KH2's bosses take the ordinary stand-in (KH1's bosses' caps of 15–30 a hit would make fights on KH2's
  HP very long): open.
- More twins once KH2's enemy names are read from its files (Air Pirate, Fat Bandit, Darkball,
  Defender, Wyvern, Wight Knight, Neoshadow, Tornado Step, Crescendo, Creeper Plant…).
- Damage Sora takes (the owner: KH2's for now).
