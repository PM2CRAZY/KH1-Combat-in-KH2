//! KH1 character models (`.mdls`): just the skeleton, which is all the port needs (KH2's
//! motions are retargeted onto it; KH1 draws its own meshes).
//!
//! Layout, worked out from the user's `xa_ex_0010.mdls` and the executable's loader
//! (`0x1401D5AD0`, which relocates a `MOBJ`; `0x1401DCF90` walks the tagged chunks):
//! - the file: `count u32`, then `count` section offsets; section 0 is the model object.
//! - a section is a run of chunks `{tag u32, size u32, data}`, the next at `+size+8`: `MOBJ`
//!   (model), `MENV`, `TEXA`, `BONE`, `MMTN`, `URSL`.
//! - `MOBJ`: offsets from its start at word 2 (mesh-group table, 16-byte entries, count =
//!   word 3 >> 4), 4, 6, 8 (the model header), 10, 12.
//! - The model header (`MOBJ + word 8`): `bone_count u32`, an offset (from the header) to the
//!   bone table, another offset, `mesh_count u32`, then `mesh_count` 16-byte mesh entries.
//! - A bone is 0x30 bytes: `scale[3] f32, index u32, rotation[3] f32 (radians, KH2's Euler
//!   order), pad u32, translation[3] f32, links u32`, where `links` packs the parent (bits
//!   0-9), the next sibling (bits 10-19) and the first child (bits 20-29), 0x3FF meaning none.
//!   This is KH2's MDLX bone (`kh2_formats::anim::Bone`) packed tighter, so the KH2 tools pose
//!   it unchanged.

use kh2_formats::anim::Bone;
use kh2_formats::{Error, Result};

const MOBJ: u32 = u32::from_le_bytes(*b"MOBJ");
const NONE: u32 = 0x3FF;
const BONE_SIZE: usize = 0x30;

fn u32_at(b: &[u8], at: usize) -> Result<u32> {
    b.get(at..at + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap())).ok_or_else(|| Error::Format(format!("mdls: read past the end at {at:#x}")))
}

fn f32_at(b: &[u8], at: usize) -> Result<f32> {
    Ok(f32::from_bits(u32_at(b, at)?))
}

/// The skeleton of a KH1 `.mdls`, as KH2 bones (parents before children, as stored).
pub fn parse_skeleton(mdls: &[u8]) -> Result<Vec<Bone>> {
    let count = u32_at(mdls, 0)? as usize;
    if count == 0 || count > 64 {
        return Err(Error::Format(format!("mdls: {count} sections")));
    }
    let mobj = u32_at(mdls, 4)? as usize;
    if u32_at(mdls, mobj)? != MOBJ {
        return Err(Error::Format("mdls: section 0 is not a MOBJ".into()));
    }
    let header = mobj + u32_at(mdls, mobj + 8 * 4)? as usize;
    let bone_count = u32_at(mdls, header)? as usize;
    let table = header + u32_at(mdls, header + 4)? as usize;
    parse_bones(mdls, table, bone_count)
}

/// `count` bones of the 0x30-byte table at `table` in `bytes` (a whole `.mdls`, or the table
/// itself read from the game's memory with `table` 0).
pub fn parse_bones(mdls: &[u8], table: usize, bone_count: usize) -> Result<Vec<Bone>> {
    if bone_count == 0 || bone_count > 1024 {
        return Err(Error::Format(format!("mdls: {bone_count} bones")));
    }
    let link = |v: u32, shift: u32| -> i16 {
        let x = (v >> shift) & NONE;
        if x == NONE { -1 } else { x as i16 }
    };
    (0..bone_count)
        .map(|i| {
            let at = table + i * BONE_SIZE;
            let f3 = |o: usize| -> Result<[f32; 3]> { Ok([f32_at(mdls, at + o)?, f32_at(mdls, at + o + 4)?, f32_at(mdls, at + o + 8)?]) };
            let index = u32_at(mdls, at + 0x0C)?;
            if index as usize != i {
                return Err(Error::Format(format!("mdls: bone {i} says it is {index}")));
            }
            let links = u32_at(mdls, at + 0x2C)?;
            Ok(Bone {
                index: i as i16,
                sibling: link(links, 10),
                parent: link(links, 0),
                child: link(links, 20),
                flags: (links >> 30) as i32,
                scale: f3(0)?,
                length: 0.0,
                rotation: f3(0x10)?,
                translation: f3(0x20)?,
            })
        })
        .collect()
}

/// One vertex of a KH1 mesh: in the space of the bone it hangs from (`joint`), with its texture
/// coordinates and normal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub joint: u16,
    pub position: [f32; 3],
    pub weight: f32,
    pub uv: [f32; 2],
    pub normal: [f32; 3],
}

/// A mesh group: the texture it draws with and its triangles (vertex indices, KH1's winding).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    pub texture: u32,
    pub vertices: Vec<Vertex>,
    pub triangles: Vec<[u32; 3]>,
}

/// A KH1 model's texture: 8-bit indices and a 256-colour RGBA palette, as stored.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub palette: Vec<u8>,
}

/// KH1's 8-bit palette index swizzle: indices 8–15 and 16–23 of every 32 trade places (the PS2
/// GS's CSM1 palette layout).
pub fn palette_index(i: u8) -> u8 {
    match i & 31 {
        8..=15 => i + 8,
        16..=23 => i - 8,
        _ => i,
    }
}

