# KH1 Sora's player model in KH2

Owner (after t25): "Lets see if we can have KH1 sora's player model be placed in KH2", then
"B": KH1 Sora with **KH1's own skeleton** (293 bones, KH1's proportions), so KH1's swings play on
him exactly (no lean fitting), and every animation KH2 plays on Sora is converted onto that
skeleton at build time. A Config row, **Player Model: KH1 / KH2**, switches it. Everything is
built from the player's own KH1 and KH2 files into the mod folder; no game data is in the repo.

## Plan

1. **Read KH1's model** (`kh1_data::mdls::parse_mesh`). Done in t26.
2. **Write a KH2 model** (`kh2_formats::mdlx` writer): KH1's meshes, textures and skeleton as
   an MDLX KH2 loads as `obj/P_EX100.mdlx`. Sora's model also carries data KH2's code reads by bone
   index; it has to point at KH1's matching bones. That includes the collision shapes (attack,
   reaction, the hit check `0x1403B68E0` reads) and the bones the Keyblade, effects, camera and
   lock-on attach to. Reference: OpenKH's `OpenKh.Kh2.Models.ModelSkeletal` / `VIF` writer
   (Apache-2.0). Our code reads the same layout.
3. **Convert KH2's motions onto KH1's skeleton**: every entry of `P_EX100.mset` (idle, run,
   jump, guard, reactions, Reaction Commands, Limits, summons…) through `kh1_data::retarget`
   (the sister project's KH2-to-KH1 transfer). KH1's own motions (98–104 and later others) go in
   unconverted, from `kh1_data::rig`, which matches KH1 exactly (`kh1_truth`). The writer
   (`convert::write_motion`) already writes KH2 motions; it takes KH1's bone count.
4. **The Config row** and switching. Player Model KH2 = KH2's own files, as now.
5. **Cutscenes**: event motions for Sora live in the event files. They need the same
   conversion, or Sora switches to KH2's model in cutscenes. Drive Forms have their own models
   (KH1 has none): they stay KH2's.

## Step 1: KH1's model (t26)

`parse_mesh` reads the `.mdls` model object's mesh groups and textures (layout in the code's doc
comment, after OpenKH's `OpenKh.Kh1.Mdls`, our own code). On the player's `xa_ex_0010.mdls`:
9 mesh groups, 5,182 vertices, 2,828 triangles, 7 textures (128×128, 8-bit, 256-colour palettes).
Every vertex hangs from one bone (weight 1.0): no multi-weight packets. Drawn offline on KH1's
own idle and swing 98 (`kh1_data::rig`, which matches the kh1-posedump recording), the mesh is
whole. Checked (files).

## Steps 2–4: the model, the motions, the Config row (t27)

**The model writer** (`kh2_formats::mdlx_write`). Layout decoded from the player's
`obj/P_EX100.mdlx` (Checked (files)):
- Model entry (BAR type 4): 0x90 zero bytes, then (offsets from there) the header `3, 0, 0,
  shadow (0), bone count u16, texture count u16, bone table, skeleton data, group count`, 0x20
  bytes a group (`attributes, texture, triangle count, 0, DMA tags, matrix list, tag count, 0`),
  the skeleton data (0x120: bounding box, IK bias, 56 body-part bone numbers at 0x30, a distance
  at 0x110), 0x40 bytes a bone, then per group its packets' VIF code, its DMA tags and its matrix
  list (each packet's bones, packets parted by -1).
- A packet's VIF code and DMA tags are written exactly as KH2's (header, u/v, masked vertex index
  and flag bytes, STCOL 1.0 + positions, per-matrix counts; a code reference, a 4-quadword matrix
  reference per bone, MSCNT), at most 0xE0 quadwords of VU memory. Every triangle is three index
  entries (flags 0x10, 0x10, 0x20). Checked (files): KH2 Sora's own mesh written back reads to
  the same 3 groups, every triangle, bone, position and u/v (`sora_model_writes_back`).
- Textures (BAR type 7, `tim_`): header `0, palette words, image count, texture count, map,
  uploads, registers, images, palettes`, one 0x90-byte GS upload for all palettes (a 64×32 CT32
  page at block 0x3A00, 4 blocks a palette) and one per image (8-bit, sent as CT32 at half
  width, from block 0x3A20 on), 0xA0 bytes of GS registers a texture, `_KN5`. Same addresses and
  register values as KH2 Sora's. Checked (files) against our reader (`textures_read_back`).
  Not written: KH2's texture animation footer (TEXA, Sora's blinking eyes and mouth). KH1 Sora
  has none of KH2's. Assumption: KH2 draws a model without one (many of its models have none).

**KH1 Sora as `P_EX100.mdlx`** (`kh1_assets::player_model::build`): KH1's 293 bones, 9 meshes
(2,828 triangles; KH1's winding is KH2's: 90 % / 88 % of same-bone triangles face out with the
same corner order, Checked (files)), 7 textures with KH1's palettes; KH2's collision list and
skeleton data kept with their bone numbers moved to KH1's bone at the same place
(`player_model::BONE_MAP`: head 140→149, neck 137→144, chest 67→77, hips 2→2, collarbones
75/167→195/90, upper arms 78/170→197/92, elbows 79/171→199/94, hands 85/177→202/97, thighs
11/40→42/16, knees 12/41→44/18, feet 15/44→46/20, toes 18/47→50/24). Each pair is within 12
units after scaling for height (KH1 Sora 151 tall, KH2 Sora 160), Checked (files). HD textures:
KH1's own (`-66380.dds` … `-7e380.dds`, one per PS2 texture at its offset) written as
`remastered/obj/P_EX100.mdlx/-0.dds` … `-6.dds`. Assumption: OpenKH serves a mod's
`remastered/<file>/` assets in place of the game's.

**The motions** (`player_model::build_mset`): all 79 motions of KH2's `P_EX100.mset` (839 slots,
the rest share their data) posed by KH2's own evaluator, moved onto KH1's bones by
`kh1_data::retarget` (a fresh one a motion, so cloth starts from rest), written for the 293-bone
skeleton. Slots that shared data still share it (`convert::map_bar`). Read back, every joint
lands within 0.62 units of the pose written (`kh2_motions_on_kh1_skeleton`). KH1's swings
(98–104, slots 210–216) are posed by `kh1_data::rig`, KH1's own bone pass, with no lean fitting.
A motion for another skeleton than its template's (`write_motion`): bone count and total from
the new skeleton, the joint table in bone order, KH2's IK helpers kept (renumbered past the new
bones, hung from the root: the travel curves write helper 52), Euler forms seeded from KH1's bind.

