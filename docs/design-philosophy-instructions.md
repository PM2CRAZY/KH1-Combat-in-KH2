# Core instructions

The project's rules. Detail lives in `engineering-rules.md` and `systems-to-port.md`. If this file
and the project's instructions ever differ, this file wins on the testing branch.

```
PROJECT GOAL
Bring Kingdom Hearts 1's combat into Kingdom Hearts 2 (PC, Steam build first), all Rust, as a DLL
(crates/kh2-hook). Every setting has a Config-menu toggle between KH2 and KH1. With every setting on
KH1, the game must feel 1:1 with KH1. With every setting on KH2, it must play exactly like KH2.
Sister mod: KH2-Combat-in-KH1 (the opposite direction), read-only from here.

NATIVE ALWAYS
KH2's own systems own Sora. Native means giving KH2's engine more abilities so it can do everything
KH1 can, so that KH1 runs 1:1 inside KH2. We add to KH2's own systems: new states in its state table,
new rows and requests in its tables, new motion slots, new particle programs, new sound banks, new
camera modes, new functions or wrappers patched into KH2's own functions. KH2 then runs the KH1
feature itself: it steps it, draws it, plays it, collides it and ends it. The model is the sister
project's effects work: KH1's effect engine was taught KH2's programs, with no second effect player.
Here, KH2's engine is taught KH1's.

It is not native when KH1 logic of ours runs beside KH2's and copies its results into KH2's fields,
even if it never takes Sora over: a KH1 controller of ours taking Sora, a separate KH1 combo or
finisher picker asked next to KH2's own, our own hit sweeps, our own spell-script runtime, our own
renderer. That code is "running along" or "deprecated"; every change moves it into KH2's systems.
If something can only run along for now, say so in the thread and ask before shipping it that way.
Never offer hybrid or KH1-controller versions and never ask which version to build; use one only
if the owner or a tester explicitly asks.

BUILD WHAT KH2 LACKS
If KH1 has a feature, state or mechanic KH2 doesn't have, build it into KH2's engine without asking
each time (KH1's MP gained by landing hits, KH1's Gravity, Stop and Aero, for example). Ask only about
open design choices, and before choosing between native designs that change how the feature plays.

TARGET VS METHOD
- Target: with a setting on KH1, every value matches KH1's exe exactly (timings, frame data,
  momentum, speeds, windows, camera). No approximating.
- Method: KH1's numbers run inside KH2's engine. Confirm KH2's tick rate in its decomp before
  converting frame counts. Don't assume the two games differ.
- 1:1 is done when the tester's trace matches KH1 frame for frame, or the KH1 decomp confirms the
  numbers. Say which you checked and what you couldn't. If a real native attempt still can't match
  KH1, stop and ask: what differs, why, and what the native options are.

KH2 STAYS WHOLE
Nothing KH2 does may break or change because of our work, including what KH1 never had: Drive
Forms, Reaction Commands, Limits, KH2's summons. Replaced systems still give KH2's events and scripts
everything they read (tutorial counters, Drive and form tallies, reaction-command prompts, story
flags). KH2's and KH1's systems share the same native code wherever they can. A choice that strongly
affects how the game feels becomes a Config toggle, not a silent change for everyone.

LITTLE OF OURS PER FRAME
As little of our code as possible runs every frame. Prefer data KH2's own code reads (rows,
requests, motion slots, attack data, effect programs, sound banks) over per-frame hooks that steer
it. Measure before proposing a performance fix; don't trade native integration for a small speed
win without asking. Assets: a build step makes them once, from the player's own extracted KH1 and
KH2 files, into the mod folder for KH2 to load as its own. Nothing of the games' goes in the repo.
Each step is planned with the tester first.

SCOPE IS OPEN-ENDED
docs/systems-to-port.md lists examples, not the full list. Look in the KH1 decomp, and in KH2, for
related mechanics nobody mentioned; say what it is, where, and whether it matters for 1:1 feel, then
ask before adding it.

HARD RULES (details in docs/engineering-rules.md)
- Testing branch only. Never commit to, push to, merge into or open a PR against main or any other
  branch. Never rebase or force-push testing. No code or data pulled from main.
- Steam addresses only; never reuse Epic addresses (the OG KH2 Rust rewrite is Epic).
- No decompiled code or game files in the repo or docs: plain-language notes only.
- The sister repo is read-only; nothing copied from it unless the owner says so.

WORKFLOW
- The owner works directly in chat (owner, 2026-11-12: "dont worry about the tester request
  system at all. Listen to me and do what i say"). The owner's chat messages are the requests and
  the go-ahead; the tester threads are not used unless the owner brings them back.
- One system per version. Any change under crates/ bumps the version (VERSION in lib.rs plus a
  matching AGENTS.md stage; python3 tools/testers/version.py). The testing letter is t (t1 onward).
- After a change, state the version name and the 1:1 check made; don't mention compiling.
```
