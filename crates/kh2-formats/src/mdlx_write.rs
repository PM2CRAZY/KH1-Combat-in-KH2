//! Writes a KH2 skeletal model (the MDLX model entry, BAR type 4): the inverse of
//! [`crate::mdlx`]'s reader, in the packet layout KH2's own models use (checked against the
//! player's `obj/P_EX100.mdlx`):
//!
//! - 0x90 reserved zero bytes, then a 0x20-byte header: `3, 0, 0, shadow offset (0: none),
//!   bone count u16, texture count u16, bone table offset, skeleton data offset, group count`,
//!   then 0x20 bytes a group: `attributes, texture, triangle count, 0, DMA offset, matrix list
//!   offset, DMA tag count, 0`. Offsets are from the end of the reserved bytes.
//! - The skeleton data (0x120 bytes: bounding box, IK bias, 56 body-part bone numbers, a
//!   distance), then 0x40 bytes a bone.
//! - Per group: its packets' VIF code, its DMA tags, its matrix list.
//! - A packet: VU memory `0` header (`1,0,0,0` / `index count, index place, count list place,
//!   first matrix place` / `0` / `vertex count, vertex place, 0, matrix count`), the index
//!   entries (u, v in 1/4096; vertex index; strip flag), the bone-local vertices grouped by
//!   matrix, how many each matrix has, then a 4-quadword matrix per bone. Its VIF code:
//!   STCYCL 1,1 then UNPACKs with the double-buffer flag: V4-32 header, V2-16 u/v, masked
//!   V1-8 vertex indices (to z), masked V1-8 flags (to w), STCOL 1.0 and masked V3-32 positions
//!   (w = 1), V4-32 counts. Its DMA tags: a reference to the code, one reference per matrix (its
//!   address the bone number, its VIF words STCYCL and an UNPACK to the matrix's place), then
//!   MSCNT. The matrix list: each packet's bones in matrix order, packets parted by -1.
//!
//! Every triangle is written as its own three index entries: two that only feed the strip
//! (flag 0x10) and one that draws (0x20, the reader's `[0, 1, 2]` order).

use crate::anim::Bone;
use crate::mdlx::Group;

const RESERVED: usize = 0x90;
const HEADER: usize = 0x20;
const GROUP: usize = 0x20;
const SKELETON_DATA: usize = 0x120;
/// VU memory a packet may use (quadwords), as OpenKH's writer keeps to; KH2's own packets
/// stay under it too (Sora's largest about 0xCA).
const VU_LIMIT: usize = 0xE0;
const STCYCL: u32 = 0x0100_0101;

/// One packet's contents before encoding.
#[derive(Default)]
struct Packet {
    bones: Vec<u16>,
    /// Unique vertices: (bone, position bits).
    verts: Vec<(u16, [u32; 3])>,
    /// Index entries: (u, v, vertex in `verts`, flag).
    entries: Vec<(i16, i16, usize, u8)>,
}

impl Packet {
    fn vu_size(bones: usize, verts: usize, entries: usize) -> usize {
        4 + entries + verts + bones.div_ceil(4) + bones * 4
    }
}

fn pack_group(g: &Group) -> Vec<Packet> {
    let mut out = Vec::new();
    let mut p = Packet::default();
    for tri in &g.triangles {
        let new_bones: Vec<u16> = tri.iter().map(|v| v.bone).filter(|b| !p.bones.contains(b)).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
        let new_verts = tri.iter().filter(|v| !p.verts.contains(&(v.bone, v.position.map(f32::to_bits)))).count();
        let fits = Packet::vu_size(p.bones.len() + new_bones.len(), p.verts.len() + new_verts, p.entries.len() + 3) <= VU_LIMIT
            && p.entries.len() + 3 <= 255
            && p.verts.len() + new_verts <= 255;
        if !fits && !p.entries.is_empty() {
            out.push(std::mem::take(&mut p));
        }
        for (k, v) in tri.iter().enumerate() {
            if !p.bones.contains(&v.bone) {
                p.bones.push(v.bone);
            }
            let key = (v.bone, v.position.map(f32::to_bits));
            let i = match p.verts.iter().position(|x| *x == key) {
                Some(i) => i,
                None => {
                    p.verts.push(key);
                    p.verts.len() - 1
                }
            };
            let uv = |x: f32| (x * 4096.0).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            p.entries.push((uv(v.uv[0]), uv(v.uv[1]), i, if k == 2 { 0x20 } else { 0x10 }));
        }
    }
    if !p.entries.is_empty() {
        out.push(p);
    }
    out
}

