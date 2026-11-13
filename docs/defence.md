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

## Next

1. Dodge Roll: KH2's roll (motion 209) is not in `P_EX100.mset` (a DUMM there, Checked (files)):
   find the set it is loaded from; KH1's roll is motion 117 (38 frames, no events), its
   invulnerability actor flag `0x40` from op `0x2B` / `0x2A` (`0x14028E6E0`, source not found).
2. Hit reactions: KH2's hurt and blow motions (Sora's slots in the 40s–70s, `A320`..`A372`)
   against KH1's (requests and slots to map), knockback, stun and recovery, i-frames after a hit.
