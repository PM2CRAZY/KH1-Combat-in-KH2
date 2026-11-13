//! KH1's model holds the Keyblade in KH1's palm (`model = kh1`).
//!
//! KH2 hangs a weapon on the bones of a `sklt` row (`03system.bin`, kept at `0x142AE5798`; rows of
//! 8 bytes from `+8`: id u32, right bone i16, left bone i16; looked up by `0x1403EAAE0`, Checked
//! (decomp)). The row is the object's weapon joint, its `00objentry.bin` row's byte `+7`
//! (`0x1403B5C10` reads it through the object table, `0x1403D7620` the bone, Checked (decomp)).
//! KH2 Sora (objentry 84, `P_EX100`) has row 1: his palms, 178 / 86 (Checked (files)). On KH1's
//! skeleton those are other bones, so the Keyblade floated (the owner, t44 / t48).
//!
//! So, once KH2 has loaded both tables at boot (`0x14039CAF0` loads the object table, then the
//! table loader `0x1403F4C30` the `sklt`, Checked (decomp)): a row of ours is added to `sklt`
//! with KH1's palms (`kh1_assets::player_model::PALMS`: 101 / 206, KH1's grip bone 280's parent
//! and the left one), and P_EX100's weapon joint is set to it. Only P_EX100: the forms and the
//! world outfits (their own objects and models) keep KH2's row 1. The Keyblade's motions on
//! KH1's model are KH1's own hold in that palm (`kh1_assets::swing_triggers::keyblade_motion`).
//! Nothing runs per frame.

use super::log;

/// The object tables KH2 searches (`0x1403E0F10`: up to 3, each `+4` count, rows of 0x60 from
/// `+8`, `+0` id, `+7` weapon joint, `+8` model name; Checked (decomp)).
const OBJENTRY_TABLES: usize = 0x2A2_5030;
const OBJENTRY_ROW: usize = 0x60;
/// The `sklt` table pointer (`0x1403EAB20` stores it, Checked (decomp)).
const SKLT: usize = 0x2AE_5798;
/// KH2 Sora's object (Checked (files): `P_EX100`, moveset `P_EX100.mset`).
const SORA_OBJECT: u32 = 84;

unsafe fn sklt_row(base: usize) -> Option<u8> {
    let at = (base + SKLT) as *mut usize;
    let t = *at;
    if t == 0 {
        return None;
    }
    let n = (*((t + 4) as *const i32)).clamp(0, 255) as usize;
    let rows = std::slice::from_raw_parts((t + 8) as *const [u8; 8], n);
    let [right, left] = kh1_assets::player_model::PALMS;
    if let Some(r) = rows.iter().find(|r| i16::from_le_bytes([r[4], r[5]]) == right && i16::from_le_bytes([r[6], r[7]]) == left) {
        return Some(r[0]);
    }
    let id = rows.iter().map(|r| u32::from_le_bytes([r[0], r[1], r[2], r[3]])).max().unwrap_or(0) + 1;
    if id > 255 {
        return None;
    }
    // KH2's table, a row added at its end (ids rise, so the binary search still finds them).
    let mut new = std::slice::from_raw_parts(t as *const u8, 8 + n * 8).to_vec();
    new[4..8].copy_from_slice(&((n + 1) as i32).to_le_bytes());
    new.extend_from_slice(&id.to_le_bytes());
    new.extend_from_slice(&right.to_le_bytes());
    new.extend_from_slice(&left.to_le_bytes());
    *at = Box::leak(new.into_boxed_slice()).as_ptr() as usize;
    Some(id as u8)
}

/// After KH2's table loader: KH1's palms as P_EX100's weapon bones.
pub unsafe fn patch(base: usize) {
    let Some(row) = sklt_row(base) else {
        log!("model kh1: no sklt table yet; the Keyblade stays on KH2's hand bones");
        return;
    };
    for k in 0..3 {
        let t = *((base + OBJENTRY_TABLES + k * 8) as *const usize);
        if t == 0 {
            continue;
        }
        let n = (*((t + 4) as *const i32)).clamp(0, 0x10000) as usize;
        for i in 0..n {
            let r = t + 8 + i * OBJENTRY_ROW;
            let name = std::slice::from_raw_parts((r + 8) as *const u8, 8);
            if *(r as *const u32) == SORA_OBJECT && name == b"P_EX100\0" {
                let at = (r + 7) as *mut u8;
                log!("model kh1: P_EX100's weapon joint {} -> sklt row {row} (KH1's palms {:?})", *at, kh1_assets::player_model::PALMS);
                *at = row;
                return;
            }
        }
    }
    log!("model kh1: P_EX100 not in the object table; the Keyblade stays on KH2's hand bones");
}
