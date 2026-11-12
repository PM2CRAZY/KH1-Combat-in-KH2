# Sora's object in KH2 (Steam)

Addresses are Steam KH2 (`docs/sources.md`). Tags: Checked (decomp / exe / trace) or Assumption.

## Where Sora is

- KH2's object classes have RTTI. Sora's class is `YS::SORA` (through `YS::PLAYER`, `YS::PARTY`,
  `YS::BTLOBJ`, `YS::STDOBJ`, `YS::OBJ`); its class table pointer is `0x140750300`. Checked (exe).
  Other player classes: `PLAYER` `0x14074A518`, `TRPLAYER` `0x140750148`, `NMPLAYER` `0x140750228`,
  `LMPLAYER` `0x1407503E0`, `MICKEY` `0x1407523B8`, `PARTY` `0x14074C468`. Checked (exe).
- `0x140405030` builds Sora (base build `0x1403A7A40`) and stores him at `0x142AE9A28`;
  `0x140405180` returns it; `0x140404BB0` clears it. Checked (decomp).

## Fields

| Offset | What | Tag |
|---|---|---|
| `+0x128` | walk speed (from the `prty` row, word 0) | Checked (decomp): `0x14040A480` |
| `+0x12C` | run speed (`prty` word 1) | Checked (decomp) |
| `+0x130` | jump height (`prty` word 2) | Checked (decomp) |
| `+0x134` | turn rate (`prty` word 3) | Checked (decomp) |
| `+0x13C`, `+0x140` | `prty` words 8 and 10 | Checked (decomp) |
| `+0x148` / `+0x150` | the `prty` row / the row from `0x1403E95C0` | Checked (decomp) |
| `+0x670` | world position (y down); `+0x70` while attached to another object (`+0x6A0`): `0x1403B5B40` | Checked (decomp) |
| `+0xF0` / `+0xF4` / `+0xF8` / `+0xFC` | the arc: air time, time to the apex, start height, apex offset | Checked (decomp): `0x1403CB360`, `0x1403CBC30` |
| `+0x138` | fall cap per frame (from `sstm`) | Checked (decomp): the arc steps |
| `+0x180` | the motion playing (the combo picker compares rows with it) | Assumption |
| `+0xDE0` | a state number (`0x140404A40` tests it against 0xC) | Assumption |

The defaults before a `prty` row is read (`0x14040A440`): walk 2.0, run 8.0, jump 160.0, turn
π/16, and `sstm`+0x14, 0.95. Checked (decomp).

## The combo picker

`0x140402C50` walks the combo table (`ptya`, rows of 0x44 bytes) in order, takes the first valid
row (a bit set in its own valid mask, at `+0x60`) whose motion isn't the one playing, else the
first valid row's `sub` row; `0x140402D20` runs it, with a fallback. Called from the attack press
`0x1403A76B0`. Checked (decomp), matching the OG rewrite's notes (Epic `0x140401F30`, `0x140402000`,
`0x1403A6A60`, mapped to Steam by matching bytes).

## Per frame

Panacea calls our `OnFrame` export once a game frame (Assumption for KH2, as for KH1; the t3
trace's frame count against its clock shows it). The trace runs there with no hook in KH2.

## Tick rate

KH2's frame step `0x140717480` is the 60 Hz frames a game frame takes (`docs/movement.md`),
Checked (decomp). The t3 trace's clock column shows the frame rate it ran at.
