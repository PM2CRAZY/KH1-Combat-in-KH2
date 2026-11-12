# Engineering rules

Detail behind the HARD RULES in `design-philosophy-instructions.md`.

## Steam vs Epic addresses
The tester plays Steam. The OG KH2 Rust rewrite's addresses are Epic: never reuse them. Find the
Steam address by matching the logic in the KH2 Steam decomp, and check the bytes in the Steam exe.
New addresses are plain constants, used only when the build is Steam. Every hook checks the
function's exact first bytes (at least 5, read from the Steam exe) and refuses to install on a
mismatch. The Epic build comes later: once the Steam work is done, every Steam-only hook gets its
Epic address too.

## No decompiled code in the repo
No decompiled code or game files in the repo or the docs. Decomp findings go in as plain-language
notes (address, what it does, how sure we are), never as pasted decompiler output. Ghidra databases
and output stay outside the repo. The `.gitignore` is a whitelist for this reason.

## Don't break KH2's story and events
Every system we replace must still give KH2's events and scripts what they read: tutorial counters,
Drive and form tallies, reaction-command prompts, story flags. Before replacing a system, check what
in KH2 reads it and say what you found.

## Performance
- Say whether a design adds hooks, per-frame work or duplicate state on top of KH2, and what that
  costs on low-end hardware.
- Avoid per-frame allocations, redundant hooks and heavy tracing in normal play.
- Measure, don't guess: say which section is costly before proposing a fix.
- Don't trade native integration for a small performance win without asking.

## Keep the docs current
When something is verified, record it in the matching doc in this folder: the address, what it
does, and a tag: Checked (exe), Checked (decomp), Checked (files), Checked (trace), or Assumption.

## Shared findings go to kh-blender too
Engine and format facts about KH1 or KH2 found here also go into the kh-blender repo, under
`docs/research/kh1/` or `docs/research/kh2/`, with the same tag and the version they came from (for
example "KH1-Combat-in-KH2 t12"). Project decisions and tester threads stay here. If the same note
exists in both places, keep them in step; the newer one wins.

## Versioning
Any change under `crates/` is a version: `python3 tools/testers/version.py <n> "<title>" "<body>"
"<msg>" [patch]` adds the AGENTS.md stage, bumps VERSION in `crates/kh2-hook/src/lib.rs`, type-checks
for Windows and commits (set `KH_TRAILER` to the commit attribution lines). Before pushing, run
`cargo test --no-fail-fast`. A push under `crates/` or `Cargo.*` starts the Windows build, which
commits `kh2-kh1.dll` to testing; pull with `--rebase` after it lands.