**The Config row**: `model = kh1 / kh2` in `kh2-kh1.ini` (a new `SYSTEMS` row). With `kh1`,
launch builds `obj/P_EX100.mdlx`, the converted `obj/P_EX100.mset`, `obj/W_EX010.mset` and the HD
textures; with `kh2` they are removed. Works with either combo style.

**Not converted yet** (they still carry KH2's 228-bone skeleton and will look wrong on KH1
Sora): Sora's motions kept outside `P_EX100.mset` — the Reaction Commands' captured motions in
enemy and object motion sets (nested type 20 slots, tag `0010`), Limits and summons, world
variants. And cutscenes (step 5). Open: where KH2 attaches the Keyblade (not the objentry
`WeaponJoint` byte: Sora's is 1, Donald's 2, every Keyblade's 25, so it is an id, not a bone).

## t34: the motion set fits KH2's room

The Keyblade-switch crash (t27–t33) is the motion set's size: with KH2's model (`model = kh2`,
motion set about KH2's own 2.6 MB) it does not crash; with KH1's (3.4 MB in t33) it does, and at
9.8 MB (t27) the camp menu crashed. Checked (owner, t33 runs). Assumption: KH2 keeps Sora's files
in an area heap sized by the area's script (`0x1403A2170` sets `0x142A0F7B0` = 0x280000 by
default, Checked (decomp)), and a bigger file overruns into other objects' data (the overwritten
collision list `0x140407DE0` read).

So the converted set is now smaller than KH2's own: 2.39 MB (KH2's 2.61 MB; the KH1 model is
260 KB against KH2's 437 KB):
- Two IK helpers a motion (a root and the travel helper) instead of the template's 11–53 (KH2's
  IK targets, unused without IK). This also fixes the travel curves pointing at helper 52 in
  motions whose template has only 11 helpers.
- Rotation tolerance per bone: what moves the farthest joint it carries by 0.12 units (0.002 to
  0.03 rad).
- Hermite keys: after fitting, keys their neighbours can do without are removed; tangents rounded
  to 1/1024 a frame (the fit checks with the rounded ones), so most share one stored value.
- Read back, every joint within 0.67 units of the pose written (`kh2_motions_on_kh1_skeleton`).

## t43: the travel stays in the bones; one Euler form for every motion

- The owner (t41, t42): "Sora still rubber bands during his ground combo". Checked (files): KH2's
  own motions keep the travel in the bones (A300's root ends at z 145, its travel at 150; 154 at
  141 / 144), so KH2 draws the model at the object less the travel. Our KH1 motions had the travel
  taken out of the bones: Sora was drawn behind himself and caught up when a swing ended. Checked
  (trace, t42): the object itself moved by KH1's travel (74 / 133 / 186 on 151 / 153 / 155). Now
  the bones keep it (KH1's rig poses as they are; `kh2_poses` puts it back after the transfer; the
  converted KH2 motions take it off for the transfer only, which scales the hips' place).
- The owner: "Sora still flips 180 during his KH1 guard with the KH1 player model". Assumption:
  KH2 blends motions by mixing their Euler numbers, and a turn's form was chosen per motion
  (nearest the bind): KH1's root turns about 110° from its bind (`kh1-posedump.md`), so two
  motions could start in the two forms of the same turn and the blend went round by 180°. Every
  motion for KH1's skeleton now starts from the forms of one reference pose (KH1's battle idle,
  motion 1, frame 0; `convert::set_own_seed`).

