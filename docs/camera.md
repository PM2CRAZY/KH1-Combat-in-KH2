# KH1's camera inside KH2 (plan and research)

The owner (2026-11-13): "bring in KH1's Camera, and have the setting to switch between KH1 and
KH2's camera be in the config", right after Phase 4's first MP step. The Config page's Camera row
(`camera = kh2 / kh1`, t40) is the switch.

## KH2's field camera (Checked (decomp), Steam)

- The camera's update `0x140166100 (camera, ..)` reads the right stick and then runs the camera's
  **mode** (`camera + 0x48`, int) through a switch: 0 the follow camera `0x140165110`, 1
  `0x140165E30` (first person), 2 the lock-on camera `0x140176C70`, 3..10 `0x140176380`,
  `0x1401762E0`, `0x140175A40`, `0x140177600`, `0x1401779C0`, `0x140177C90`, `0x140178810`,
  `0x140177B40` (event, area and other cameras, not yet named). Then `0x140165700` (the lag
  toward the mode's goal) runs for every mode.
- The follow camera `0x140165110`: the subject (`+0x50`, Sora); the yaw `+0x5C` and its goal
  `+0x60`, the distance `+0x58` (`[0x16]`), the stick turn (3° a frame at full tilt × the step)
  and zoom (`0x1401654D0`); the look point = his position − 170 in y (`+0x84`); the placement
  `0x140164520 (camera, look point, cut)` (KH2's height curve over the distance, the sister
  project's `camera-model.md`); `0x140164940` the eye against the walls.
- The camera the game draws: eye `+0x08`, target `+0x18`, two more floats `+0x28` / `+0x2C`
  (filled from an override by `0x1401780C0` / `0x140177FE0..0x140178010`).

## KH1's field camera (the sister project's `kh1-camera.md`, Checked (decomp) there, Epic)

Mode 1 `0x1401DF7C0`: pitch (default 10°), yaw, distance (default 400); the target = Sora's
matrix × (0, −100, −50), springing 20 % of the horizontal gap a step, 50 % down / 10 % up;
eye = target + d (cos p sin y, −sin p, cos p cos y); the stick (its §5.3); the auto helpers
(swing behind while moving, wall avoid, pitch assist); the eye walked toward its place in 20-unit
steps with a 40-radius sphere sweep; R3 recenter (60 steps); the distance springing back 10 % a
step after a pull-in. The lock-on camera: KH1's mode for a locked target (to read).

### KH1's follow camera on Steam (Checked (decomp), the local KH1 Steam decomp; the sister
project's `kh1-kh2-camera.md` mode table, Epic / Steam)

The mode table `0x140506CE0` (16 bytes a mode: update, exit). Mode 1, follow, `0x1401E1920`;
mode 3, lock-on, `0x1401E2650`; the camera task `0x1401E5D90`. Mode 1's globals on Steam: flags
`0x142537E44` (bit 1 entering, bit 0 from mode 0, bit 2 restored), subject `0x142537E48`, eye
`0x142537E60`, target `0x142537E70`, pitch `0x142537E80`, yaw `0x142537E84`, distance
`0x142537E8C`, defaults pitch `0x142538060` / distance `0x14253806C`, target offset
`0x142538070`, the recenter `0x142537EE0..0x142537EF8`. Its pieces, in order:
1. entry: pitch, yaw (from the drawn eye / target, `0x14028C720`), distance; FOV 53°
   (`0x14028CAF0(0x42540000)`); the placement `0x1401E4820 (yaw, eye, target)`;
2. the target spring (no blend running): goal = subject matrix (`+0xD0`) × offset, or the
   head-height point if a segment (`0x140193180`) is blocked; × 0.2 sideways, × 0.5 down / × 0.1 up,
   halved at the high frame rate (step `0x14233FBDC` < 2);
3. the stick `0x1401E7320`; the recenter (R3) `0x142537EEC` timer, its two kinds;
4. auto (Auto camera, state ≠ 0x1B): swing behind `0x1401E6FF0`, wall avoid `0x1401E6C10`, pitch
   assist `0x1401E6670`; eye = target + offset(yaw, pitch, distance) (rotate (0, 0, d) by X(pitch)
   then Y(yaw)); re-placed when the distance grew past default + 300; else walked in ≤ 20 steps
   with the 40-radius sweep `0x140196560` (a jump ≥ 1000 teleports); `0x1401E7780` (five segments
   from his head) re-places when all are blocked; manual: the ray `0x140193790` to 60 past the eye;
5. an automatic recenter `0x1401E62A0` when he stands close (< 40) at pitch < 60° (state-based
   speed tests 1 / 7 / 0.1); R3 (`0x14298CA98 & 0x200`) recenter; Select (`& 0x8000`) first person
   (mode 2, blend 30) unless `0x142867364 & 0x2000`;
6. exit: distance + (default − distance) × 0.1, yaw from eye / target, the state camera switch
   `0x1401E5480`.

## Plan (one system per version)

1. **KH1's follow camera as KH2's mode 0 with `camera = kh1`:** KH2's own follow function runs
   KH1's mode-1 numbers and rules (target, spring, yaw / pitch / distance, stick, R3 recenter,
   the auto helpers), with KH2's own collision for the eye (its wall pull-in `0x140164940`),
   writing KH2's goal and drawn camera fields; KH2's lag `0x140165700` left out for it (KH1's
   spring is its own). To read first: KH1's Steam addresses for mode 1 and its stick numbers
   (§5.3), and `0x140165700` / `0x140164520` in KH2.
2. **KH1's lock-on camera** as KH2's mode 2 the same way.
3. A trace to check it frame for frame against KH1's.

## t62: KH1's follow camera on KH2's mode 0

The owner (2026-11-13): "continue with the camera", and "feel free to look at any of the code we did
for putting KH1 into ReCoM". KH1-in-ReCom (testing) already has KH1's follow and lock camera
written out engine-agnostic, every number from the KH1 Steam decomp (`crates/kh1-recom/src/
camera.rs` `pub mod kh1`, `crates/kh1-core/src/math.rs`). They are copied here as `kh1cam.rs`
(the Re:CoM stick and inversion helpers left out).

`win/camera.rs` (`camera = kh1`): KH2's follow `0x140165110` (prologue checked), on Sora, runs
KH1's follow (`kh1cam::kh1::Follow::step` / `view`) and writes its eye and look as KH2's goals
(`+0x64` / `+0x84`) and drawn view (`+0x18` / `+0x08`), with the spring's velocities (`+0x74` /
`+0x94`) zeroed, so KH2's lag `0x140165700` leaves them where KH1 put them; KH2's yaw `+0x5C` /
`+0x60` follow. Entry: KH2's cut flag `+0x41` (a new room, an event's end) re-places it behind Sora
(KH1's way); coming back after other modes (more than 0.1 s without a follow step), KH1's entry at
the drawn view's yaw. Walls: KH2's segment test `0x140171210`, the eye backed off by the sweep's
radius (Assumption: a ray stands for KH1's 40-unit sphere). Inputs: see the file's notes.

Not yet: KH1's lock camera (KH2's mode 2 stays KH2's); KH1's FOV (53°; KH2's kept); first person.
Assumptions: KH2's right stick's y sign and its Normal as KH1's; KH2's world and KH1's share units
and axes (both y down, yaw `atan2(x, z)`). Not checked in game.
