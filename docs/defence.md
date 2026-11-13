# Phase 3: KH1's defence and reactions inside KH2 (plan and progress)

`defence = kh1`. Native as the swings (`docs/combos.md`): KH1's motions in KH2's own slots, KH1's
windows on KH2's own triggers, KH1's numbers in KH2's own tables. Research basis: the sister
project's `docs/kh1-roll-guard-native.md` (KH1 Steam addresses, re-checked here where used).

## t41: KH1's guard in KH2's guard slot

- KH1's guard is attack row `0x3D` (request `0xD4`, motion 110): an attack object of priority 100
  live in the motion's hit window (sister notes §3.3). KH1 motion 110's events (Checked (files)):
  54 frames, hit (guard) window 10–36, combo window from 22.
- KH2's guard is combo row 11, motion 173 (`A322`), its Keyblade guard window ATKP 161 at 6–37
  (sister notes, Checked (files) there).
- The build writes KH1's 110 into slot 173 (`kh1_assets::GUARD`, on KH2's or KH1's skeleton by
  `model`), Sora's triggers moved with the guard window, Sora's combo window (range 11) from 22,
  and the Keyblade's guard window (range 10, ATKP 161 kept) at 10–36 in `W_EX010.mset`.
- Not yet: KH1's rule for when Square guards (standing or on a light tilt, KH1 `0x1402A7B30`;
  KH2 guards only at rest), the guard's arc (KH1 ±100°, its data `+0x64`).

## t51: KH1's Dodge Roll in KH2's roll

- **Motion.** KH2's roll is motion 209 (the action handler `0x1403A81F0` requests `0xD1` for growth
  kind 4, Checked (decomp)). Its motion is row 838 (209 × 4 + 2, `A180`, 26 frames at 30 fps, 435
  units forward; rows 836 / 837 / 839 are DUMMs; Checked (files)). The t41 note above ("a DUMM")
  read row 836. KH1's roll is motion 117: 38 frames, 350 units forward by frame 30, "may be left"
  (event kind 0) at 30 (Checked (files)). The build writes 117 into row 838 (`kh1_assets::ROLL`,
  `defence = kh1`), on either skeleton, with its travel; the Keyblade's row holds KH1's grip with
  KH1's model (t49).
- **Triggers.** KH2's own move with the roll (`swing_triggers::sora`), anchored by KH2's
  blend-to-idle (range 2, from 36) onto KH1's "may be left" 30 (`free_anchor`): Quick Run check
  10 → 8, fall check 34 → 28, Reaction Commands from 7, footsteps and the voice moved alike.
  Checked (files, test `roll_triggers`).
- **No-damage time.** KH2 gives the roll its level's `dodge_roll_star_time` (fmab `+0x40`: 10 / 16 /
  24 / 32) as `+0xD70`, counted down in 60 Hz frames (`0x1403D4CC0`, `0x1403D68B0`; Checked
  (decomp)). KH1's roll is 12 frames of no damage at every level (`0x1402C9E60` →
  `0x140297F30(actor, 12.0)`, the sister project's notes, Checked (decomp) there). `win/roll.rs`:
  the load at `0x1403A848D` (`movss xmm1, [rax+0x40]`, bytes checked) calls a stub that gives base
  Sora 12; Quick Run's store comes after it and keeps KH2's; Drive Forms keep KH2's. KH1's 12 means
  "hits land without damage or reaction", KH2's "hits don't land": the same for Sora.
- Square's guard-or-roll rule: t52. Not checked in game.

## t52: KH1's Square rule: guard standing or walking, roll running

- KH1 (Checked (decomp), the Square handler `0x1402A7B30`): with Guard equipped, Square guards when
  Dodge Roll isn't equipped or his move speed (`move +0xC`) is under his run speed (stats `+8`);
  else it rolls. The ground step `0x1402A5F10` (state 0's, `0x1402B1B80(0, ...)`) sets the move
  speed to his walk speed (stats `+4`, 2) up to a 0.8 tilt and his run speed (8) past it. So KH1
  guards standing or walking.
- KH2 (Checked (decomp + exe), `0x1403A81F0`): guards only at a move speed `+0x1C` of 0 (the test
  at `0x1403A83C8`), so a walking Square rolled. Its move speed is picked the same way (walk
  `+0x128` under 0.8, run `+0x12C` from it, `0x1403A8FA0`). Without Dodge Roll it guards at any
  tilt, as KH1.
- `win/roll.rs` (`defence = kh1`): the 12-byte test (bytes checked) is a call into a stub: base Sora
  guards when `+0x1C` is under `+0x12C`; anyone else (Drive Forms) at 0, as KH2.
- Not ported: KH1's tilt of exactly 0.8 walks (KH2's runs). Not checked in game.
- Still open: the guard's arc (KH1 ±100° around his facing, attack data `+0x64`); KH2's guard is
  the Keyblade's guard window (ATKP 161), whose reach is its collision shape, not an angle.

