# Phase 5: KH1's abilities and specials inside KH2

`abilities = kh1`. Owner's answers (2026-11-14): specials offered KH1's way, on Triangle, in KH2's
reaction-command slot (a real KH2 Reaction Command takes the slot first); KH1's abilities arrive at
KH2's unlock points; and, because of KH2's 80 slots (below), **each KH1 ability stands in the slot
of the KH2 ability it is paired with**, KH2's list coming back at `kh2`.

## KH2's ability list (Steam). Checked (decomp + files) unless tagged

- Sora's save record: `save + 0x24F0` (save `0x1409A98B0`; a character's record is
  `0x1403A14E0(save + 0x24F0, char)` = `(slot − 1) × 0x114`, Sora char 1 → slot 1 → `0x1409ABDA0`).
  Abilities: 80 u16 at `+0x54`, item id, bit 15 = equipped.
- Adding: `0x1403A1330(record, item, equipped)` takes the first free entry and **drops the ability
  when all 80 are full**. Every give goes through it: `0x1403C3F00(item, char, equipped)` (kind
  `0x13` → `0x1403A1330`; levels, boss bonuses `bons`, Drive Form levels `0x1403E8AA0`, shops,
  events) and the event op `0x14041E7E0`.
- How full KH2 gets (the owner's files): Sora's level list 23 abilities (each Dream Weapon), his
  boss bonuses 21, Drive Form levels 15, chests none; plus story gives and the growth entries:
  about 75 of 80 by the end. So KH1's abilities can't take slots of their own (owner's answer (a)).
- The ability block (`0x1404014A0`, from the status build `0x1403C1D90`): every equipped entry's
  item through `0x140401730(block, item)`: a switch **on the item id** for KH2's numeric effects
  (Combo Plus 0xA2 … Damage Control 0x21E), then the bit `item +4` (u16) in the block's 256-bit set
  (`+0` .. `+0x20`; the growth levels follow at `+0x20`). An item whose `+6` byte is not 0 is
  skipped in a Drive Form.
- KH2's ability checks: combo rows `ptya +0x40` test the block bit by **number**
  (`0x140402750`); Limits test `limt +0x32` (an item id → its number; no Sora action ability
  there); scripts test the save list **by item id** (`0x1401B40E0`). AP total: `0x140351560` sums
  `item +7` over equipped entries.
- `item` rows (24 bytes, table `0x142A25370`, walked by `0x1403E39A0`): `+0` id, `+2` kind (`0x13`
  ability), `+3` flags, `+4` number, `+6` 0 support / 1 party / 2 Sora action / 3 growth, `+7` AP,
  `+8` / `+0xA` name / description (`0x8000 | message`). KH2's last ability number is 198.

## t90: KH1's action abilities in their pairs' slots

`ability_rows` + `win/abilities.rs`: after KH2's table loader, a copy of KH2's `item` table whose
paired rows carry KH1's name, description, AP and an ability number of ours (200 + k); KH2's
pointer is that copy at `abilities = kh1`, KH2's own at `kh2` (the Config row switches it at once).
The save is untouched (KH2's ids), so: KH2 gives each KH1 ability exactly where it gives its pair;
KH2's scripts still see the KH2 item; a save never holds anything of ours. KH2's combo rows for the
paired move test KH2's number, which the row no longer sets: KH2's move is off while KH1's stands
in. Names: `msg/us/sys.bar` with ours at `0x5720..0x573B`, written at every launch whatever the
settings (`kh1_assets::build_text`).

| KH2 ability (item, how Sora gets it) | KH1 ability (id, AP) |
|---|---|
| Upper Slash (137, bonus 2) | Sonic Blade (11, 3) |
| Magnet Burst (561, bonus 8) | Strike Raid (13, 3) |
| Vicinity Break (562, bonus 31) | Ars Arcanum (12, 4) |
| Aerial Finish (272, bonus 21) | Ragnarok (14, 4) |
| Slide Dash (264, bonus 43) | Sliding Dash (54, 1) |
| Dodge Slash (263, bonus 16) | Vortex (17, 1) |
| Aerial Sweep (269, bonus 9) | Aerial Sweep (18, 2) |
| Aerial Spiral (270, bonus 62) | Hurricane Blast (55, 2) |
| Flash Step (559, bonus 18) | Blitz (20, 2) |
| Explosion (266, bonus 61) | Ripple Drive (56, 3) |
| Guard Break (265, bonus 60) | Stun Impact (57, 2) |
| Finishing Leap (267, bonus 37) | Gravity Break (58, 2) |
| Retaliating Slash (273, bonus 3) | Zantetsuken (59, 2) |
| Counterguard (268, bonus 5) | Counterattack (19, 2) |

- KH1's ids and AP: btltbl `0x33D8`, row id − 1, u16 cost. Checked (files).
- The pairing is ours (by role; open to change). Every paired item: a Sora action ability, given
  by a boss bonus, no case in `0x140401730`, no Limit gate. Checked (files + decomp).
- Same in both games, no swap: Guard, Dodge Roll, High Jump, Glide, Combo Plus, Air Combo Plus,
  Combo Master, Scan, MP Haste, MP Rage, Second Chance, Leaf Bracer, Jackpot, EXP Zero / No
  Experience. Their rows stay KH2's; where KH1's effect differs it is that system's work (Combo
  Plus: same rule, Checked (decomp) in Phase 2; MP Haste / MP Rage: Phase 4).
- Not in yet: KH1's support abilities KH2 lacks (Treasure Magnet, Critical Plus, Berserk, Lucky
  Strike, Encounter Plus): their pairs are an open question (KH2's support abilities near them
  have item-id effects in `0x140401730` that a swap can't turn off). Trinity Limit: KH2's Trinity
  Limit gates its Limit, so it can't be a pair.
- Assumptions: KH2 reads `msg/us/sys.bar` once, at boot; the ability block keeps the old bits until
  Sora's status is built again (room change, equip screen) after a Config switch; English only.
- Each KH1 ability does nothing yet when equipped: its move arrives in its own version (next:
  Sonic Blade).

## t91: no KH2 combo modifiers or finishers at abilities = kh1

The owner (2026-11-14): "With abilities set to KH1 I don't want to see any KH2 combo modifiers or
finishers at all". KH2's combo abilities left after the pairs (`kh1_assets::abilities::HIDDEN`):
Slapshot (262), Horizontal Slash (271), Aerial Dive (560), Combo Boost (390), Air Combo Boost
(391), Finishing Plus (393), Negative Combo (394), Berserk Charge (395). At `kh1`:
- off the ability screen: KH2's list build `0x140360920` runs with their ids at 0 (put back after);
- no effect: the ability block build `0x1404014A0` runs with their equipped bit clear (put back
  after), so neither their bits (their `ptya` rows) nor their item-id effects in `0x140401730`
  apply (Checked (decomp): Finishing Plus `+0x32`, Negative Combo `+0x30/+0x31` −1, Berserk Charge
  `+0x56`, Combo Boost `+0x4E`, Air Combo Boost `+0x50`);
- no AP: their row's `+7` is 0 in the KH1 table (KH2's totals sum every equipped entry).
The save keeps the entries and their equipped bits; at `kh2` they are back as they were.
Kept: Combo Plus, Air Combo Plus, Combo Master (KH1 has them).

## t92: KH1's modifier motions in their pairs' motion slots

`kh1_assets::MODIFIERS` (`abilities = kh1`, the asset build): each paired KH2 ability's own motion
(its `ptya` row's; only row `id × 4` holds one, Checked (files)) holds KH1's motion, with KH1's hit,
combo and trail windows and the slot's other KH2 triggers moved with the hit (`swing_triggers`, as
the swings). Checked (files, the owner's `xa_ex_0010.mset`):

| KH2 slot (pair) | KH1 motion (request) | frames | hit | combo window |
|---|---|---|---|---|
| 164 Slide Dash | 106 Sliding Dash (208) | 94 | 26–58 | 28–86 |
| 163 Dodge Slash | 109 Vortex (211) | 76 | 18–42 | 38–66 |
| 191 Aerial Sweep | 112 Aerial Sweep (214) | 56 | 6–28 | 20–44 |
| 192 Aerial Spiral | 107 Hurricane Blast (209) | 76 | 4–54 | — |
| 169 Flash Step | 108 Blitz (210) | 80 | 30–38 | — |
| 166 Explosion | 113 Ripple Drive (215) | 100 | 34–46 | — |
| 165 Guard Break | 114 Stun Impact (216) | 94 | 34–50 | — |
| 167 Finishing Leap | 115 Gravity Break (217) | 66 | 32–38 | — |
| 172 Retaliating Slash | 116 Zantetsuken (218) | 100 | 36–50 | — |
| 171 Counterguard | 111 Counterattack (213) | 86 | 8–24 | 18–56 |

Sizes with these (the owner's files, KH1 model): `W_EX010.mset` 266,304 and `P_EX100.mset`
2,582,224, both under KH2's (270,224 / 2,611,760). Nothing picks these motions until t93.

## t93: KH1's modifier rows in KH2's own combo picker

`modifiers` + `win/modifiers.rs` (abilities = kh1). KH2's picker (Checked (decomp)): the pick
`0x140402D20` builds a context (`0x1404028D0`: `+0` the target, `+8` its part, radius `+0xC`,
centre `+0x20`; `+0x30` the aim; `+0x40` Sora; `+0x50` his position), tests the kind-2 rows for
every enemy part and the kind-1 rows for the target (`0x1404029F0` → `0x140402750` ability / air /
stage, `0x1404027F0` windows: `near ≤ gap < far`, `high ∓ r < dy ≤ low ± r`, `min < dot ≤ max`),
then the kind-3 rows (`0x140402750` alone), and `0x140402C50` takes the first valid row in set
order whose motion isn't playing: KH1's own rule (`0x1402A6EE0` / `0x1402A6C70`: the lowest flagged
row whose request isn't playing).

- **The rows**: Sora's set (37 rows) rewritten in place in KH1's order: Aerial Sweep; Stun Impact,
  Gravity Break, Zantetsuken, Blitz, Ripple Drive (KH1 rows 3..7, before the finisher 8);
  Hurricane Blast (15, before the air finisher 16); Sliding Dash, Vortex (target), Vortex (side
  enemies) (9, 10, before 11..14); then KH2's plain rows in KH2's order, the kind-0 rows
  (Counterguard's id 39 → Counterattack, which KH2 takes after a guard: `0x1403A76B0`), then KH2's
  ability rows, dead at kh1. Each of ours on a KH2 template of its kind, our ability number, the
  pair's motion slot (t92). KH2's bytes back at kh2; taken off around the combos patch (which
  finds rows by id). Test with the player's `00battle.bin`.
- **KH1's tests** (our rows only; KH2's code for every other row):
  - the finisher press draws one r: Stun Impact, Gravity Break, Zantetsuken (equipped, in order)
    each take the next 0.3; none → Blitz, unless the scan found a side enemy; Ripple Drive always;
  - with a target (gap = flat distance less the part's radius, h = its height above Sora): h ≤ 160
    or h ≥ reach + 150 or gap ≥ 300 → (gap ≥ 300 or ≤ 180 → Sliding Dash (equipped, gap ≥ 300, h <
    160) else the scan; between: Vortex); else Aerial Sweep (the jump-up attack);
  - the scan's side enemy: cos 120° < dot ≤ cos 22°, gap < 180 (Vortex's side row; Blitz);
  - Hurricane Blast: a target no more than 100 above.
  Checked (decomp) through KH1-in-ReCom's `kh1_core::combo` (its tests' cases are ours).
- **Assumptions**: KH2's aim (`ctx +0x30`, toward the target, else his facing) is KH1's; reach =
  KH2's jump height of Sora (`0x1403B5A40`: KH1's 190 / 290 at movement = kh1); KH1's "state 2"
  for Hurricane Blast is any air pick; Aerial Sweep's hop and speed are KH2's jump approach's (as
  the plain jump-up today), not KH1's computed ones; Counterattack when KH2's Counterguard would
  be taken.
- Still KH2's on these motions: the ATKP rows of the slots (power, reaction): t94.

## t94: the modifiers' hits and KH1's damage

- **Hit windows** (`swing_triggers::modifier`, the build): in a modifier slot KH2's own attack
  windows on Sora (Explosion's burst, Flash Step's and Guard Break's guard windows, ATKP 124 /
  161 / 162 / 406) and its no-hurt range (27) go: KH1's move has its hit only. Where the Keyblade
  has no hit window in the KH2 slot (Explosion 166, Finishing Leap 167, Counterguard 171: Sora's
  body hits, Checked (files)), KH1's hit goes on Sora with the slot's first own attack (911 / 854 /
  1581) and group. Checked (files): Ripple Drive 34–46, Gravity Break 32–38, Counterattack 8–24 on
  Sora; the rest on the Keyblade at KH1's frames.
- **Damage** (`damage = kh1` and `abilities = kh1`, `kh1dmg::MODIFIER_ATKP`): the slots' attacks
  take KH1's rows (Checked (files), btltbl): Sliding Dash (968) `0x50B8` power 0; Vortex (1007),
  Aerial Sweep (909) `0x4CC8` power 2; Hurricane Blast (1131) `0x5048` 6, crit 4 / +4; Blitz (1896)
  `0x4D38` 6, crit 4 / +4; Ripple Drive (911) `0x4E88` 10 + max MP × 3; Stun Impact (876) `0x4EF8`
  4, element 10; Gravity Break (854) `0x4F68` (Strength + 40)% of HP, crit 200; Zantetsuken (1582)
  `0x4FD8` 250% of HP, no stat, crit 200; Counterattack (1581) `0x4E18` 0, crit 200 / +4. At
  abilities = kh2 these ATKPs are KH2's moves and keep KH2's damage.
- **Not yet KH1's**: the reaction each hit gives (stun, launch, KH1's radii): the slots' KH2 ATKP
  rows' reactions stay (Assumption: close enough until a trace compares). Unknown range 44 in
  Explosion / Finishing Leap / Counterguard kept (Assumption: not a hurt rule).

