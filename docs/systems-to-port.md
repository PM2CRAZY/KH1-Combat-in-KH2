# Systems to build inside KH2, 1:1 with KH1

Examples, not the full list: look for related systems, hidden states, flags and special cases in
the KH1 decomp and in KH2, and report what you find before adding it. Everything here runs inside
KH2's own systems with KH1's values and logic. Each item is "not yet looked at" unless noted; the
descriptions of KH1 below are from general knowledge of the game (Assumption) until checked in the
KH1 Steam decomp.

- **Ground and air combos, finishers and combo abilities**: KH1's combo lengths (Combo Plus / Air
  Combo Plus), its finishers and their windows, through KH2's own combo and finisher selection.
- **Movement**: KH1's run speed, turning, jump arc, High Jump, Glide and Superglide, Dodge Roll,
  through KH2's movement states.
- **Guard and hit reactions**: KH1's Guard, its knockback, stun and recovery, through KH2's states.
- **MP**: KH1's MP bar, MP gained by landing hits, and MP costs; KH2's MP Charge stays for KH2.
- **Magic**: KH1's Fire, Blizzard, Thunder, Cure, Gravity, Stop and Aero, through KH2's magic system
  (Gravity, Stop and Aero are new to KH2's engine).
- **Special attacks**: KH1's MP-using abilities (Sonic Blade, Ars Arcanum, Strike Raid, Ragnarok and
  the rest) as KH2 commands.
- **Camera and lock-on**: KH1's field camera and lock-on, as KH2 camera modes, checked against traces.
- **Animations, sounds and effects**: KH1's motions in KH2's motion slots, KH1's effects as KH2
  particle programs, KH1's sounds as KH2 sound banks, built once from the player's own files.
- **Damage and stats**: KH1's damage formula, Keyblade strength and level-up growth (open: may be
  out of scope; ask).

Rules for every item:
- Each setting's KH2 mode must still behave like vanilla KH2.
- A KH1 setting must not be changed by unrelated Config options.
- KH2-only features (Drive Forms, Reaction Commands, Limits, KH2's summons) keep working at either
  setting.
