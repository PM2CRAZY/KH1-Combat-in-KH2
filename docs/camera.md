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
`+0x94`) zeroed (t63: these are the previous goals, not velocities; see below), so KH2's lag `0x140165700` leaves them where KH1 put them; KH2's yaw `+0x5C` /
`+0x60` follow. Entry: KH2's cut flag `+0x41` (a new room, an event's end) re-places it behind Sora
(KH1's way); coming back after other modes (more than 0.1 s without a follow step), KH1's entry at
the drawn view's yaw. Walls: KH2's segment test `0x140171210`, the eye backed off by the sweep's
radius (Assumption: a ray stands for KH1's 40-unit sphere). Inputs: see the file's notes.

Not yet at t62 (done at t63, below): KH1's lock camera; KH1's FOV. First person stays KH2's.
Assumptions: KH2's right stick's y sign and its Normal as KH1's; KH2's world and KH1's share units
and axes (both y down, yaw `atan2(x, z)`). Not checked in game.

## t63: KH1's lock camera on KH2's mode 2, KH1's FOV, the camera in the trace

The owner (2026-11-13): "trace done. finish the camera."

KH2 (Checked (decomp), Steam):
- The update `0x140166100` first copies each goal to its "previous goal": look `+0x84` → `+0x94`,
  eye `+0x64` → `+0x74`, FOV `+0xA4` → `+0xB4`; then runs the mode; then `+0xA4` = the FOV goal
  `+0x38` (`+0xA8` the roll); then the lag `0x140165700`. The lag's spring `0x140165BD0 (out, drawn,
  goal, previous goal, k, 2.5, 0.5, 1)` leaves the drawn value alone when it equals the goal. KH2's
  snap `0x140164D70` sets previous goal = goal = drawn. t62 zeroed `+0x74` / `+0x94` (harmless
  for the spring, since drawn = goal, but mode 0's wall pull reads `+0x74`); t63 writes them as
  KH2's snap does.
- FOV: `+0x28` is the drawn FOV in radians, given to `0x14019DDE0 (projection, fov, 0)`: focal
  `256 / tan(fov / 2)` across KH2's 512-wide view; the projection `0x14019CAD0` / `0x14019C820`
  divides it into a half-height of 208 and a half-width of 256 plus the wide-screen extra
  (`0x140142EC0`). The base FOV `+0xE0` comes from the table `0x140718C00` (rows of 6 floats;
  row 0: 1.5 rad, then 420 / 380 / 250 / 500, 0.008; Checked (exe)). KH2's follow `0x140165110` and
  first person (mode 1 in `0x140166100`) set `+0x38` from `+0xE0` each frame.
- The camera object is `0x140718C60` (`0x140164F00` runs its update and copies its eye `+0x18` and
  look `+0x08` out).
- Lock-on: `0x140176910 (target)` (KH2's lock-on start) sets `+0x43` = 1 and mode 2, and keeps the
  target record at `0x140ABE540` (enemy handle, target part, special target handle);
  `0x140176A00` (unlock) clears `+0x43` and leaves mode 2. Mode 2 `0x140176C70` reads the target
  point through `0x1403BDBE0`; `0x1403BE000 (record, out)` fills the whole target block: the part's
  radius at `out + 4` (the part's `+0x10`, `0x1403C9C00`) and the point at `out + 0x18`. Mode 2's
  lag step adds the eye pull `0x140176E40`.

KH1 (Checked (decomp), Steam): `0x14028CAF0 (53)` keeps the FOV and the focal `256 / tan(26.5°)`
= 513.46; the projection `0x14018D410` divides it into a half-height of 229 and a half-width of
407.11 (16:9). So KH1's view is 2 atan(229 / 513.46) = 48.1° high.

`win/camera.rs` (t63):
- KH2's mode 2 `0x140176C70` (prologue 48 89 5C 24 08, checked) on Sora runs KH1's lock camera
  (`Follow::lock_step`, `0x1401E2650`) with KH2's target point and the part's radius as KH1's lock
  radius (Assumption: the part's radius stands for KH1's per-enemy lock radius). The first frame in
  mode 2 is KH1's lock-on start (`0x1401E41A0`, a 60-frame blend); a new target record while locked
  is KH1's re-lock (60 frames); the first frame back in mode 0 is KH1's unlock (`0x1401E4110`,
  45 frames). Without a target point, KH2's own mode 2 runs that frame.
- KH1's FOV: KH2's `tan(fov / 2) = (229 / 208) × tan(26.5°)`, 1.00403 rad (`kh1cam::kh2_fov`,
  tested), on `+0x38`, `+0xA4`, `+0xB4` and `+0x28`. KH1's vertical view and, both being 16:9,
  its horizontal one. Assumption: the projection's scales `+0x4C` / `+0x50` are 1 in the field.
- The walls' flat probe (`near_wall`, KH1's `0x140196300`): eight flat rays (Assumption, as in
  KH1-in-ReCom).
- KH2's own eye pull after the mode (mode 0's `0x140164940` with KH2's camera collision
  `0x140167A50`, mode 2's `0x140176E40`) still runs: the maps' own camera walls. Where it moves the
  eye, the view is not KH1's frame for frame (Assumption: rare; the trace shows it).
- Which lock-ons use the lock camera is still KH2's (its own lock-on start sets mode 2).

Trace: new columns `cam_mode, eye_x/y/z, look_x/y/z, fov` from `0x140718C60`.

Not checked in game. First person (mode 1) stays KH2's.