fn w32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn pad16(out: &mut Vec<u8>) {
    while out.len() % 16 != 0 {
        out.push(0);
    }
}

/// A packet's VIF code (padded to quadwords) and its matrix places, its bones in that order.
fn encode(p: &Packet) -> (Vec<u8>, Vec<(u16, u32)>) {
    // Vertices grouped by bone, in the packet's bone order.
    let mut order: Vec<usize> = (0..p.verts.len()).collect();
    order.sort_by_key(|&i| p.bones.iter().position(|b| *b == p.verts[i].0).unwrap());
    let mut new_index = vec![0usize; p.verts.len()];
    for (n, &i) in order.iter().enumerate() {
        new_index[i] = n;
    }
    let counts: Vec<u32> = p.bones.iter().map(|b| p.verts.iter().filter(|v| v.0 == *b).count() as u32).collect();
    let (ni, nv, nm) = (p.entries.len(), p.verts.len(), p.bones.len());
    let idx_at = 4;
    let vert_at = idx_at + ni;
    let counts_at = vert_at + nv;
    let matrix_at = counts_at + nm.div_ceil(4);
    let unpack = |cmd: u32, n: usize, imm: u32| (cmd << 24) | (((n & 0xFF) as u32) << 16) | imm;
    let mut c = Vec::new();
    w32(&mut c, STCYCL);
    w32(&mut c, unpack(0x6C, 4, 0x8000));
    for q in [[1u32, 0, 0, 0], [ni as u32, idx_at as u32, counts_at as u32, matrix_at as u32], [0, 0, 0, 0], [nv as u32, vert_at as u32, 0, nm as u32]] {
        for x in q {
            w32(&mut c, x);
        }
    }
    w32(&mut c, STCYCL);
    w32(&mut c, unpack(0x65, ni, 0x8000 | idx_at as u32));
    for e in &p.entries {
        c.extend_from_slice(&e.0.to_le_bytes());
        c.extend_from_slice(&e.1.to_le_bytes());
    }
    for (mask, field) in [(0xCFCF_CFCFu32, 0usize), (0x3F3F_3F3F, 1)] {
        w32(&mut c, 0x2000_0000);
        w32(&mut c, mask);
        w32(&mut c, STCYCL);
        w32(&mut c, unpack(0x72, ni, 0xC000 | idx_at as u32));
        for e in &p.entries {
            c.push(if field == 0 { new_index[e.2] as u8 } else { e.3 });
        }
        while c.len() % 4 != 0 {
            c.push(0);
        }
    }
    w32(&mut c, 0x3100_0000);
    for _ in 0..4 {
        w32(&mut c, 1.0f32.to_bits());
    }
    w32(&mut c, 0x2000_0000);
    w32(&mut c, 0x8080_8080);
    w32(&mut c, STCYCL);
    w32(&mut c, unpack(0x78, nv, 0x8000 | vert_at as u32));
    for &i in &order {
        for x in p.verts[i].1 {
            w32(&mut c, x);
        }
    }
    w32(&mut c, STCYCL);
    let cq = nm.div_ceil(4);
    w32(&mut c, unpack(0x6C, cq, 0x8000 | counts_at as u32));
    for k in 0..cq * 4 {
        w32(&mut c, counts.get(k).copied().unwrap_or(0));
    }
    pad16(&mut c);
    let slots = p.bones.iter().enumerate().map(|(k, &b)| (b, (matrix_at + 4 * k) as u32)).collect();
    (c, slots)
}

