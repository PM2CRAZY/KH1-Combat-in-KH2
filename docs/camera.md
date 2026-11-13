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

## Plan (one system per version)

1. **KH1's follow camera as KH2's mode 0 with `camera = kh1`:** KH2's own follow function runs
   KH1's mode-1 numbers and rules (target, spring, yaw / pitch / distance, stick, R3 recenter,
   the auto helpers), with KH2's own collision for the eye (its wall pull-in `0x140164940`),
   writing KH2's goal and drawn camera fields; KH2's lag `0x140165700` left out for it (KH1's
   spring is its own). To read first: KH1's Steam addresses for mode 1 and its stick numbers
   (§5.3), and `0x140165700` / `0x140164520` in KH2.
2. **KH1's lock-on camera** as KH2's mode 2 the same way.
3. A trace to check it frame for frame against KH1's.
