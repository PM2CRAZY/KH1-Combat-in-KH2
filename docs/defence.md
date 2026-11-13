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
- Not yet: when Square rolls rather than guards (KH1: a light tilt still guards; KH2 rolls on any
  tilt). Not checked in game.

## Next

1. Done in t51 (above).
2. Hit reactions: KH2's hurt and blow motions (Sora's slots in the 40s–70s, `A320`..`A372`)
   against KH1's (requests and slots to map), knockback, stun and recovery, i-frames after a hit.