/// A KH1 `.mdls`'s meshes and textures. Layout after OpenKH's `OpenKh.Kh1.Mdls` (Apache-2.0;
/// our own reading code), checked against the user's `xa_ex_0010.mdls`:
/// - `MOBJ` words 2–5: texture info offset and size (16 bytes a texture: size, log2 width and
///   height, width u16, height u16), texture data offset; word 8 the model header; word 10 the
///   end of the last mesh's packets. Offsets from the `MOBJ`.
/// - The model header: `bone count, bone table offset, ?, mesh count`, then 16 bytes a mesh:
///   `?, texture, texture, packet offset` (from the header).
/// - Packets: a 16-byte header, then commands until the word `0x8000`, then 0x20 bytes. Command
///   0: `0, joint, slot, 0` (and 0x70 zero bytes): matrix slot `slot` is bone `joint`. Command 1:
///   `1, size, vertex count, winding, …` (32 bytes), then 48-byte vertices: normal xyz, slot u32,
///   position xyz, weight, u, v, ?, pad. Each run is a triangle strip.
/// - Texture pixels follow at the data offset, then a 1024-byte palette per texture.
pub fn parse_mesh(mdls: &[u8]) -> Result<(Vec<Mesh>, Vec<Texture>)> {
    let mobj = u32_at(mdls, 4)? as usize;
    if u32_at(mdls, mobj)? != MOBJ {
        return Err(Error::Format("mdls: section 0 is not a MOBJ".into()));
    }
    let word = |i: usize| -> Result<usize> { Ok(u32_at(mdls, mobj + i * 4)? as usize) };
    let header = mobj + word(8)?;
    let mesh_count = u32_at(mdls, header + 12)? as usize;
    if mesh_count > 256 {
        return Err(Error::Format(format!("mdls: {mesh_count} meshes")));
    }
    let last_end = mobj + word(10)?;
    let mut meshes = Vec::with_capacity(mesh_count);
    for m in 0..mesh_count {
        let e = header + 16 + m * 16;
        let texture = u32_at(mdls, e + 4)?;
        let start = header + u32_at(mdls, e + 12)? as usize;
        let end = if m + 1 < mesh_count { header + u32_at(mdls, e + 16 + 12)? as usize } else { last_end };
        let mut mesh = Mesh { texture, ..Default::default() };
        let mut slots = [0u16; 64];
        let mut p = start;
        while p + 16 <= end {
            p += 16;
            loop {
                let cmd = u32_at(mdls, p)?;
                if cmd == 0x8000 || p >= end {
                    break;
                }
                match cmd {
                    0 => {
                        let joint = u32_at(mdls, p + 4)? as u16;
                        let slot = u32_at(mdls, p + 8)? as usize;
                        if u32_at(mdls, p + 12)? == 0 && slot < slots.len() {
                            slots[slot] = joint;
                        }
                        p += 16 + 0x70;
                    }
                    1 => {
                        let count = u32_at(mdls, p + 8)? as usize;
                        let winding = u32_at(mdls, p + 12)?;
                        p += 32;
                        let first = mesh.vertices.len() as u32;
                        for k in 0..count {
                            let v = p + k * 48;
                            let f = |o: usize| f32_at(mdls, v + o);
                            let slot = u32_at(mdls, v + 12)? as usize;
                            mesh.vertices.push(Vertex {
                                joint: slots.get(slot).copied().unwrap_or(0),
                                normal: [f(0)?, f(4)?, f(8)?],
                                position: [f(16)?, f(20)?, f(24)?],
                                weight: f(28)?,
                                uv: [f(32)?, f(36)?],
                            });
                            if k >= 2 {
                                let i = first + k as u32;
                                // Strips alternate winding; `winding` flips the whole strip.
                                let odd = (k + 1) % 2 == 0;
                                let tri = if (winding == 0) == odd { [i, i - 1, i - 2] } else { [i, i - 2, i - 1] };
                                mesh.triangles.push(tri);
                            }
                        }
                        p += count * 48;
                    }
                    other => return Err(Error::Format(format!("mdls: packet command {other} at {p:#x}"))),
                }
            }
            p += 0x20;
        }
        meshes.push(mesh);
    }
    let (info, info_size, data) = (mobj + word(2)?, word(3)?, mobj + word(4)?);
    let mut textures = Vec::new();
    let mut at = data;
    for t in 0..info_size / 16 {
        let i = info + t * 16;
        let w = u16::from_le_bytes([mdls[i + 4], mdls[i + 5]]) as u32;
        let h = u16::from_le_bytes([mdls[i + 6], mdls[i + 7]]) as u32;
        let n = (w * h) as usize;
        let pixels = mdls.get(at..at + n).ok_or_else(|| Error::Format("mdls: texture past the end".into()))?.to_vec();
        at += n;
        textures.push(Texture { width: w, height: h, pixels, palette: Vec::new() });
    }
    for t in textures.iter_mut() {
        t.palette = mdls.get(at..at + 1024).ok_or_else(|| Error::Format("mdls: palette past the end".into()))?.to_vec();
        at += 1024;
    }
    Ok((meshes, textures))
}