/// The model entry for `bones`, `groups` (their triangles, bone-local) and `texture_count`
/// textures, with `skeleton_data` (0x120 bytes) as is.
pub fn write_model(bones: &[Bone], groups: &[Group], texture_count: u16, skeleton_data: &[u8; SKELETON_DATA]) -> Vec<u8> {
    let mut out = vec![0u8; RESERVED + HEADER + GROUP * groups.len()];
    let rel = |o: usize| (o - RESERVED) as u32;
    let skel_at = out.len();
    out.extend_from_slice(skeleton_data);
    let bone_at = out.len();
    for b in bones {
        for v in [b.index, b.sibling, b.parent, b.child] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        w32(&mut out, 0);
        out.extend_from_slice(&b.flags.to_le_bytes());
        for x in b.scale {
            w32(&mut out, x.to_bits());
        }
        w32(&mut out, b.length.to_bits());
        for x in b.rotation {
            w32(&mut out, x.to_bits());
        }
        w32(&mut out, 0);
        for x in b.translation {
            w32(&mut out, x.to_bits());
        }
        w32(&mut out, 0);
    }
    let mut descs = Vec::new();
    for g in groups {
        pad16(&mut out);
        let packets = pack_group(g);
        let mut coded = Vec::new();
        for p in &packets {
            let at = out.len();
            let (code, slots) = encode(p);
            out.extend_from_slice(&code);
            coded.push((at, code.len() / 16, slots));
        }
        let dma_at = out.len();
        let mut tags = 0u32;
        let tag = |out: &mut Vec<u8>, qwc: u16, id: u16, addr: u32, v0: u32, v1: u32| {
            out.extend_from_slice(&qwc.to_le_bytes());
            out.extend_from_slice(&(id << 12).to_le_bytes());
            w32(out, addr);
            w32(out, v0);
            w32(out, v1);
        };
        for (at, qwc, slots) in &coded {
            tag(&mut out, *qwc as u16, 3, rel(*at), 0, 0);
            for &(bone, slot) in slots {
                tag(&mut out, 4, 3, bone as u32, STCYCL, 0x6C04_8000 | slot);
            }
            tag(&mut out, 0, 1, 0, 0x1700_0000, 0);
            tags += 2 + slots.len() as u32;
        }
        let list_at = out.len();
        let mut list: Vec<i32> = Vec::new();
        for (k, (_, _, slots)) in coded.iter().enumerate() {
            if k > 0 {
                list.push(-1);
            }
            list.extend(slots.iter().map(|s| s.0 as i32));
        }
        w32(&mut out, list.len() as u32);
        for x in list {
            out.extend_from_slice(&x.to_le_bytes());
        }
        descs.push([g.attributes, g.texture as u32, g.triangles.len() as u32, 0, rel(dma_at), rel(list_at), tags, 0]);
    }
    pad16(&mut out);
    let mut h = Vec::new();
    for x in [3u32, 0, 0, 0] {
        w32(&mut h, x);
    }
    h.extend_from_slice(&(bones.len() as u16).to_le_bytes());
    h.extend_from_slice(&texture_count.to_le_bytes());
    w32(&mut h, rel(bone_at));
    w32(&mut h, rel(skel_at));
    w32(&mut h, groups.len() as u32);
    out[RESERVED..RESERVED + HEADER].copy_from_slice(&h);
    for (k, d) in descs.iter().enumerate() {
        let at = RESERVED + HEADER + k * GROUP;
        for (i, x) in d.iter().enumerate() {
            out[at + i * 4..at + i * 4 + 4].copy_from_slice(&x.to_le_bytes());
        }
    }
    out
}

/// An 8-bit texture for [`write_textures`]: indices, and its 256 colours (RGBA, PS2 alpha where
/// 0x80 is opaque) in GS order (entries 8–15 and 16–23 of every 32 swapped, as KH1 and KH2
/// both store them).
pub struct IndexedTexture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub palette: Vec<u8>,
}

