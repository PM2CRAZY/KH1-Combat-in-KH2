# Movement: KH1 inside KH2 (Phase 1)

Steam addresses (KH1 `KINGDOM HEARTS FINAL MIX.exe`, KH2 `KINGDOM HEARTS II FINAL MIX.exe`).
Tags: Checked (decomp / exe / files / trace) or Assumption. KH1 leads came from the sister
project's `docs/kh1-physics.md` and were re-checked here.

## Time

- **KH2's frame step** `0x140717480`: the 60 Hz frames a game frame takes (1.0 at 60 fps, 2.0 at
  30, capped at 6; set at the frame's end by `0x140154740` / `0x1401548E0`, 2.0 by default
  `0x1401544C0`). Speeds, air time and fall caps are multiplied by it. Checked (decomp).
- **KH1's frame step** `0x14233FBDC` (RVA `0x233FBDC`): the same meaning. Checked (decomp).
- So a KH1 number per 60 Hz frame is the same number in KH2: no conversion. Both games' world
  units are about 1 cm (KH1 Sora is 0.89 × KH2 Sora's size, the sister project's measure).
  Checked (decomp) for the time; the size is a note, not a conversion: KH1's values are used as
  they are (the 1:1 rule).

## The jump and fall arc

| | KH1 | KH2 | Tag |
|---|---|---|---|
| Takeoff | 4 frames (`0x1402AFDA0`) | 4 frames (OG rewrite's trace) | KH1 Checked (decomp); KH2 Assumption (t3 trace) |
| Rise | `y0 + h · sin(π/2 · sin(π/2 · t/T))` | the same (`0x1403CB360`) | Checked (decomp) both |
| Time to apex | `T = sqrt(2H / g)` | the same (`0x1403B73F0`) | Checked (decomp) both |
| Fall | `y0 + h + g/2 (t − T)²` | the same | Checked (decomp) both |
| Gravity `g` | 0.48469388 (RVA `0x3E9D14`) | 20/49 (RVA `0x5CBD84`, shared by every object) | Checked (exe) both |
| Fall cap | 20 × frame step (RVA `0x42F968`) | Sora's `+0x138` × frame step (from `sstm`, 16) | Checked (exe) KH1; KH2 Checked (decomp), its 16 from the OG rewrite (files) |
| Jump height | 190; 290 with KH1's High Jump (stats `+0x10`) | `prty` row 1 → Sora `+0x130` (185); High Jump from `fmab` | Checked (trace) both: KH1 `h = -190, T = 28.0` / `h = -290, T = 34.592` (motion 9, the High Jump jump); KH2 t3 trace |
| Early release | button let go, `t ≥ 6` and more than 5 frames before the apex: a launch at speed 5 (`0x1402A7B30` → `0x1402AF8F0`), a hop of `g/2 · 25` (6.06) | the same window (OG rewrite); its hop goes through Sora's class (`0x1403CB680`) | KH1 Checked (decomp + trace: the cut 6 frames before the apex); KH2's hop gravity: next trace |

t4 puts KH1's gravity and fall cap on Sora's arc (`win/arc.rs`). Checked (trace, t4): the launch's
time to the apex fits 27.6 frames at height 185 (KH1's gravity; KH2's would be 30.1), and past the
apex the drop grows by 1.9385 a 2-frame step = 0.4846 a frame² (KH1's).

KH2's physics on Steam runs on a 30 Hz step at 60 fps: Sora's position changes by the same amount
on two frames in a row (t3 trace, frames 16.7 ms apart). Checked (trace). Time is still counted in
60 Hz frames (the step is 2).

## Read from KH1 traces (the sister mod, physics KH1, 2026-11-12)

KH1 fills Sora's stats at runtime (not in the exe or `btltbl.bin` as plain numbers). From the
owner's three KH1 traces, Checked (trace):
- jump height 190, 290 with High Jump (above);
- air acceleration 0.1: from rest, stick full, +0.8 a frame (0.1 × the stick's speed 8) to the
  run speed 8;
- air drag 0.95 a 30 Hz frame: let go, ×0.97468 a frame (√0.95);
- turn rate on the ground π/16 (0.196 a frame at most), against KH2's 0.262 (π/12).

## Air control

- KH1 (`0x1402AF6A0`): stick held, `v += dir × accel × stick` once a game frame, its length kept
  to the larger of its old length and the run speed; let go, `v ×= drag` (its square root when the
  frame step is below 2). Checked (decomp).
- KH2 (`0x1403B7AD0 (actor, &out, step, keep, drag)`, Checked (decomp)): stick held (`+0x1C`, its
  speed, > 0), the air velocity `+0x20` eases toward the stick's direction `+0x10` × its speed by
  `keep`; let go, × `drag`; `out` = velocity × the frame step. Sora's plain air moves pass 0.9 /
  0.95 (`0x1403CB360`), once a 30 Hz step (step 2, Checked (trace)).
- So the drag is the same in both games (0.95 a 2-frame step). t8 (`win/airsteer.rs`) replaces the
  held branch on base Sora's plain call with KH1's rule, once per 60 Hz frame of the step.
  Assumption: `+0x10` is a unit direction (the next trace's +0.8 a frame from rest shows it).

## Ground

- Walk 2, run 8 in both (KH2 `prty` row 1, Checked (trace) t3; KH1 run 8 Checked (trace): the air
  speed holds at 8 with the stick held). Same numbers: nothing to change.
- Turn rate and jump height: t5 writes KH1's into Sora's `prty` row (`win/prty.rs`).
- Walking off a ledge: KH1 falls from rest and keeps its sideways speed capped at 8
  (`0x14043089C`, KH1 RVA `0x42F94C` = 8.0, Checked (exe)); KH2's `0x1403B4780` starts the fall
  with a downward speed `+0xA3C`. To do.

## Drive Forms keep KH2's movement (the owner, 2026-11-12, answer 2a)

The KH1 numbers go only on base Sora: `win/sora.rs` tells him from a Drive Form by his movement
block's `prty` row pointer `+0x148` (`0x14040A480` stores the row it loads, Checked (decomp)):
row 1 is Sora's; a form loads its own. Assumption: every Drive Form loads a row other than 1
(t6's log line on a Sora built on another row says when one is seen).

## High Jump at KH1 (the owner, 2026-11-12, answer 1a)

KH2 asks the jump height of `0x1403B59B0 (actor, &motion)` and `0x1403B5A40 (actor)`: Sora's
`+0x130`, or with a High Jump level that level's `fmab` row's height (`0x1403E92D0`) and motion 201
(the flip). Checked (decomp). `fmab` is shared with the forms, so it stays: for base Sora any High
Jump level's height becomes KH1's 290 (t7, `win/highjump.rs`).

## The t8 trace (2026-11-12), Checked (trace)

- Jump 190 (57 frames in the air), High Jump 290 (motion 201, 71 frames), the forms' own (Valor:
  run 12, jump 235, High Jump 585): t5, t6, t7 as meant.
- Air steering from a slow start: +1.6 a 2-frame step (0.8 a frame) to 8 (KH1's); Valor eases
  (KH2's). t8 as meant.
- Walking off a ledge: from rest, the drop growing 0.969 a 2-frame step (KH1's gravity), the
  sideways speed 8: already KH1's (KH1 falls from rest, its momentum capped at 8 = the run).
- Early release: the cut is in KH1's window and the fall after it KH1's, but the hop rose 5.10
  (KH2's `g/2 · 5²`, from `0x142A243D8`); KH1's rises 6.06 in 5 frames. t9 makes it KH1's.

## t42: KH1's idle, walk, run, jump, fall and landing motions (`kh1_assets::MOVES`)

The owner (t41 video): "Sora still has his KH2 walking, running, idle, keyblade idle, jumping,
falling etc animation instead of his KH1 animation". With `movement = kh1` the build writes KH1's
motions into KH2's motion set rows (KH2 id × 4 + variant), as the swings (t33):

| KH2 | rows | KH1 |
|---|---|---|
| 0 idle | 0 field, 2 battle | 0 field idle (120 frames), 1 battle idle (170, the Keyblade stance) |
| 1 walk | 4, 6, 7 | 2, 3, 4 |
| 2 run | 8, 10, 11 | 5, 6, 7 |
| 3 jump | 14 | 12 (34: crouch to stretched) |
| 4 fall | 18 | 10 (40, held) |
| 5 land | 22 | 11 (28) |

KH2's ids are the OG rewrite's, its rows Checked (files). Which KH1 slot is which is an
Assumption from their lengths and poses (Checked (files)) and the sister project's request map
(requests 0 / 1 / 2 → slots 0..1 / 2..4 / 5..7, the jump's requests 9..0xC → slots 12, 13, 10,
11); KH1's 13 (the top of the jump) has no KH2 row. The Keyblade keeps KH2's motions for these.

## t49: KH1's field and battle motions on KH2's field and battle rows

The owner (t48 video and trace): "Sora uses his KH1 battle idle and KH1 battle run even when he
doesn't have his keyblade out. Finishing a combo doesnt seamlessly transition into his idle". The
t42 table had the variants backwards (and KH1's slots partly wrong):

- **KH2's variants** (Assumption from OpenKH's `MotionSet.GetMotionSetIndex`; the t48 video agrees,
  row 2 played in the field): KH2 plays the first non-DUMM row of a motion in the order .0 .1 .3 .2
  in battle with the Keyblade out, .1 .0 .2 .3 in battle without, .2 .3 .1 .0 in the field without,
  .3 .2 .0 .1 in the field with it. So .0 is the battle row, .2 the field one, .3 the field with the
  Keyblade out.
- **KH1's slots** (Checked (decomp): the request resolver `0x1402A01E0`): the idle (kind 4) is slot 0,
  slot 1 in battle (actor `+0x374` bit 18); walk and run (kind 3) are slot +1 out of battle, +2 in
  it, +0 only with a handle at `+0x14C` and flag `0x4000` clear (Assumption: not the player in
  normal play). Checked (files): slots 4 and 7 crouch like the battle idle (mean height of the top
  bone 44 / 39 against the field cycles' 58 / 43; test `kh1_cycle_stances`).

| KH2 | rows (variant) | KH1 |
|---|---|---|
| 0 idle | 0 (battle), 2 (field) | 1 battle idle, 0 field idle |
| 1 walk | 4 (battle), 6 (field), 7 (field, Keyblade out) | 4, 3, 3 |
| 2 run | 8 (battle), 10 (field), 11 (field, Keyblade out) | 7, 6, 6 |

The combo's end: KH2 goes from the finisher to the battle idle (row 0), which was KH1's field idle
(standing tall) after KH1's crouched finisher; it is now KH1's battle idle, which KH1's own
finishers end into. Not checked in game. KH1's +0 cycles (2, 5) are no longer used.

## t47: looping motions loop; KH2's motions read from their start

KH2's motion header (`+0x80`: start, end, fps, return, in 30 Hz frames; Checked (files)): a
looping motion's return is before its end (idle 0 of 0..150, walk 20 of 20..56, fall 6 of 0..16),
a one-shot's is its end. Ours had return = end everywhere, so a long idle, walk or run froze on its
last frame (the owner, t44 video). Now a motion loops where its template does, from the same
share of the way through. And a KH2 motion's keys run from its start (the walk from 40 to 112 in
60 Hz key time, the landing from 84, Checked (files)): `convert::kh2_round_trip` reads it from
there (it read from 0 and held the first key for those).
