# Roadmap: KH1's combat inside KH2

Each phase is a run of versions, one system each. Every system gets a Config toggle (KH2 / KH1);
all on KH2 is vanilla KH2, all on KH1 is 1:1 KH1. Native throughout: KH1's values and states go into
KH2's own tables, states, motion slots, effect programs and sound banks (`kh2-tables.md`).

## Phase 0: foundation
1. Loader and log (t1, done; Checked (log): Panacea loads it in the Steam build).
2. Byte-checked hooks, the Steam build check, settings with a KH2/KH1 toggle per system (t2, done;
   Checked (log)).
3. Sora's object found (`docs/kh2-sora.md`, Checked (decomp)); KH2's tick rate and the remaining
   fields read off the trace (t3, waiting on a recording).
4. The trace (t3, done): F10 records Sora each frame, read only, from Panacea's `OnFrame`.
5. The toggles as rows in KH2's own Config menu. In research: the in-game menus are KH2's `menu::`
   code (`Mode`, `Window`, `Widget`); `YS::CONFIG` (`0x14039EE10`) is the developer config, not the
   player's. Until the rows exist, `kh2-kh1.ini` holds the toggles.
6. The asset build step: KH1's motions, effects and sounds made once from the player's own files
   into the mod folder, for KH2 to load as its own. Plan in `docs/asset-build.md`; open choices
   asked of the owner first.

## Phase 1: movement
Run and walk speed, turn rate, jump arc and height (KH1's double-sine rise), High Jump, air control,
fall cap, landing. Through KH2's `prty` / `sstm` / `fmab` values and its own movement states.

## Phase 2: ground and air combos
KH1's combo lengths (Combo Plus, Air Combo Plus), its attack order and finishers, as rows in KH2's
combo table `ptya`; KH1's swing motions in KH2's motion slots; KH1's per-hit data (damage,
knockback, re-hit) as `atkp` rows; KH1's combo windows on KH2's motion triggers.

## Phase 3: defence and reactions
KH1's Guard, Dodge Roll, hit reactions, knockback, stun and recovery, i-frames, as KH2 states.

## Phase 4: MP and magic
KH1's MP bar and MP gained by landing hits; Fire, Blizzard, Thunder, Cure, Gravity, Stop, Aero
through KH2's magic system (`magc`), with Gravity, Stop and Aero built into it.

## Phase 5: abilities and specials
KH1's Sonic Blade, Ars Arcanum, Strike Raid, Ragnarok and the other MP specials as KH2 commands;
KH1's ability list as a swap of KH2's.

## Phase 6: camera and lock-on
KH1's field camera and lock-on as KH2 camera modes, checked against traces.

## Phase 7: polish and Epic
Effects and sounds per move, damage formula and stats (to be decided), then every Steam hook's Epic
address.

Throughout: KH2-only systems (Drive Forms, Reaction Commands, Limits, summons) keep working at either
setting, and everything KH2's events read stays fed.