## Where KH1's specials will run (research for the next version)

- KH2 already runs multi-part specials with Triangle follow-ups: Limit Form's Sonic Rave, Strike
  Raid, Last Arcanum, Infinity (`cmd` 698 / 704 / 701 / 683, execute 707) and the party Limits
  (execute 81). Sora's command run `0x140405440` sends both executes to `0x1403D8B40(limit id =
  *(Sora +0xDD0) + 10, Sora)`: the command's `limt` row (`0x1403E7800`, table `0x142AE5690`, rows
  of 64) makes a limit object (0x128 bytes, `0x1403D7FA0`) that runs the special, takes its MP
  (`0x1403D3CF0`), and a second press sends that object event 0x17 (the follow-up; "Raid" is cmd
  705). Checked (decomp + files).
- Plan (native): each KH1 special is a `limt` row of ours with its own limit script and KH1's
  motions, run by KH2's limit engine for base Sora; KH1's offer (range, MP, target) as a candidate
  in KH2's reaction-command choice, below any real Reaction Command; KH1's follow-ups as the limit
  object's own follow-up commands. To read next: the limit object's step and script loading
  (`0x1403D7FA0`, `0x1403D9130`), the reaction-command choice, and the Drive fallback it already
  offers with no object.

## The specials: where KH2 offers and runs them (research, 2026-11-14)

Checked (decomp) unless tagged:
- **KH2's reaction-command choice** is `0x1404081C0` (from `0x140408520`, each frame): candidates
  go through `0x140408080` → `0x140408BD0(ctx, cmd row, target ref, priority, score)` (priority
  wins, then the smaller score; `ctx +0x58` the priority held, `ctx +0x28` the score): Sora's own
  `+0xDD8` command (priority 0xC9), the command menu's (0xCB), an attack's (0xCC), every object's
  type-5 collision areas (`0x140407DE0`, the area's argument the command id), then, with nothing
  else, the low-HP Drive list (`0x1404077F0(ctx, Sora, 0x1405CCA40, 5)`), the summon list
  (`0x1405CCA50`, 4) and the Limit (`0x1403E7950`), each with a self target ref built by
  `0x1403BEE60(ref, object, 0)`. `0x140402120(ctx + 8)` says whether one was chosen. KH1's offer
  (a special only when no real RC holds the slot: owner's answer) goes in at the end of this
  choice, the way KH2's own Drive / Limit fallbacks do.
- **Running a command**: Sora's `0x140405440(Sora, cmd row)` by execute `+2`: 0x49 / 4 / 5 / 0x48,
  the Limits (0x51) and Limit Form's specials (0x2C3) through `0x1403D8B40` (the `limt` row's limit
  object, `0x1403D7FA0`, MP paid by `0x1403D3CF0`), everything else `0x1403A8B10`. KH2 plays a row
  of Sora's set by id with `0x1403D78D0(Sora, row, …)` (as Counterguard's row 0x27 after a guard,
  `0x1403A76B0`).
- **KH1's specials' motions** are in KH1's specials motion set, swapped in for the special (KH1
  `0x14028D800`; KH1-in-ReCom: `xl_limit.dat`'s clips, wind-up 0xC8, dash 0xC9, finisher 0xCA for
  Sonic Blade, Checked (files) there), not in Sora's `xa_ex_0010.mset`: they need KH2 motion slots
  of their own (free ids, as the knock-downs' 20 / 21).

Plan, one version each: (1) the specials' KH1 motions into free KH2 slots; (2) Sonic Blade: its
`cmd` rows (Sonic, Rave, Blast), KH1's offer in `0x1404081C0`, a run for our execute in
`0x140405440` (KH1's MP on the press, KH1's no-damage / no-reaction flags), its rows 0x1B..0x21 in
Sora's set played by `0x1403D78D0`, the follow-up offered in the dash's combo window; (3) Strike
Raid (the throws, the catch, the clash); (4) Ars Arcanum (seven hits by themselves, the
follow-ups); (5) Ragnarok (the rising hits, the charge, the homing shots from the Keyblade's tip).

## Next versions (one system each)

1. Sonic Blade: KH1's offer on Triangle in KH2's reaction-command slot, its rows, motions, damage.
2. Strike Raid, Ars Arcanum, Ragnarok (each its own version).
3. The combo modifiers as `ptya` rows testing our numbers (KH1's rows and tests, Phase 2's way).
4. Counterattack.
