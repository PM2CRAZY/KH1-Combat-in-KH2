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

## t59: KH1's air knock-back path

The owner (answer): "KH1's exact path". KH2's blow step `0x140422780 (action, out, object, dt)`
(Checked (decomp)): per step the sideways speed × sstm `+0x18` (0.8) floored at `+0x10C`, a rising
speed × `+0x20` (0.8), then + 20/49 capped at `+0x138`; out = velocity (`+0x20..+0x28`, y down) ×
dt. KH1's (`0x14029CC40` → `0x1402B7F20(actor, 20.0, dir)`, Checked (decomp)): KH1's arc of
height 20 under its gravity 0.48469 (rising `h · sin(π/2 · sin(π/2 · t / T))`, then `g/2 · (t −
T)²`, the drop capped at 20 a frame) and 16 sideways along the hit, × 0.95 a 30 Hz frame with the
stick let go.

`win/reactions.rs`: when its big air hit starts KH2's launch, the hook gives the launch KH1's
sideways 16 along KH2's direction and marks it; KH2's blow step then moves him on KH1's arc
(`kh1::arc_height`) and KH1's drag for that launch only, writing the velocity back so KH2's own
reads (falling flag, landing) follow. Any other blow (KH2's own, a script's) is untouched: it is
told apart by its velocity. Assumptions: `dt` is in 60 Hz frames (logged once); KH1's stick is let
go during the knock-back (the drag applies). Not checked in game.

## t58: KH1's air knock-back motions

KH1 (Checked (decomp), `0x14029CC40`): a big hit in the air plays request `0x50` from the front
(`dot(facing, push) < 0`) or `0x51` (slots 69 / 70, 38 frames), queues the fall (request 6) with
an 8-frame blend, and launches him (`0x1402B7F20(actor, 20.0, dir)`); he lands on his feet (the
landing, slot 11). KH2's launch (t54's big air hits) plays 16 from the front by the same test, 18
from behind, until he lands, then 17 / 19 (`0x1403D2FE0` case 4, Checked (decomp)).

- `kh1_assets::AIR_KNOCKBACK`: rows 66 / 74 (16 / 18) ← KH1 69 / 70, each with KH1's 8-frame blend
  into the fall and then the fall looping (`POSTLUDE`); rows 70 / 78 (17 / 19) ← KH1's landing,
  blended into KH1's battle idle (`TAILS`).
- Sizes: `convert::REACH_TOL` 0.12 → 0.16 units (each joint within 0.16 of the pose written),
  `P_EX100.mset` 2,527,536 (KH2's 2,611,760); the Keyblade's rows 70 / 78 share row 22's (the same
  KH1 landing, `SHARED_KEYBLADE`, duplicate-marked): `W_EX010.mset` 266,960 (KH2's 270,224).
- The launch's path: t59. Not checked in game.

## t57: KH1's guard arc

KH1's guard (attack row `0x3D`, data `0x4DA8`) meets attacks within an arc cosine of -0.174:
100 degrees either side of his facing (the sister project's `kh1-roll-guard-native.md` §3.3,
Checked (files) there). KH2's guard window (ATKP 161, Sora's guard, t41) meets an attack when
`dot(its push, his facing) < -cos(reflect range)`, the range in degrees at ATKP `+0x22`, turned
by `+0x23` × 2 degrees (`0x1403D1F40`, Checked (decomp)); 161's range is 120, its turn 0 (Checked
(files)). `win/combo_tables.rs` (`defence = kh1`): after KH2's table loader, 161's range is 100.
The repel's guard (ATKP 162, range 180, motion 42) is KH2's own and unchanged. Assumption: only
Sora's guard uses ATKP 161. Not checked in game.

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

## t54: KH1's reaction rule, push, knock-down and mercy on Sora

The owner's three choices above, in KH2's own reaction routine (`win/reactions.rs`, base Sora,
`defence = kh1`):
- **The hit record** (Checked (decomp): `0x1403CE740` fills it, pool `0x142A24530`, 0x40 bytes):
  `+0x18` bit 0 no effect, `+0x20` ATKP handle, `+0x26` reaction, `+0x28` the damage
  (`0x1403C12F0`'s). The target's status at `+0x5C0`: `[0]` HP, `[1]` max HP.
- **The rule** (KH1 `0x1402BF290`, Checked (decomp)): `(int)(damage / max HP × 100)` under 20 → a
  flinch; else a knock-back. Only KH2's reaction groups 1..4 (push, flinch, flinch and push,
  launch) are replaced; groups 5 and up (Magnet and KH2's specials) stay KH2's.
- **Flinch:** KH2's group 3 (ground flinch and push, or the air flinch), the push KH1's: 750 ×
  share, at most 150, through a hook on KH2's hit-back `0x1403CEAB0` for that hit only.
- **Knock-down (ground):** KH2's group 2 (flinch, no push), then KH1's knock-down motion: KH1's
  slots 65 / 66 (72 frames, in place) built into KH2 Sora's free motion ids 20 / 21 (rows 82 / 86,
  each its own entry made on the front flinch row 42; Checked (files): ids 20..39 are DUMMs in
  both sets), requested by KH2's own `0x1403C86A0` / `0x1403C8310` (as `0x1403D43B0` does for
  10 / 12) on the same front / behind test. The Keyblade's rows 82 / 86 share row 42's motion
  (marked duplicate, `convert::map_bar_shared`), so `W_EX010.mset` stays 268,400 bytes.
  Assumption: KH2 treats motions 20 / 21 as damage motions as it does 10..13 (they're in its
  10..39 range).
- **Knock-back (air):** KH2's launch (group 4) with KH2's numbers and motions, for now.
- **Mercy:** `+0xD70` (KH2's no-damage time, 60 Hz frames) at least 30 past when he may act:
  flinch 46 + 30, air flinch 30 + 30, knock-down 58 + 30. Assumption: KH1's count starts at the
  "may be left" time.
- Not ported: KH1's push curve (KH2's 0.1 / × 0.9 kept); KH1's air knock-back (hop 20, 16
  sideways, slots 69 / 70). Not checked in game.

## Next

1. Done in t51 (above).
2. Hit reactions: KH2's hurt and blow motions (Sora's slots in the 40s–70s, `A320`..`A372`)
   against KH1's (requests and slots to map), knockback, stun and recovery, i-frames after a hit.
