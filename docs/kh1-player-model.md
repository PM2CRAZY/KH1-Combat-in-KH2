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
