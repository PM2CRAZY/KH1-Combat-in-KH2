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

## Next versions (one system each)

1. Sonic Blade: KH1's offer on Triangle in KH2's reaction-command slot, its rows, motions, damage.
2. Strike Raid, Ars Arcanum, Ragnarok (each its own version).
3. The combo modifiers as `ptya` rows testing our numbers (KH1's rows and tests, Phase 2's way).
4. Counterattack.
