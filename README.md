# KH1-Combat-in-KH2

Kingdom Hearts 1's combat inside Kingdom Hearts II Final Mix (PC, KH 1.5+2.5 ReMIX, Steam first),
run by KH2's own engine. Every setting has a Config toggle between KH2 and KH1: all on KH1 should
feel 1:1 with KH1, all on KH2 should play exactly like KH2. Sister mod: KH2-Combat-in-KH1.

This repository contains no game files, no extracted data and no decompiled code.

## Install (OpenKH, testing branch)

In OpenKH's Mods Manager pick Kingdom Hearts II Final Mix, then Mods > Install a New Mod and enter
`PM2CRAZY/KH1-Combat-in-KH2` on the `testing` branch; tick it and Build. You need Panacea.
`kh2-kh1.dll` and `mod.yml` at the top are the built mod, updated by each build. The current build
only loads and writes `kh2-kh1.log` in the game folder.

## Layout

```
crates/kh2-hook/   the DLL Panacea loads into KH2
docs/              rules, plans and plain-language research notes
testers/           one folder per tester thread (messages, traces, logs)
tools/testers/     scripts for the threads and versions
```

## For testers

Write requests in `docs/tester-requests.md` (how is at its top). Rules the assistant follows are in
`docs/design-philosophy-instructions.md`.

## License

MIT OR Apache-2.0.