/// GS memory the textures go to (block numbers), where KH2's Sora keeps his: all palettes in one
/// 64x32 CT32 page at 0x3A00 (4 blocks a palette), the images after it.
const CLUT_BASE: usize = 0x3A00;
const IMAGE_BASE: usize = 0x3A20;

fn log2(n: u32) -> u64 {
    n.trailing_zeros() as u64
}

/// The texture container (BAR type 7, `tim_`) for `textures`, laid out as KH2's own model
/// textures are (checked against `P_EX100.mdlx`): a 0x24-byte header (`0, palette words, image
/// count, texture count, map offset, upload list, register list, image data, palette data`),
/// the map (texture n uses register set n), one 0x90-byte upload for the palettes then one per
/// image (each image sent as CT32 with half its texture width), 0xA0 bytes of GS registers a
/// texture (TEXFLUSH, MIPTBP1/2, TEX2, TEX1, TEX0, CLAMP), the images, the palettes, `_KN5`.
/// `repeat`: textures wrap (KH1's UVs leave 0–1) rather than clamp to their edge.
pub fn write_textures(textures: &[IndexedTexture], repeat: bool) -> crate::Result<Vec<u8>> {
    use crate::mdlx::{Gs, GS_BYTES};
    if textures.len() > 8 {
        return crate::bad("more than 8 palettes do not fit the palette page");
    }
    let mut gs = Gs { mem: vec![0; GS_BYTES] };
    for (k, t) in textures.iter().enumerate() {
        if t.palette.len() != 1024 || !t.width.is_power_of_two() || !t.height.is_power_of_two() || t.width < 128 || t.height < 64 {
            return crate::bad(format!("texture {k}: needs 1024 palette bytes and power-of-two size at least 128x64"));
        }
        for n in 0..256 {
            let a = Gs::addr32(CLUT_BASE + 4 * k, 1, n & 15, n >> 4);
            gs.mem[a..a + 4].copy_from_slice(&t.palette[n * 4..n * 4 + 4]);
        }
    }
    // (block pointer, texture width in 64s, upload width/height in CT32 pixels)
    let mut images = Vec::new();
    let mut bp = IMAGE_BASE;
    for t in textures {
        let tbw = (t.width / 64) as usize;
        for y in 0..t.height as usize {
            for x in 0..t.width as usize {
                gs.mem[Gs::addr8(bp, tbw, x, y)] = t.pixels[y * t.width as usize + x];
            }
        }
        let (w, h) = (t.width as usize / 2, t.height as usize / 2);
        images.push((bp, tbw, w, h));
        bp += t.width as usize * t.height as usize / 256;
        if bp > 0x4000 {
            return crate::bad("textures past the end of GS memory");
        }
    }
    let read32 = |bp: usize, bw: usize, w: usize, h: usize| -> Vec<u8> {
        let mut out = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                let a = Gs::addr32(bp, bw, x, y);
                out.extend_from_slice(&gs.mem[a..a + 4]);
            }
        }
        out
    };
    let header = 0x24usize;
    let map_len = textures.len();
    let uploads_at = (header + map_len).next_multiple_of(16);
    let regs_at = uploads_at + 0x90 * (1 + textures.len());
    let data_at = (regs_at + 0xA0 * textures.len()).next_multiple_of(0x80);
    let mut pictures = Vec::new();
    let mut picture_at = Vec::new();
    for &(bp, tbw, w, h) in &images {
        picture_at.push(data_at + pictures.len());
        pictures.extend(read32(bp, (tbw / 2).max(1), w, h));
    }
    let palette_at = data_at + pictures.len();
    let palettes = read32(CLUT_BASE, 1, 64, 32);
    let mut out = vec![0u8; header];
    let q = |out: &mut Vec<u8>, v: u64| out.extend_from_slice(&v.to_le_bytes());
    for (k, x) in [0u32, (palettes.len() / 4) as u32, textures.len() as u32, textures.len() as u32, header as u32, uploads_at as u32, regs_at as u32, data_at as u32, palette_at as u32].iter().enumerate() {
        out[k * 4..k * 4 + 4].copy_from_slice(&x.to_le_bytes());
    }
    out.extend((0..map_len).map(|k| k as u8));
    pad16(&mut out);
    let upload = |out: &mut Vec<u8>, dbp: usize, dbw: usize, w: usize, h: usize, src: usize| {
        let qwc = (w * h * 4 / 16) as u64;
        q(out, 0x1000_0006);
        q(out, 0x5000_0006_1300_0000);
        q(out, 0x1000_0000_0000_0004);
        q(out, 0x0E);
        q(out, ((dbp as u64) << 32) | ((dbw as u64) << 48));
        q(out, 0x50);
        q(out, 0);
        q(out, 0x51);
        q(out, ((h as u64) << 32) | w as u64);
        q(out, 0x52);
        q(out, 0);
        q(out, 0x53);
        q(out, 0x0800_0000_0000_8000 | qwc);
        q(out, 0);
        q(out, ((src as u64) << 32) | 0x3000_0000 | qwc);
        q(out, (0x5000_0000 | qwc) << 32);
        q(out, 0x6000_0000);
        q(out, 0x1300_0000);
    };
    upload(&mut out, CLUT_BASE, 1, 64, 32, palette_at);
    for (k, &(bp, tbw, w, h)) in images.iter().enumerate() {
        upload(&mut out, bp, (tbw / 2).max(1), w, h, picture_at[k]);
    }
    for (k, (t, &(bp, tbw, _, _))) in textures.iter().zip(&images).enumerate() {
        let cbp = (CLUT_BASE + 4 * k) as u64;
        // TEX0: PSMT8, colour + alpha, modulate; palette CT32, CSM1, offset 0.
        let tex0 = bp as u64 | (tbw as u64) << 14 | 19 << 20 | log2(t.width) << 26 | log2(t.height) << 30 | 1 << 34 | cbp << 37;
        // TEX2: the palette, loaded when its pointer changes (CLD 4).
        let tex2 = 19 << 20 | cbp << 37 | 4 << 61;
        let (mu, mv) = (t.width as u64 - 1, t.height as u64 - 1);
        let clamp = if repeat { 0 } else { 2 | 2 << 2 | mu << 14 | mv << 34 };
        q(&mut out, 0x1000_0008);
        q(&mut out, 0x5000_0008_1300_0000);
        q(&mut out, 0x1000_0000_0000_8007);
        q(&mut out, 0x0E);
        for (v, r) in [(0, 0x3F), (0, 0x34), (0, 0x36), (tex2, 0x16), (0x20, 0x14), (tex0, 0x06), (clamp, 0x08)] {
            q(&mut out, v);
            q(&mut out, r);
        }
        q(&mut out, 0x6000_0000);
        q(&mut out, 0x1300_0000);
    }
    out.resize(data_at, 0);
    out.extend(pictures);
    out.extend(palettes);
    out.extend_from_slice(b"_KN5");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textures_read_back() {
        let mut seed = 7u32;
        let mut rnd = || {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            (seed >> 16) as u8
        };
        let textures: Vec<IndexedTexture> = [(128u32, 128u32), (256, 64), (128, 128)]
            .iter()
            .map(|&(width, height)| IndexedTexture { width, height, pixels: (0..width * height).map(|_| rnd()).collect(), palette: (0..1024).map(|_| rnd() & 0x7F).collect() })
            .collect();
        let tim = write_textures(&textures, false).unwrap();
        let back = crate::mdlx::parse_textures(&tim).unwrap();
        assert_eq!(back.len(), textures.len());
        for (t, b) in textures.iter().zip(&back) {
            assert_eq!((t.width, t.height), (b.width, b.height));
            for (i, &px) in t.pixels.iter().enumerate() {
                let j = (px as usize & !0x18) | ((px as usize & 8) << 1) | ((px as usize & 16) >> 1);
                let c = &t.palette[j * 4..j * 4 + 4];
                assert_eq!(&b.rgba[i * 4..i * 4 + 3], &c[..3], "pixel {i}");
                assert_eq!(b.rgba[i * 4 + 3], c[3] * 2);
            }
        }
    }
}