## t45: the root block's turn

KH2's guard (173) turns Sora with its root block (OpenKH `RootPosition`): rotation y −3.165 and a
curve on it that goes 0 → −2π → 0 over the guard (Checked (files)), a whole turn. Our motions
took the template's root block with its curves cut but its rotation left: the KH1 guard was drawn
turned by −3.165, back to front (the owner, t42 / t44: "guarding still flips sora"). Now:
- every motion we write has no root rotation (and no root translation);
- a KH2 motion moved onto KH1's skeleton keeps the template's root turn curves, sampled a frame at
  a time (`convert::write_motion_keep_turn`), so KH2's own guard still turns.
The t43 Euler-seed change stays (it doesn't hurt), but it was not this.

## t48: a Keyblade bone on KH1's grip (gone in t49)

The owner (t44): "the keyblade is not in his hands for any keyblade he has equipped". KH2 hangs the
Keyblade from the hand bone in the skeleton data (Assumption: in t27 it followed our map 85 → KH1
202, while KH2's own motions, moved onto KH1, turned KH1's hand like KH2's). KH1's own motions turn
the hand KH1's way and hold the Keyblade at KH1's grip bone (280). Now the model has a 294th bone
(`player_model::WEAPON_BONE`), on the grip, turned by KH1's grip bind → KH2's hand bind (a fixed
turn, `weapon_local`), placed in every motion from the grip; the hand map 85 → 293 (skeleton data
and collision). So KH2's Keyblade motions, made for its hand, sit in KH1's grip. Assumption: the
two binds hold their Keyblades alike.

## t49: the Keyblade on KH1's palm, held KH1's way

The owner (t48 video): "The keyblade is still not in his hands". t48's bone didn't help: KH2 hangs
a weapon on the bones of its object's `sklt` row, not on the skeleton data's hand (and 85 was KH2's
left hand). Checked (files): P_EX100 (objentry 84) has weapon joint 1, `sklt` row 1 = bones 178 /
86, KH2 Sora's right and left palms; on KH1's skeleton those are other bones. Checked (decomp): the
row is read from the object table (`0x1403B5C10`, objentry byte `+7`) and looked up in `sklt`
(`0x1403EAAE0`, the table at `0x142AE5798`, rows of 8 bytes: id, right, left).

Now, with `model = kh1`:
- t48's bone is gone (the model is KH1's 293 bones again; the hand map 85 → 202 is back).
- `kh2-hook` `weapon_hold`: after KH2's table loader (`0x1403F4C30`, which runs at boot right
  after the object table is loaded, `0x14039CAF0`, Checked (decomp)) a `sklt` row of ours (the next
  id, 28 on the owner's files) holds KH1's palms, 101 / 206 (`player_model::PALMS`; Checked (files):
  KH1's grip bone 280, the Keyblade's root, is 101's child; 206 is the left palm). P_EX100's weapon
  joint is set to it. The forms and the world outfits are other objects and keep row 1.
- The Keyblade's motions (`W_EX010.mset`) in every row where Sora plays a KH1 motion are KH1's own
  Keyblade bones in that palm (`swing_triggers::keyblade_motion`): KH2's Keyblade skeleton is KH1's
  grip chain bone for bone (bone k = KH1's 280 + k, the same parents, Checked (files)), so its bone
  k plays `palm⁻¹ × KH1 bone (280 + k)` at each of KH1's frames. Checked (files, test
  `keyblade_on_kh1_hold`): the blade (bone 2, KH1's 282) within 0.11 degrees of KH1's in every KH1
  slot. KH1 holds the blade still in the palm (bones 0..2 have no keys in any slot); what moves is
  the keychain, kept within 0.7 degrees a link so the set stays smaller than KH2's (266,720 of
  270,224 bytes). Channels at the Keyblade's bind are left out, as KH2's own motions do, so each
  Keyblade keeps its own keychain lengths.
- KH2's motions on KH1's model (everything not KH1's) keep KH2's Keyblade motions, now on KH1's palm
  (KH2 held them on its own palm; the two palms' binds face within 3 degrees, the sister project's
  `kh1-weapon-second.md` §11).

Before (KH2's Keyblade motions against KH1's hold, test `keyblade_hold_vs_kh1`): 8 to 19 degrees
off on average, up to 105 in KH2's third swing. Not checked in game.