## t53: KH1's flinch motions on KH2's flinches

- KH2's reaction routine `0x1403D2FE0` plays motion 10 / 12 for a ground flinch from the front /
  behind, 11 / 13 for the other hit part, 14 / 15 in the air (the sister project's
  `kh1-hit-reactions-native.md` §5 / §9, Checked (decomp) there). Their motions are rows 42, 46,
  50, 54, 58, 62 (id × 4 + 2, `A200`..`A205`; the other variants DUMMs; Checked (files)): 70 frames,
  free state (range 2) from 50, Reaction Commands from 8; the air ones 36 frames.
- KH1's flinch requests `0x48` front, `0x49`, `0x4A` behind, `0x4B`, air `0x4E` / `0x4F` are slots
  61..64, 67, 68 (§9 there; here, Checked (files): 70 frames, "may be left" at 46; the air ones 30
  frames). KH1 calls a hit "from the front" by the same test as KH2 (`dot(facing, push) < 0`).
- `kh1_assets::REACTIONS` (`defence = kh1`): those six KH1 motions in those rows, either skeleton.
  KH2's triggers move with them (free state 50 → KH1's 46, `free_anchor`).
- Stay KH2's: which reaction a hit gives (KH2's ATKP reaction byte), the push, the launches (16 / 18
  and their landings 17 / 19; KH1 knocks Sora down in place instead, slots 65 / 66), KH2's
  no-damage time after a hit. Open questions to the owner (below).
- The Keyblade's rows hold KH1's grip in these too (t49); its keychain tolerance went to 1.7
  degrees a link to keep `W_EX010.mset` within KH2's size (268,400 of 270,224).
- Not checked in game.

## The owner's choices for the rest of the hit reactions (2026-11-13)

Asked after t53; all three answered with KH1's:
1. **What reaction a hit gives:** KH1's rule. The damage's share of his max HP picks it (btltbl's
   reaction block: under 20 % a flinch, 20 % and over a knock-down; KH1 `0x14029BBD0` /
   `0x1402BF290`), with KH1's push (750 × share, at most 150; the curve 0.22314 a step, × 0.8 a
   step). KH2's special reactions (Magnet and the rest, reaction groups 5 and up) stay KH2's.
2. **Big hits:** KH1's knock-down in place on the ground (slots 65 / 66, 72 frames, "may be left"
   58) and KH1's air knock-back in the air (requests `0x50` / `0x51`, slots 69 / 70, 38 frames: hop
   20, 16 sideways along the push).
3. **After a reaction:** KH1's 30 frames without damage, counted from when the reaction ends
   (KH1 `0x140297F30(actor, 30.0)`, its count starting once the action flag clears).

Where it goes in KH2 (research so far, Checked (decomp)): Sora's reaction slot is a thin wrapper
`0x140406890` (vtable entry at `0x1405CC928`) around KH2's generic reaction `0x1403D2FE0 (object,
hit)`: the hit's push direction at `+0x0`, a "no effect" bit at `+0x18`, the reaction byte at
`+0x26` (folded by `0x1403CE980`), the attack's ATKP handle at `+0x20` (hit-back `+0xC` / `+0xE`,
read by `0x1403CEAB0` / `0x1403CEA00`). Still to find: where the hit's damage is applied (for the
share) relative to this call, and a KH2 state to play the in-place knock-down in (KH2's flinch
state, `0x1403D43B0`, with KH1's motion on a free row, is the candidate).

## Next

1. Done in t51 (above).
2. Hit reactions: KH2's hurt and blow motions (Sora's slots in the 40s–70s, `A320`..`A372`)
   against KH1's (requests and slots to map), knockback, stun and recovery, i-frames after a hit.
