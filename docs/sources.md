# Sources on the owner's PC

Where the references live (found 2026-11-12; the paths in the project instructions were out of date).
Nothing here goes into the repo: these are read in the assistant's workspace only.

| What | Where |
|---|---|
| KH1 / KH2 Steam Ghidra decomps (also BBS, Re:CoM) | `C:\Users\PM\source\repos\kh2-combat\local\decomps\steam\` (`kh1-steam-decomp.zip`, `kh2-steam-decomp.zip`, unpacked as `kh1\`, `kh2\`; `<start>.c` per 0x10000 block, `functions.tsv`, `strings.tsv`; layout in `local\decomps\README.txt`) |
| Extracted game files | `C:\Users\PM\source\repos\kh2-combat\local\re\` |
| Steam install (KH1, KH2 exes) | `D:\Steam\steamapps\common\KINGDOM HEARTS -HD 1.5+2.5 ReMIX-\` |
| Epic install (for the later Epic port) | `D:\epic\KH_1.5_2.5\` |

## The exes

Image base 0x140000000 for both (decomp address = 0x140000000 + RVA). Checked (exe).

| Exe | Size | SHA-256 | PE link stamp |
|---|---|---|---|
| Steam KH2 `KINGDOM HEARTS II FINAL MIX.exe` | 8,659,216 | `9002b2de…5849ed` | `0x669E384A` |
| Epic KH2 `KINGDOM HEARTS II FINAL MIX.exe` | 8,661,776 | `7db77a7a…e8f623` | `0x669E44D8` |
| Steam KH1 `KINGDOM HEARTS FINAL MIX.exe` | 7,490,832 | `d7907462…884bac` | `0x669E37CE` |

The Steam decomps were made from exactly these Steam exes (the SHA-256 in the decomps' README
matches). Checked (exe). The KH2 link stamp tells Steam from Epic, so t1's log line identifies the
build (closes t1's Assumption). Checked (exe).
