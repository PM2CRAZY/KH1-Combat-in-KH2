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
