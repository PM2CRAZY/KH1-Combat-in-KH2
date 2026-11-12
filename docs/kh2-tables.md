# KH2's data tables (Steam)

KH2 reads Sora's combat and movement numbers from tables in its own system files. These are where
KH1's values go in natively: KH2's own code keeps reading its own tables; we give it KH1's rows
when a setting is on KH1. Addresses are Steam (`KINGDOM HEARTS II FINAL MIX.exe`, SHA-256
`9002b2de…`, image base 0x140000000). Checked (decomp) unless tagged.

## The loader

`0x1403F4C30` loads `03system.bin` and `00battle.bin` (each through `0x1403E6010`), finds each
table by its 4-character tag (`0x1403A6480 (file, type, tag)`) and hands its address to one setter
per table. Most setters only store the pointer in a global.

| File | Tag | Setter | Pointer kept at | What it is (Assumption until read) |
|---|---|---|---|---|
| 03system.bin | `pref` | `0x1403E9360` | see below | preferences: player, forms, party, magic, system |
| 03system.bin | `rcct` | `0x1403E7030` | | reaction commands |
| 03system.bin | `cmd` | `0x1403B6940` | | the command list |
| 03system.bin | `item` | `0x1403E39E0` | | items, equipment |
| 03system.bin | `ftst` | `0x1403EA8B0` | | |
| 03system.bin | `sklt` | `0x1403EAB20` | | skeleton attach points |
| 00battle.bin | `atkp` | `0x1403EB010` | `0x142AE57B0` | attack data (damage, knockback, hit effects per hit) |
| 00battle.bin | `ptya` | `0x1403EB780` | `0x142AE57B8` | Sora's combo table (which attack follows which) |
| 00battle.bin | `enmp` | `0x1403EBF90` | | enemy stats |
| 00battle.bin | `lvup` | `0x1403ED340` | | level-up growth |
| 00battle.bin | `plrp` | `0x1403E46A0` | `0x142A253D0` | player stats |
| 00battle.bin | `limt` | `0x1403E7B20` | | Limits |
| 00battle.bin | `sumn` | `0x1403E6D00` | | summons |
| 00battle.bin | `magc` | `0x1403C5F10` | | magic |
| 00battle.bin | `fmlv` | `0x1403E8A90` | | Drive Form levels |

Also loaded there: `went`, `wmst`, `arif`, `trsr`, `memt`, `ipic`, `evtp` (03system) and `przt`,
`vtbl`, `bons`, `btlv`, `lvpm`, `patn`, `vbrt`, `stop` (00battle).

`pref`'s parts (`0x1403E9360`): `plyr` → `0x142AE5770`, `fmab` (growth abilities) →
`0x142AE5778`, `prty` (walk/run speed, jump height per character) → `0x142AE5780`, `magi` →
`0x142AE5788`, `sstm` (system constants: fall cap, lock-on, ceilings) → `0x142AE5760`.

## Frame timing

Motion time in KH2 is in 60ths of a second (several motion readers divide by 60, around
`0x1401D0000`). Checked (decomp). Whether Sora's physics steps every frame or on a 30 Hz tick
(the OG rewrite's notes, from the Epic exe, say a 30 Hz tick, `dt = 2`) is not yet checked on
Steam: Assumption, the first thing to confirm before converting KH1 frame counts.
