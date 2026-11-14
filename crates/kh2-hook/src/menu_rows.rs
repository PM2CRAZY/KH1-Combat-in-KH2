//! KH2's command menu with room for 12 rows a submenu (owner's choice (a1), 2026-11-14;
//! `docs/command-menu-rows.md`).
//!
//! KH2's one `YS::PLAYER_COMMAND` (Steam `0x142A10620`) keeps its 19 submenus inline, `0x80`
//! bytes each: a header (`+0x00` kind, `+0x08` owner, `+0x10` row count), 8 rows of 12 bytes at
//! `+0x14`, then the cursor row (`+0x74`) and cursor command (`+0x78`). Here they move into a
//! block of ours with 12 rows each and the two cursor fields just *before* the header
//! (`-0x0C`, `-0x08`), so every cursor access keeps its 8-bit displacement (`0x74` -> `0xF4`,
//! `0x78` -> `0xF8`) and every row keeps its offset. Each place KH2 makes a submenu's address
//! from the object is pointed at the block: in place where the instruction has a 32-bit
//! displacement, through a short jump out and back where it doesn't. KH2's own code then runs the
//! menu as before; nothing of ours runs per frame. Checked (exe: every site's bytes, read from the
//! Steam exe; decomp: each function read).
//!
//! Step 3 (t78): the command HUD draws up to 16 sprites a menu level (was 9), so a submenu's
//! title and 12 rows fit (`hud_patches`).
//!
//! This module only computes the bytes (so it can be tested anywhere); `win::menu_rows` writes them.

/// The `YS::PLAYER_COMMAND` object (Steam RVA).
pub const PC_RVA: u32 = 0x2A1_0620;
/// The `YS::FIELD_COMMAND` vtable KH2's static init puts in the object after its ctor: the
/// object is built once this is there.
pub const BUILT_VTABLE_RVA: u32 = 0x5C_4C68;
/// The static init's `call` of the object's ctor (`0x1403B1C70`).
pub const CTOR_CALL_RVA: u32 = 0x2_4571;
pub const CTOR_RVA: u32 = 0x3B_1C70;
pub const SUBMENUS: usize = 19;
pub const OLD_ROWS: usize = 8;
pub const ROWS: usize = 12;
pub const ROW: usize = 12;
/// Where submenu `n` is inside the object (KH2's layout).
pub const OLD_FIRST: usize = 0x28;
pub const OLD_STRIDE: usize = 0x80;
/// Old cursor fields.
pub const OLD_CURSOR_ROW: usize = 0x74;
pub const OLD_CURSOR_CMD: usize = 0x78;
/// A slot of the block: the header sits at `HEADER` (so a header has the same alignment, 8 mod 16,
/// as KH2's), the cursor row at `HEADER - 0x0C`, the cursor command at `HEADER - 0x08`.
pub const SLOT: usize = 0xC0;
pub const HEADER: usize = 0x18;
pub const CURSOR_ROW: i8 = -0x0C;
pub const CURSOR_CMD: i8 = -0x08;
pub const BLOCK: usize = SUBMENUS * SLOT;
/// The jumps out start here in our memory (after the block).
pub const CAVES: usize = 0xE40;
/// Our memory: the block, then the jumps out.
pub const MEM: usize = 0x2000;
/// The submenu stack: pointers at `+0xAF0` (15), top pointer at `+0x9A8`.
pub const STACK: usize = 0xAF0;
pub const STACK_LEN: usize = 15;
pub const STACK_TOP: usize = 0x9A8;

const _: () = assert!(HEADER as i32 + CURSOR_ROW as i32 >= 0 && HEADER + 0x14 + ROWS * ROW <= SLOT && BLOCK <= CAVES && SLOT % 16 == 0);

/// A cursor access on a submenu pointer: (RVA, the instruction's bytes, where its 8-bit
/// displacement `0x74` / `0x78` is).
pub const CURSOR_SITES: &[(u32, &[u8], usize)] = &[
    (0x015b833, &[57, 126, 116], 2),  // cmp dword ptr [rsi + 0x74], edi
    (0x015b977, &[72, 99, 70, 116], 3),  // movsxd rax, dword ptr [rsi + 0x74]
    (0x015ba19, &[68, 139, 82, 116], 3),  // mov r10d, dword ptr [rdx + 0x74]
    (0x03b1140, &[72, 99, 65, 116], 3),  // movsxd rax, dword ptr [rcx + 0x74]
    (0x03b1154, &[68, 139, 81, 120], 3),  // mov r10d, dword ptr [rcx + 0x78]
    (0x03b1185, &[69, 137, 88, 120], 3),  // mov dword ptr [r8 + 0x78], r11d
    (0x03b1192, &[65, 137, 72, 116], 3),  // mov dword ptr [r8 + 0x74], ecx
    (0x03b11a0, &[65, 137, 64, 120], 3),  // mov dword ptr [r8 + 0x78], eax
    (0x03b11aa, &[65, 199, 64, 116, 0, 0, 0, 0], 3),  // mov dword ptr [r8 + 0x74], 0
    (0x03b11b2, &[65, 137, 64, 120], 3),  // mov dword ptr [r8 + 0x78], eax
    (0x03b11bc, &[199, 65, 116, 0, 0, 0, 0], 2),  // mov dword ptr [rcx + 0x74], 0
    (0x03b11c3, &[137, 65, 120], 2),  // mov dword ptr [rcx + 0x78], eax
    (0x03b1260, &[137, 120, 116], 2),  // mov dword ptr [rax + 0x74], edi
    (0x03b1263, &[137, 112, 120], 2),  // mov dword ptr [rax + 0x78], esi
    (0x03b12ac, &[137, 115, 116], 2),  // mov dword ptr [rbx + 0x74], esi
    (0x03b12ed, &[137, 123, 116], 2),  // mov dword ptr [rbx + 0x74], edi
    (0x03b1341, &[72, 99, 67, 116], 3),  // movsxd rax, dword ptr [rbx + 0x74]
    (0x03b1352, &[68, 139, 83, 120], 3),  // mov r10d, dword ptr [rbx + 0x78]
    (0x03b1383, &[68, 137, 91, 120], 3),  // mov dword ptr [rbx + 0x78], r11d
    (0x03b1391, &[137, 75, 116], 2),  // mov dword ptr [rbx + 0x74], ecx
    (0x03b139d, &[137, 67, 120], 2),  // mov dword ptr [rbx + 0x78], eax
    (0x03b13a4, &[137, 123, 116], 2),  // mov dword ptr [rbx + 0x74], edi
    (0x03b13a7, &[137, 67, 120], 2),  // mov dword ptr [rbx + 0x78], eax
    (0x03b13ac, &[137, 123, 116], 2),  // mov dword ptr [rbx + 0x74], edi
    (0x03b13af, &[137, 115, 120], 2),  // mov dword ptr [rbx + 0x78], esi
    (0x03b1421, &[137, 122, 116], 2),  // mov dword ptr [rdx + 0x74], edi
    (0x03b1424, &[137, 114, 120], 2),  // mov dword ptr [rdx + 0x78], esi
    (0x03b1462, &[72, 99, 75, 116], 3),  // movsxd rcx, dword ptr [rbx + 0x74]
    (0x03b1475, &[137, 83, 116], 2),  // mov dword ptr [rbx + 0x74], edx
    (0x03b1486, &[137, 115, 120], 2),  // mov dword ptr [rbx + 0x78], esi
    (0x03b149d, &[137, 75, 116], 2),  // mov dword ptr [rbx + 0x74], ecx
    (0x03b14ab, &[137, 115, 120], 2),  // mov dword ptr [rbx + 0x78], esi
    (0x03b14bd, &[137, 123, 116], 2),  // mov dword ptr [rbx + 0x74], edi
    (0x03b14c0, &[137, 115, 120], 2),  // mov dword ptr [rbx + 0x78], esi
    (0x03b14da, &[72, 99, 83, 116], 3),  // movsxd rdx, dword ptr [rbx + 0x74]
    (0x03b14ec, &[137, 67, 116], 2),  // mov dword ptr [rbx + 0x74], eax
    (0x03b14fd, &[137, 115, 120], 2),  // mov dword ptr [rbx + 0x78], esi
    (0x03b150c, &[137, 123, 116], 2),  // mov dword ptr [rbx + 0x74], edi
    (0x03b150f, &[137, 115, 120], 2),  // mov dword ptr [rbx + 0x78], esi
    (0x03b151c, &[137, 123, 116], 2),  // mov dword ptr [rbx + 0x74], edi
    (0x03b1529, &[137, 115, 120], 2),  // mov dword ptr [rbx + 0x78], esi
    (0x03b153d, &[72, 99, 67, 116], 3),  // movsxd rax, dword ptr [rbx + 0x74]
    (0x03b1b70, &[199, 65, 116, 0, 0, 0, 0], 2),  // mov dword ptr [rcx + 0x74], 0
    (0x03b1b77, &[199, 65, 120, 255, 255, 255, 255], 2),  // mov dword ptr [rcx + 0x78], 0xffffffff
    (0x03b1b85, &[137, 81, 116], 2),  // mov dword ptr [rcx + 0x74], edx
    (0x03b1b94, &[137, 65, 120], 2),  // mov dword ptr [rcx + 0x78], eax
    (0x03b1b98, &[199, 65, 116, 0, 0, 0, 0], 2),  // mov dword ptr [rcx + 0x74], 0
    (0x03b1b9f, &[199, 65, 120, 255, 255, 255, 255], 2),  // mov dword ptr [rcx + 0x78], 0xffffffff
    (0x03b21e3, &[73, 99, 64, 116], 3),  // movsxd rax, dword ptr [r8 + 0x74]
    (0x03f232e, &[72, 99, 67, 116], 3),  // movsxd rax, dword ptr [rbx + 0x74]
    (0x03f2360, &[72, 99, 67, 116], 3),  // movsxd rax, dword ptr [rbx + 0x74]
    (0x03f2376, &[72, 99, 67, 116], 3),  // movsxd rax, dword ptr [rbx + 0x74]
    (0x03f23a6, &[72, 99, 66, 116], 3),  // movsxd rax, dword ptr [rdx + 0x74]
    (0x03f23c0, &[72, 99, 67, 116], 3),  // movsxd rax, dword ptr [rbx + 0x74]
    (0x03f2415, &[72, 99, 66, 116], 3),  // movsxd rax, dword ptr [rdx + 0x74]
    (0x03f2427, &[72, 99, 83, 116], 3),  // movsxd rdx, dword ptr [rbx + 0x74]
    (0x03f25ab, &[139, 82, 116], 2),  // mov edx, dword ptr [rdx + 0x74]
    (0x03f3b1b, &[139, 82, 116], 2),  // mov edx, dword ptr [rdx + 0x74]
];

/// An address of submenu `n` (or one of its fields) made as `object + disp32`: (RVA, the
/// instruction's bytes, where the 32-bit displacement or immediate is). KH2's value is
/// `0x28 + 0x80 n + field`.
pub const NUMBER_SITES: &[(u32, &[u8], usize)] = &[
    (0x03b26d5, &[72, 129, 193, 168, 5, 0, 0], 3),  // add rcx, 0x5a8  (submenu 11, +0x0)
    (0x03b26ed, &[72, 141, 139, 168, 5, 0, 0], 3),  // lea rcx, [rbx + 0x5a8]  (submenu 11, +0x0)
    (0x03b2b02, &[72, 141, 179, 40, 7, 0, 0], 3),  // lea rsi, [rbx + 0x728]  (submenu 14, +0x0)
    (0x03f27fb, &[72, 141, 141, 40, 3, 0, 0], 3),  // lea rcx, [rbp + 0x328]  (submenu 6, +0x0)
    (0x03f29ff, &[72, 129, 193, 40, 5, 0, 0], 3),  // add rcx, 0x528  (submenu 10, +0x0)
    (0x03f2a2a, &[72, 129, 193, 40, 2, 0, 0], 3),  // add rcx, 0x228  (submenu 4, +0x0)
    (0x03f2a84, &[72, 99, 135, 56, 2, 0, 0], 3),  // movsxd rax, dword ptr [rdi + 0x238]  (submenu 4, +0x10)
    (0x03f2aa4, &[72, 141, 143, 40, 2, 0, 0], 3),  // lea rcx, [rdi + 0x228]  (submenu 4, +0x0)
    (0x03f2ac5, &[72, 141, 143, 40, 4, 0, 0], 3),  // lea rcx, [rdi + 0x428]  (submenu 8, +0x0)
    (0x03f2b16, &[76, 99, 183, 56, 4, 0, 0], 3),  // movsxd r14, dword ptr [rdi + 0x438]  (submenu 8, +0x10)
    (0x03f2b4d, &[72, 141, 143, 40, 4, 0, 0], 3),  // lea rcx, [rdi + 0x428]  (submenu 8, +0x0)
    (0x03f2b70, &[198, 132, 135, 65, 4, 0, 0, 5], 3),  // mov byte ptr [rdi + rax*4 + 0x441], 5  (submenu 8, +0x19)
    (0x03f2b83, &[72, 141, 143, 168, 0, 0, 0], 3),  // lea rcx, [rdi + 0xa8]  (submenu 1, +0x0)
    (0x03f2b96, &[72, 141, 143, 168, 0, 0, 0], 3),  // lea rcx, [rdi + 0xa8]  (submenu 1, +0x0)
    (0x03f2ba2, &[72, 141, 159, 40, 1, 0, 0], 3),  // lea rbx, [rdi + 0x128]  (submenu 2, +0x0)
    (0x03f2c9c, &[72, 141, 143, 168, 1, 0, 0], 3),  // lea rcx, [rdi + 0x1a8]  (submenu 3, +0x0)
    (0x03f2d33, &[72, 141, 143, 168, 1, 0, 0], 3),  // lea rcx, [rdi + 0x1a8]  (submenu 3, +0x0)
    (0x03f2d5c, &[72, 141, 143, 168, 4, 0, 0], 3),  // lea rcx, [rdi + 0x4a8]  (submenu 9, +0x0)
    (0x03f2d84, &[72, 141, 143, 168, 3, 0, 0], 3),  // lea rcx, [rdi + 0x3a8]  (submenu 7, +0x0)
    (0x03f2d97, &[72, 141, 143, 168, 3, 0, 0], 3),  // lea rcx, [rdi + 0x3a8]  (submenu 7, +0x0)
    (0x03f2e0e, &[72, 141, 143, 40, 6, 0, 0], 3),  // lea rcx, [rdi + 0x628]  (submenu 12, +0x0)
    (0x03f2e21, &[72, 141, 143, 40, 6, 0, 0], 3),  // lea rcx, [rdi + 0x628]  (submenu 12, +0x0)
    (0x03f2f0a, &[72, 141, 143, 168, 2, 0, 0], 3),  // lea rcx, [rdi + 0x2a8]  (submenu 5, +0x0)
    (0x03f2faf, &[72, 141, 143, 168, 2, 0, 0], 3),  // lea rcx, [rdi + 0x2a8]  (submenu 5, +0x0)
    (0x03f387a, &[72, 141, 177, 40, 3, 0, 0], 3),  // lea rsi, [rcx + 0x328]  (submenu 6, +0x0)
    (0x03f38ea, &[57, 135, 184, 3, 0, 0], 2),  // cmp dword ptr [rdi + 0x3b8], eax  (submenu 7, +0x10)
    (0x03f395c, &[72, 141, 143, 168, 1, 0, 0], 3),  // lea rcx, [rdi + 0x1a8]  (submenu 3, +0x0)
    (0x03f3999, &[72, 141, 143, 168, 1, 0, 0], 3),  // lea rcx, [rdi + 0x1a8]  (submenu 3, +0x0)
    (0x03947b4, &[72, 141, 139, 40, 5, 0, 0], 3),  // lea rcx, [rbx + 0x528]  (submenu 10, +0x0)
    (0x03947d2, &[72, 141, 139, 40, 5, 0, 0], 3),  // lea rcx, [rbx + 0x528]  (submenu 10, +0x0)
    (0x0403f38, &[72, 141, 139, 40, 7, 0, 0], 3),  // lea rcx, [rbx + 0x728]  (submenu 14, +0x0)
    (0x0403f4a, &[72, 141, 139, 40, 7, 0, 0], 3),  // lea rcx, [rbx + 0x728]  (submenu 14, +0x0)
];

/// The places too short for a 32-bit displacement: (RVA, KH2's bytes there). Each becomes a
/// jump to a stub that does the same with the block, then jumps back (`stub`).
pub const SHORT_SITES: &[(u32, &[u8])] = &[
    (0x3f27f2, &[0x48, 0x8D, 0x4D, 0x28, 0xE8, 0x75, 0xF3, 0xFB, 0xFF]), // lea rcx,[rbp+0x28]; call 0x1403b1b70
    (0x3f28d2, &[0x48, 0x8D, 0x59, 0x28, 0xBF, 0x13, 0, 0, 0]), // lea rbx,[rcx+0x28]; mov edi,0x13
    (0x3f28e8, &[0x48, 0x83, 0xEB, 0x80, 0x48, 0x83, 0xEF, 0x01]), // sub rbx,-0x80; sub rdi,1
    (0x3f2dab, &[0x48, 0x8D, 0x4F, 0x28, 0x0F, 0xB6, 0xD8]), // lea rcx,[rdi+0x28]; movzx ebx,al
    (0x3f2dcb, &[0x48, 0x8D, 0x4F, 0x28, 0x44, 0x0F, 0xB6, 0x48, 0x04]), // lea rcx,[rdi+0x28]; movzx r9d,byte [rax+4]
    (0x3f2ded, &[0x48, 0x8D, 0x57, 0x28, 0x48, 0x8B, 0xCF]), // lea rdx,[rdi+0x28]; mov rcx,rdi
    (0x3f2e4c, &[0x48, 0x8D, 0x4F, 0x28, 0x45, 0x33, 0xC0]), // lea rcx,[rdi+0x28]; xor r8d,r8d
    (0x3f2e62, &[0x48, 0x8D, 0x4F, 0x28, 0xBA, 0x03, 0, 0, 0]), // lea rcx,[rdi+0x28]; mov edx,3
    (0x3f2e8c, &[0x48, 0x8D, 0x4F, 0x28, 0x45, 0x33, 0xC0]), // lea rcx,[rdi+0x28]; xor r8d,r8d
    (0x3f2eab, &[0x48, 0x8D, 0x4F, 0x28, 0x41, 0xB9, 0x01, 0, 0, 0]), // lea rcx,[rdi+0x28]; mov r9d,1
    (0x3f3b0a, &[0x48, 0x83, 0xC1, 0x28, 0x48, 0x8B, 0x10, 0x49, 0x8B, 0xC0, 0x48, 0xC1, 0xE0, 0x07, 0x48, 0x03, 0xC8]), // add rcx,0x28; mov rdx,[rax]; mov rax,r8; shl rax,7; add rcx,rax
    (0x3f259d, &[0x48, 0xC1, 0xE1, 0x07, 0x48, 0x8B, 0x10, 0x48, 0x83, 0xC1, 0x28, 0x48, 0x03, 0xCB]), // shl rcx,7; mov rdx,[rax]; add rcx,0x28; add rcx,rbx
    (0x3b1f7f, &[0x48, 0xC1, 0xE0, 0x07, 0x48, 0x83, 0xC0, 0x28, 0x4C, 0x89, 0x81, 0xA8, 0x09, 0, 0, 0x48, 0x03, 0xC1]), // shl rax,7; add rax,0x28; mov [rcx+0x9a8],r8; add rax,rcx
    (0x3b281c, &[0x48, 0xC1, 0xE0, 0x07, 0x48, 0x83, 0xC0, 0x28, 0x4C, 0x89, 0x89, 0xA8, 0x09, 0, 0, 0x48, 0x03, 0xC1]), // shl rax,7; add rax,0x28; mov [rcx+0x9a8],r9; add rax,rcx
    (0x3b1002, &[0x48, 0xC1, 0xE0, 0x07, 0x48, 0x03, 0x43, 0x08]), // shl rax,7; add rax,[rbx+8]
    (0x3b193a, &[0x48, 0xC1, 0xE0, 0x07, 0x48, 0x03, 0x43, 0x08]), // shl rax,7; add rax,[rbx+8]
    (0x3b1acd, &[0x48, 0xC1, 0xE0, 0x07, 0x48, 0x03, 0x43, 0x08]), // shl rax,7; add rax,[rbx+8]
    // element -> Magic list, in KH1's order (`0x1403F27D0`, the pause menu's `0x14035B440`)
    (0x3f2826, &[0x8B, 0xCB, 0xE8, 0xD3, 0x30, 0xFD, 0xFF]), // mov ecx,ebx; call 0x1403c5900
    (0x35b7f4, &[0x8B, 0xCB, 0xE8, 0x45, 0x90, 0x06, 0x00]), // mov ecx,ebx; call 0x1403c4840
    (0x35b88f, &[0x8B, 0xCE, 0xE8, 0xAA, 0xA0, 0x06, 0x00]), // mov ecx,esi; call 0x1403c5940
    // the HUD's slots (`0x14015D140`, `0x14015E800`)
    (0x15d1b2, &[0x48, 0x8B, 0x74, 0xF9, 0x20, 0x49, 0x8B, 0x8C, 0x24, 0x98, 0x03, 0, 0]), // mov rsi,[rcx+rdi*8+0x20]; mov rcx,[r12+0x398]
    (0x15d302, &[0x49, 0x89, 0x74, 0xFE, 0x20, 0xEB, 0x44]), // mov [r14+rdi*8+0x20],rsi; jmp 0x14015d34d
    (0x15ea5f, &[0x49, 0x8D, 0x4E, 0x20, 0x4C, 0x89, 0x6C, 0x24, 0x50]), // lea rcx,[r14+0x20]; mov [rsp+0x50],r13
];


// ---- The command HUD (`dk::COMMAND_DRAW`): step 3 ------------------------------------------

/// KH2's command HUD (one object, made by `0x14015E410`, `0x670` bytes) keeps, for each of 4
/// menu levels, 9 sprite slots (`+0x20 + 8 (9 level + k)`: the title and 8 rows) and a flag byte
/// for each (`+0x140 + 9 level + k`); `DAT_140ABC4F8` counts the slots used this frame per level.
/// The object is made bigger and both arrays move past KH2's end, 16 slots a level (title, 12
/// rows, and the few extra sprites level 0 and the cursor add). Checked (exe + decomp).
pub const HUD_OLD_SIZE: u32 = 0x670;
/// `DAT_140ABC508`, the HUD (0 until KH2 makes it, on the first field load).
pub const HUD_POINTER_RVA: u32 = 0xA_BC508;
pub const HUD_LEVEL: u32 = 16;
pub const HUD_SPRITES: u32 = HUD_OLD_SIZE;
pub const HUD_FLAGS: u32 = HUD_SPRITES + 4 * HUD_LEVEL * 8;
pub const HUD_SIZE: u32 = HUD_FLAGS + 4 * HUD_LEVEL;
const OLD_SPRITES: u32 = 0x20;
const _: () = assert!(HUD_SIZE % 16 == 0 && HUD_SPRITES / 8 < 0x80 * 2);

/// KH2's `0x14015C5D0` (clears the slots and flags, the per-level row and cursor ints to -1).
const HUD_CLEAR_OLD: &[u8] = &[0xC7, 0x81, 0x90, 0x01, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x48, 0x8D, 0x91, 0x41, 0x01, 0x00, 0x00, 0x4C, 0x8D, 0x81, 0x64, 0x01, 0x00, 0x00, 0x45, 0x33, 0xC9, 0x48, 0x8D, 0x41, 0x28, 0xB9, 0x04, 0x00, 0x00, 0x00, 0x41, 0xC7, 0x40, 0x10, 0xFF, 0xFF, 0xFF, 0xFF, 0x48, 0x8D, 0x52, 0x09, 0x41, 0xC7, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x4D, 0x8D, 0x40, 0x04, 0x4C, 0x89, 0x48, 0xF8, 0x48, 0x8D, 0x40, 0x48, 0x44, 0x88, 0x4A, 0xF6, 0x4C, 0x89, 0x48, 0xB8, 0x44, 0x88, 0x4A, 0xF7, 0x4C, 0x89, 0x48, 0xC0, 0x44, 0x88, 0x4A, 0xF8, 0x4C, 0x89, 0x48, 0xC8, 0x44, 0x88, 0x4A, 0xF9, 0x4C, 0x89, 0x48, 0xD0, 0x44, 0x88, 0x4A, 0xFA, 0x4C, 0x89, 0x48, 0xD8, 0x44, 0x88, 0x4A, 0xFB, 0x4C, 0x89, 0x48, 0xE0, 0x44, 0x88, 0x4A, 0xFC, 0x4C, 0x89, 0x48, 0xE8, 0x44, 0x88, 0x4A, 0xFD, 0x4C, 0x89, 0x48, 0xF0, 0x44, 0x88, 0x4A, 0xFE, 0x48, 0x83, 0xE9, 0x01, 0x75, 0x97, 0xC3];

/// The same, for KH2's arrays and ours: `rcx` = the HUD.
fn hud_clear() -> Vec<u8> {
    let mut v = vec![0xC7, 0x81, 0x90, 0x01, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF]; // mov dword [rcx+0x190],-1
    v.extend([0x57, 0x48, 0x8B, 0xD1]); // push rdi; mov rdx,rcx
    v.extend([0x48, 0x8D, 0xB9, 0x64, 0x01, 0, 0]); // lea rdi,[rcx+0x164]
    v.extend([0x83, 0xC8, 0xFF, 0xB9, 8, 0, 0, 0, 0xF3, 0xAB]); // or eax,-1; mov ecx,8; rep stosd
    v.extend([0x48, 0x8D, 0x7A, OLD_SPRITES as u8, 0x31, 0xC0, 0xB9]); // lea rdi,[rdx+0x20]; xor eax,eax; mov ecx,
    v.extend((0x164 - OLD_SPRITES).to_le_bytes()); // KH2's slots and flags, up to the per-level ints
    v.extend([0xF3, 0xAA, 0x48, 0x8D, 0xBA]); // rep stosb; lea rdi,[rdx+
    v.extend(HUD_SPRITES.to_le_bytes());
    v.push(0xB9); // mov ecx,
    v.extend((HUD_SIZE - HUD_SPRITES).to_le_bytes());
    v.extend([0xF3, 0xAA, 0x5F, 0xC3]); // rep stosb; pop rdi; ret
    v.resize(HUD_CLEAR_OLD.len(), 0xCC);
    v
}

/// In-place changes to the HUD's code: (RVA, KH2's bytes, ours).
pub fn hud_patches() -> Vec<(u32, &'static [u8], Vec<u8>)> {
    let l = HUD_LEVEL as u8;
    let base = (HUD_SPRITES / 8) as i32; // slot index of level 0 slot 0, from the object
    let flags = |old: &'static [u8], at: usize| {
        let mut n = old.to_vec();
        n[at..at + 4].copy_from_slice(&HUD_FLAGS.to_le_bytes());
        n
    };
    let mut out: Vec<(u32, &'static [u8], Vec<u8>)> = vec![
        // the object's size (`0x14015E410`: mov edx,0x670)
        (0x15e46f, &[0xBA, 0x70, 0x06, 0, 0], { let mut n = vec![0xBA]; n.extend(HUD_SIZE.to_le_bytes()); n }),
        (0x15c5d0, HUD_CLEAR_OLD, hud_clear()),
        // level x 9 -> level x 16 (imul r, r, 16)
        (0x15ae28, &[0x48, 0x8D, 0x0C, 0xC9], vec![0x48, 0x6B, 0xC9, l]),
        (0x15ae70, &[0x48, 0x8D, 0x0C, 0xC9], vec![0x48, 0x6B, 0xC9, l]),
        (0x15b8a8, &[0x48, 0x8D, 0x0C, 0xC9], vec![0x48, 0x6B, 0xC9, l]),
        (0x15bac9, &[0x4B, 0x8D, 0x0C, 0xFF], vec![0x49, 0x6B, 0xCF, l]),
        (0x15bb07, &[0x4B, 0x8D, 0x0C, 0xFF], vec![0x49, 0x6B, 0xCF, l]),
        (0x15c3de, &[0x4B, 0x8D, 0x14, 0xC0], vec![0x49, 0x6B, 0xD0, l]),
        (0x15d1a3, &[0x4B, 0x8D, 0x3C, 0xFF], vec![0x49, 0x6B, 0xFF, l]),
        // first slot of a level: (9 level + 4) -> (16 level + HUD_SPRITES / 8)
        (0x15ad47, &[0x4A, 0x8D, 0x1C, 0xD5, 4, 0, 0, 0, 0x49, 0x03, 0xDA], [&[0x49, 0x6B, 0xDA, l][..], &x86::add(x86::RBX, base)].concat()),
        (0x15aef6, &[0x48, 0x8D, 0x1C, 0xC5, 4, 0, 0, 0, 0x48, 0x03, 0xD8], [&[0x48, 0x6B, 0xD8, l][..], &x86::add(x86::RBX, base)].concat()),
        // slots a level
        (0x15ad70, &[0xBF, 9, 0, 0, 0], vec![0xBF, l, 0, 0, 0]),
        (0x15af10, &[0xBF, 9, 0, 0, 0], vec![0xBF, l, 0, 0, 0]),
        (0x15eab0, &[0xBB, 9, 0, 0, 0], vec![0xBB, l, 0, 0, 0]),
        // next level's slots in `0x14015E800`: add rcx,0x48 -> sub rcx,-0x80
        (0x15eb10, &[0x48, 0x83, 0xC1, 0x48], vec![0x48, 0x83, 0xE9, (0u8).wrapping_sub(8 * l)]),
    ];
    // the frees of a level's unused slots (`0x14015AF50`, and its copy in `0x14015CBE0`)
    for (cmp, body, step) in [(0x15af86u32, 0x15af8bu32, 0x15afbfu32), (0x15cc25, 0x15cc2a, 0x15cc5f)] {
        out.push((cmp, &[0x83, 0xFA, 9], vec![0x83, 0xFA, l]));
        // lea rbx,[rdx+4]; mov edi,9; add rbx,r14; sub edi,edx; lea rbx,[r15+rbx*8]
        let mut n = vec![0x4A, 0x8D, 0x9C, 0x32]; // lea rbx,[rdx+r14+disp32]
        n.extend(base.to_le_bytes());
        n.extend([0x6A, l, 0x5F]); // push l; pop rdi
        n.extend([0x2B, 0xFA, 0x49, 0x8D, 0x1C, 0xDF, 0x90]); // sub edi,edx; lea rbx,[r15+rbx*8]; nop
        out.push((body, &[0x48, 0x8D, 0x5A, 0x04, 0xBF, 9, 0, 0, 0, 0x49, 0x03, 0xDE, 0x2B, 0xFA, 0x49, 0x8D, 0x1C, 0xDF], n));
        out.push((step, &[0x49, 0x83, 0xC6, 9], vec![0x49, 0x83, 0xC6, l]));
    }
    // the flag bytes
    for (rva, old) in [
        (0x15ae37u32, &[0x80, 0xBC, 0x01, 0x40, 0x01, 0, 0, 0][..]),
        (0x15ae81, &[0x80, 0xBC, 0x01, 0x40, 0x01, 0, 0, 0]),
        (0x15b8ba, &[0x80, 0xBC, 0x01, 0x40, 0x01, 0, 0, 0]),
        (0x15badb, &[0x80, 0xBC, 0x01, 0x40, 0x01, 0, 0, 0]),
        (0x15bb0e, &[0x80, 0xBC, 0x01, 0x40, 0x01, 0, 0, 0]),
        (0x15c3e5, &[0x80, 0xBC, 0x02, 0x40, 0x01, 0, 0, 0]),
    ] {
        out.push((rva, old, flags(old, 3)));
    }
    out.push((0x15d433, &[0x42, 0x88, 0xAC, 0x37, 0x40, 0x01, 0, 0], flags(&[0x42, 0x88, 0xAC, 0x37, 0x40, 0x01, 0, 0], 4)));
    out
}


// ---- KH2's magic elements: 6 -> 9 (step 4) ----------------------------------------------------

/// KH2 keeps one entry of `0x50` per magic element at `0x142A23A80` (6: Fire, Blizzard, Thunder,
/// Cure, Magnet, Reflect): `+0x00` the loaded spell file, `+0x08` / `+0x10` its parts, `+0x18` the
/// effect set (ctor `0x1402C4050`, dtor `0x1402C40C0`), `+0x48` the MAGC row of the level Sora has.
/// They move into our memory with room for 9: Gravity (6), Stop (7) and Aero (8) are KH2 magic
/// elements like the others. Checked (decomp + exe): every reference to the array (12 by address,
/// 2 by the image base), its end (the static init's dtor, `0x1403C5DC0`), and every count of 6.
pub const ELEMENTS: usize = 9;
pub const ELEMENT: usize = 0x50;
pub const GRAVITY: u8 = 6;
pub const STOP: u8 = 7;
pub const AERO: u8 = 8;
/// KH1's order in the Magic menus (the command menu and the pause menu's Magic page).
pub const ORDER: [u8; ELEMENTS] = [0, 1, 2, 3, GRAVITY, STOP, AERO, 4, 5];
/// The magic item of each element (KH2's `0x1405C5D18` has the first 6; the item's count is the
/// spell's level). 84 to 86 are unused item ids in KH2's `item` table (Checked (files)).
pub const ITEMS: [u16; ELEMENTS] = [21, 22, 23, 24, 87, 88, 84, 85, 86];
/// Where they are in our memory.
pub const MAGIC_ELEMS: usize = 0x1800;
pub const MAGIC_ITEMS: usize = MAGIC_ELEMS + ELEMENTS * ELEMENT;
pub const MAGIC_ORDER: usize = MAGIC_ITEMS + 0x20;
/// The command menu's Magic list (KH2's `+0xB68` has room for 6 and the 0).
pub const MAGIC_LIST: usize = MAGIC_ORDER + 0x10;
const OLD_ELEMS: u32 = 0x2A2_3A80;
const OLD_ELEMENTS: u32 = 6;
const OLD_ITEMS: u32 = 0x5C_5D18;
const OLD_LIST: u32 = 0xB68;
const _: () = assert!(MAGIC_LIST + 2 * (ELEMENTS + 1) <= MEM && MAGIC_ELEMS >= CAVES + 0x600);

/// `lea r, [rip + d]` to the array or its item table: (RVA, KH2's bytes).
const MAGIC_RIP_SITES: &[(u32, &[u8])] = &[
    (0x256a4, &[72, 141, 29, 29, 228, 159, 2]),   // static init: lea rbx,[+0x48]
    (0x57760f, &[72, 141, 29, 98, 198, 74, 2]),   // its dtor: lea rbx,[end of the +0x18 sets]
    (0x3c519d, &[72, 141, 61, 220, 232, 101, 2]), // the cast 0x1403C5170
    (0x3c555f, &[72, 141, 29, 50, 229, 101, 2]),
    (0x3c5875, &[72, 141, 61, 28, 226, 101, 2]),
    (0x3c58b1, &[72, 141, 29, 200, 225, 101, 2]),
    (0x3c590a, &[72, 141, 5, 183, 225, 101, 2]),
    (0x3c592a, &[72, 141, 5, 151, 225, 101, 2]),
    (0x3c5ace, &[76, 141, 53, 171, 223, 101, 2]),
    (0x3c5ad5, &[76, 141, 37, 188, 223, 101, 2]),
    (0x3c5ddb, &[72, 141, 53, 230, 220, 101, 2]),
    (0x3c5de4, &[72, 141, 45, 189, 222, 101, 2]), // 0x1403C5DC0's end (+0x48 of a 7th)
    (0x3c4843, &[72, 141, 13, 206, 20, 32, 0]),   // element -> item (0x1405C5D18)
];

/// `[image + index + 0x2A23AC8]`: (RVA, KH2's bytes); the displacement is at 4.
const MAGIC_IMAGE_SITES: &[(u32, &[u8])] = &[(0x3c5966, &[72, 139, 132, 214, 200, 58, 162, 2]), (0x3c5c6d, &[72, 139, 132, 215, 200, 58, 162, 2])];

/// Counts of 6 elements: (RVA, KH2's bytes); the 6 is the last byte, or byte 1 of a `mov r32`.
const MAGIC_COUNTS: &[(u32, &[u8])] = &[
    (0x2569f, &[191, 6, 0, 0, 0]),
    (0x57760a, &[191, 6, 0, 0, 0]),
    (0x3c51c9, &[65, 131, 249, 6]),
    (0x3c5566, &[190, 6, 0, 0, 0]),
    (0x3c587c, &[189, 6, 0, 0, 0]),
    (0x3c5b9a, &[65, 131, 255, 6]),
    (0x3c5dab, &[131, 255, 6]),
    (0x3f283c, &[131, 251, 6]),   // the command menu's Magic list
    (0x3f2956, &[131, 251, 6]),   // its second builder `0x1403F2910` (t96)
    (0x35b803, &[131, 251, 6]),   // the pause menu's Magic page (0x14035B440)
    (0x35b843, &[72, 131, 248, 6]),
    (0x35b8af, &[131, 254, 6]),
];

/// The Magic list at `object + disp32`: (RVA, KH2's bytes, where the displacement is).
const MAGIC_LIST_SITES: &[(u32, &[u8], usize)] = &[
    (0x3f281c, &[72, 141, 189, 104, 11, 0, 0], 3),
    (0x3f2852, &[102, 68, 137, 180, 69, 104, 11, 0, 0], 5),
    (0x3f2b8f, &[72, 141, 151, 104, 11, 0, 0], 3),
    // t96 (the owner: "I can't open the magic submenu because it's a ?"): the root's Magic row is
    // KH2's Magic (cmd 2) only while the list's first entry is non-zero, else cmd 0x68 ("?",
    // message 0x503): `cmp word [rdi+0xB68], 0` in `0x1403F29E0` (Checked (decomp)), and the same
    // test in `0x1403F26F0` / `0x1403F36C0`. t79 moved the list but left these reading KH2's old
    // one. Also KH2's second list builder `0x1403F2910` and `0x1403F3460`'s hand-over (no caller
    // found; moved for safety, as their count).
    (0x3f2e34, &[102, 131, 191, 104, 11, 0, 0, 0], 3),
    (0x3f27c0, &[102, 131, 185, 104, 11, 0, 0, 0], 3),
    (0x3f3774, &[102, 131, 191, 104, 11, 0, 0, 0], 3),
    (0x3f292d, &[72, 141, 185, 104, 11, 0, 0], 3),
    (0x3f296d, &[102, 68, 137, 180, 69, 104, 11, 0, 0], 5),
    (0x3f3479, &[72, 141, 151, 104, 11, 0, 0], 3),
];

fn magic_patches(base: u64, mem: u64) -> Result<Vec<Patch>, String> {
    let rel = |at: u64, len: u64, to: u64| i32::try_from(to as i64 - (at + len) as i64).map_err(|_| format!("{at:#x}: our memory is more than 2 GB away"));
    let elem = |old_rva: u64| -> Option<u64> {
        let off = old_rva.checked_sub(OLD_ELEMS as u64)?;
        let off = if off >= (OLD_ELEMENTS as u64) * ELEMENT as u64 { off + (ELEMENTS as u64 - OLD_ELEMENTS as u64) * ELEMENT as u64 } else { off };
        (off <= ((ELEMENTS + 1) * ELEMENT) as u64).then(|| mem + MAGIC_ELEMS as u64 + off)
    };
    let mut out = Vec::new();
    for &(rva, bytes) in MAGIC_RIP_SITES {
        let at = base + rva as u64;
        let old = (at as i64 + 7 + i32::from_le_bytes(bytes[3..7].try_into().unwrap()) as i64) as u64 - base;
        let to = if old == OLD_ITEMS as u64 { mem + MAGIC_ITEMS as u64 } else { elem(old).ok_or(format!("{rva:#x}: {old:#x} not the magic array"))? };
        let mut new = bytes.to_vec();
        new[3..7].copy_from_slice(&rel(at, 7, to)?.to_le_bytes());
        out.push(Patch { rva, expect: bytes, new });
    }
    for &(rva, bytes) in MAGIC_IMAGE_SITES {
        let old = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as u64;
        let to = elem(old).ok_or("image site")?;
        let mut new = bytes.to_vec();
        new[4..8].copy_from_slice(&i32::try_from(to as i64 - base as i64).map_err(|_| "image site: too far".to_string())?.to_le_bytes());
        out.push(Patch { rva, expect: bytes, new });
    }
    for &(rva, bytes) in MAGIC_COUNTS {
        let mut new = bytes.to_vec();
        let i = if bytes.len() == 5 { 1 } else { bytes.len() - 1 };
        if new[i] != OLD_ELEMENTS as u8 {
            return Err(format!("{rva:#x}: not a count of 6"));
        }
        new[i] = ELEMENTS as u8;
        out.push(Patch { rva, expect: bytes, new });
    }
    let pc = base + PC_RVA as u64;
    for &(rva, bytes, at) in MAGIC_LIST_SITES {
        let mut new = bytes.to_vec();
        if u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) != OLD_LIST {
            return Err(format!("{rva:#x}: not the Magic list"));
        }
        new[at..at + 4].copy_from_slice(&i32::try_from(mem as i64 + MAGIC_LIST as i64 - pc as i64).map_err(|_| "list: too far".to_string())?.to_le_bytes());
        out.push(Patch { rva, expect: bytes, new });
    }
    Ok(out)
}

/// Our magic data, written into our memory once (`mem` = all `MEM` bytes).
pub fn magic_data(mem: &mut [u8]) {
    mem[MAGIC_ELEMS..MAGIC_ELEMS + ELEMENTS * ELEMENT].fill(0);
    for (i, item) in ITEMS.iter().enumerate() {
        mem[MAGIC_ITEMS + 2 * i..MAGIC_ITEMS + 2 * i + 2].copy_from_slice(&item.to_le_bytes());
    }
    mem[MAGIC_ORDER..MAGIC_ORDER + ELEMENTS].copy_from_slice(&ORDER);
    mem[MAGIC_LIST..MAGIC_LIST + 2 * (ELEMENTS + 1)].fill(0);
}

/// KH2's ctor of an element's effect set (for the elements when KH2's static init already ran).
pub const ELEMENT_SET_CTOR_RVA: u32 = 0x2C_4050;

/// `object + D0` is submenu 0's header in the block.
fn d0(pc: u64, mem: u64) -> Result<i32, String> {
    i32::try_from(header(mem, 0) as i64 - pc as i64).map_err(|_| format!("our memory {mem:#x} is more than 2 GB from the menu {pc:#x}"))
}

/// Submenu `n`'s header in our memory at `mem`.
pub fn header(mem: u64, n: usize) -> u64 {
    mem + (n * SLOT + HEADER) as u64
}

/// A small x86-64 encoder: only what the stubs need.
pub mod x86 {
    pub const RAX: u8 = 0;
    pub const RCX: u8 = 1;
    pub const RDX: u8 = 2;
    pub const RBX: u8 = 3;
    pub const RBP: u8 = 5;
    pub const RDI: u8 = 7;
    fn rex(reg: u8, index: u8, base: u8) -> u8 {
        0x48 | ((reg >> 3) << 2) | ((index >> 3) << 1) | (base >> 3)
    }
    /// `lea reg, [base + disp32]` (base not rsp / r12).
    pub fn lea(reg: u8, base: u8, disp: i32) -> Vec<u8> {
        assert!(base & 7 != 4);
        let mut v = vec![rex(reg, 0, base), 0x8D, 0x80 | ((reg & 7) << 3) | (base & 7)];
        v.extend_from_slice(&disp.to_le_bytes());
        v
    }
    /// `lea reg, [base + index + disp32]` (index not rsp).
    pub fn lea_bi(reg: u8, base: u8, index: u8, disp: i32) -> Vec<u8> {
        assert!(index & 7 != 4 || index == 12);
        let mut v = vec![rex(reg, index, base), 0x8D, 0x84 | ((reg & 7) << 3), ((index & 7) << 3) | (base & 7)];
        v.extend_from_slice(&disp.to_le_bytes());
        v
    }
    /// `imul reg, src, imm32`.
    pub fn imul(reg: u8, src: u8, imm: i32) -> Vec<u8> {
        let mut v = vec![rex(reg, 0, src), 0x69, 0xC0 | ((reg & 7) << 3) | (src & 7)];
        v.extend_from_slice(&imm.to_le_bytes());
        v
    }
    /// `add reg, imm32`.
    pub fn add(reg: u8, imm: i32) -> Vec<u8> {
        let mut v = vec![rex(0, 0, reg), 0x81, 0xC0 | (reg & 7)];
        v.extend_from_slice(&imm.to_le_bytes());
        v
    }
    /// `jmp` / `call rel32` at `at` to `to`.
    pub fn rel(op: u8, at: u64, to: u64) -> Result<Vec<u8>, String> {
        let d = i32::try_from(to as i64 - (at as i64 + 5)).map_err(|_| format!("{at:#x} -> {to:#x}: more than 2 GB"))?;
        let mut v = vec![op];
        v.extend_from_slice(&d.to_le_bytes());
        Ok(v)
    }
    pub const JMP: u8 = 0xE9;
    pub const CALL: u8 = 0xE8;
}

/// What a stub does in place of KH2's bytes at a short site (then it jumps back to the
/// instruction after them). `at` is where the stub starts; `base` is KH2's image.
fn stub(rva: u32, base: u64, at: u64, d0: i32, mem: u64) -> Result<Vec<u8>, String> {
    use x86::*;
    let slot = SLOT as i32;
    let mut v: Vec<u8> = Vec::new();
    let site = |r: u32| base + r as u64;
    match rva {
        // lea rcx,[rbp+sub0]; call 0x1403B1B70 (reset the cursor)
        0x3f27f2 => {
            v.extend(lea(RCX, RBP, d0));
            v.extend(rel(CALL, at + v.len() as u64, site(0x3b1b70))?);
        }
        // the loop over the 19 submenus (`0x1403F28C0`): start, and step
        0x3f28d2 => {
            v.extend(lea(RBX, RCX, d0));
            v.extend([0xBF, 0x13, 0, 0, 0]);
        }
        0x3f28e8 => {
            v.extend(add(RBX, slot));
            v.extend([0x48, 0x83, 0xEF, 0x01]);
        }
        // submenu 0 in `0x1403F29E0`, with the instruction after it
        0x3f2dab => {
            v.extend(lea(RCX, RDI, d0));
            v.extend([0x0F, 0xB6, 0xD8]);
        }
        0x3f2dcb => {
            v.extend(lea(RCX, RDI, d0));
            v.extend([0x44, 0x0F, 0xB6, 0x48, 0x04]);
        }
        0x3f2ded => {
            v.extend(lea(RDX, RDI, d0));
            v.extend([0x48, 0x8B, 0xCF]);
        }
        0x3f2e4c | 0x3f2e8c => {
            v.extend(lea(RCX, RDI, d0));
            v.extend([0x45, 0x33, 0xC0]);
        }
        0x3f2e62 => {
            v.extend(lea(RCX, RDI, d0));
            v.extend([0xBA, 0x03, 0, 0, 0]);
        }
        0x3f2eab => {
            v.extend(lea(RCX, RDI, d0));
            v.extend([0x41, 0xB9, 0x01, 0, 0, 0]);
        }
        // submenu r8 of the object rcx (`0x1403F3AF0`)
        0x3f3b0a => {
            v.extend([0x48, 0x8B, 0x10, 0x49, 0x8B, 0xC0]); // mov rdx,[rax]; mov rax,r8
            v.extend(imul(RAX, RAX, slot));
            v.extend(lea_bi(RCX, RCX, RAX, d0));
        }
        // submenu rcx of the object rbx (`0x1403F2470`)
        0x3f259d => {
            v.extend(imul(RCX, RCX, slot));
            v.extend([0x48, 0x8B, 0x10]); // mov rdx,[rax]
            v.extend(lea_bi(RCX, RCX, RBX, d0));
        }
        // push submenu rax of the object rcx (`0x1403B1F30`, `0x1403B2800`)
        0x3b1f7f => {
            v.extend(imul(RAX, RAX, slot));
            v.extend([0x4C, 0x89, 0x81, 0xA8, 0x09, 0, 0]); // mov [rcx+0x9a8],r8
            v.extend(lea_bi(RAX, RAX, RCX, d0));
        }
        0x3b281c => {
            v.extend(imul(RAX, RAX, slot));
            v.extend([0x4C, 0x89, 0x89, 0xA8, 0x09, 0, 0]); // mov [rcx+0x9a8],r9
            v.extend(lea_bi(RAX, RAX, RCX, d0));
        }
        // submenu rax of a submenu's owner, less 0x28 (the code reads +0x38 count, +0x41 rows)
        0x3b1002 | 0x3b193a | 0x3b1acd => {
            v.extend(imul(RAX, RAX, slot));
            v.extend([0x48, 0x03, 0x43, 0x08]); // add rax,[rbx+8]
            v.extend(add(RAX, d0 - OLD_FIRST as i32));
        }
        // element = ORDER[i] (`rbx` or `rsi` the counter), then KH2's call
        0x3f2826 | 0x35b7f4 | 0x35b88f => {
            let order = mem + MAGIC_ORDER as u64;
            v.extend([0x48, 0x8D, 0x0D]); // lea rcx,[rip+order]
            v.extend(i32::try_from(order as i64 - (at as i64 + 7)).map_err(|_| "order: too far".to_string())?.to_le_bytes());
            v.extend([0x0F, 0xB6, 0x0C, if rva == 0x35b88f { 0x31 } else { 0x19 }]); // movzx ecx,byte [rcx+rbx/rsi]
            let callee = match rva {
                0x3f2826 => 0x3c5900,
                0x35b7f4 => 0x3c4840,
                _ => 0x3c5940,
            };
            v.extend(rel(CALL, at + v.len() as u64, site(callee))?);
        }
        0x15d1b2 => {
            v.extend([0x48, 0x8B, 0xB4, 0xF9]); // mov rsi,[rcx+rdi*8+disp32]
            v.extend(HUD_SPRITES.to_le_bytes());
            v.extend([0x49, 0x8B, 0x8C, 0x24, 0x98, 0x03, 0, 0]); // mov rcx,[r12+0x398]
        }
        0x15d302 => {
            v.extend([0x49, 0x89, 0xB4, 0xFE]); // mov [r14+rdi*8+disp32],rsi
            v.extend(HUD_SPRITES.to_le_bytes());
            v.extend(rel(JMP, at + v.len() as u64, site(0x15d34d))?);
        }
        0x15ea5f => {
            v.extend([0x49, 0x8D, 0x8E]); // lea rcx,[r14+disp32]
            v.extend(HUD_SPRITES.to_le_bytes());
            v.extend([0x4C, 0x89, 0x6C, 0x24, 0x50]); // mov [rsp+0x50],r13
        }
        _ => return Err(format!("no stub for {rva:#x}")),
    }
    Ok(v)
}

/// One write into KH2's code: `new` over `expect` at `rva`.
#[derive(Debug, Clone)]
pub struct Patch {
    pub rva: u32,
    pub expect: &'static [u8],
    pub new: Vec<u8>,
}

/// Everything to write: KH2's code, and the stubs (for our memory at `mem + CAVES`).
#[derive(Debug, Clone)]
pub struct Plan {
    pub patches: Vec<Patch>,
    pub stubs: Vec<u8>,
}

/// The writes for KH2 at `base` with our memory at `mem`.
pub fn plan(base: u64, mem: u64) -> Result<Plan, String> {
    let pc = base + PC_RVA as u64;
    let d0 = d0(pc, mem)?;
    let mut patches = Vec::new();
    for &(rva, bytes, at) in CURSOR_SITES {
        let mut new = bytes.to_vec();
        new[at] = match bytes[at] as usize {
            OLD_CURSOR_ROW => CURSOR_ROW as u8,
            OLD_CURSOR_CMD => CURSOR_CMD as u8,
            b => return Err(format!("{rva:#x}: displacement {b:#x}")),
        };
        patches.push(Patch { rva, expect: bytes, new });
    }
    for &(rva, bytes, at) in NUMBER_SITES {
        let old = i32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as i64 - OLD_FIRST as i64;
        let (n, field) = (old / OLD_STRIDE as i64, old % OLD_STRIDE as i64);
        if old < 0 || n >= SUBMENUS as i64 || field >= OLD_CURSOR_ROW as i64 {
            return Err(format!("{rva:#x}: {old:#x} is not a submenu's header or row"));
        }
        let disp = d0 as i64 + n * SLOT as i64 + field;
        let mut new = bytes.to_vec();
        new[at..at + 4].copy_from_slice(&i32::try_from(disp).map_err(|_| "displacement".to_string())?.to_le_bytes());
        patches.push(Patch { rva, expect: bytes, new });
    }
    for (rva, expect, new) in hud_patches() {
        patches.push(Patch { rva, expect, new });
    }
    patches.extend(magic_patches(base, mem)?);
    let mut stubs = Vec::new();
    for &(rva, bytes) in SHORT_SITES {
        let at = mem + CAVES as u64 + stubs.len() as u64;
        let mut s = stub(rva, base, at, d0, mem)?;
        let back = base + rva as u64 + bytes.len() as u64;
        s.extend(x86::rel(x86::JMP, at + s.len() as u64, back)?);
        while s.len() % 16 != 0 {
            s.push(0xCC);
        }
        let mut new = x86::rel(x86::JMP, base + rva as u64, at)?;
        new.resize(bytes.len(), 0x90);
        patches.push(Patch { rva, expect: bytes, new });
        stubs.extend(s);
    }
    if CAVES + stubs.len() > MEM {
        return Err(format!("stubs need {:#x} bytes", stubs.len()));
    }
    Ok(Plan { patches, stubs })
}

/// Moves the submenus from KH2's object (`object`, its first `0xC00` bytes, at address `pc`) into
/// the block (`block`, `BLOCK` bytes at address `mem`): header, the 8 rows and the cursor; rows
/// 8 to 11 as KH2's ctor makes a row (`0x1404020E0`: `+4` = `0x500`, the rest 0). Stack entries
/// pointing at an old submenu are pointed at the new one. Runs once, after KH2's ctor built the
/// object and before any site is changed.
pub fn migrate(object: &mut [u8], pc: u64, block: &mut [u8], mem: u64) {
    for n in 0..SUBMENUS {
        let old = OLD_FIRST + n * OLD_STRIDE;
        let h = n * SLOT + HEADER;
        block[h..h + 0x14 + OLD_ROWS * ROW].copy_from_slice(&object[old..old + 0x14 + OLD_ROWS * ROW]);
        for r in OLD_ROWS..ROWS {
            let row = &mut block[h + 0x14 + r * ROW..h + 0x14 + (r + 1) * ROW];
            row.fill(0);
            row[4..6].copy_from_slice(&0x500u16.to_le_bytes());
        }
        let row = u32::from_le_bytes(object[old + OLD_CURSOR_ROW..][..4].try_into().unwrap());
        let cmd = u32::from_le_bytes(object[old + OLD_CURSOR_CMD..][..4].try_into().unwrap());
        let c = (h as isize + CURSOR_ROW as isize) as usize;
        block[c..c + 4].copy_from_slice(&row.to_le_bytes());
        let c = (h as isize + CURSOR_CMD as isize) as usize;
        block[c..c + 4].copy_from_slice(&cmd.to_le_bytes());
    }
    for k in 0..STACK_LEN {
        let e = STACK + 8 * k;
        let v = u64::from_le_bytes(object[e..e + 8].try_into().unwrap());
        let rel = v.wrapping_sub(pc) as usize;
        if v >= pc && rel >= OLD_FIRST && rel < OLD_FIRST + SUBMENUS * OLD_STRIDE && (rel - OLD_FIRST) % OLD_STRIDE == 0 {
            let n = (rel - OLD_FIRST) / OLD_STRIDE;
            object[e..e + 8].copy_from_slice(&header(mem, n).to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const BASE: u64 = 0x1_4000_0000;
    const MEMAT: u64 = 0x1_3000_0000;

    #[test]
    fn layout_keeps_kh2s_alignment_and_reach() {
        let pc = BASE + PC_RVA as u64;
        for n in 0..SUBMENUS {
            assert_eq!(header(MEMAT, n) % 16, (pc + (OLD_FIRST + n * OLD_STRIDE) as u64) % 16);
        }
        assert!(d0(pc, MEMAT).is_ok());
        assert!(d0(pc, 0x7000_0000).is_err());
    }

    #[test]
    fn plan_covers_every_site_once() {
        let p = plan(BASE, MEMAT).unwrap();
        assert_eq!(p.patches.len(), CURSOR_SITES.len() + NUMBER_SITES.len() + SHORT_SITES.len() + hud_patches().len() + 36);
        assert_eq!((CURSOR_SITES.len(), NUMBER_SITES.len(), SHORT_SITES.len(), hud_patches().len()), (58, 32, 23, 28));
        let mut spans: Vec<(u32, u32)> = p.patches.iter().map(|x| (x.rva, x.rva + x.expect.len() as u32)).collect();
        spans.sort();
        for w in spans.windows(2) {
            assert!(w[0].1 <= w[1].0, "{:#x} overlaps {:#x}", w[0].0, w[1].0);
        }
        for x in &p.patches {
            assert_eq!(x.new.len(), x.expect.len());
        }
    }

    #[test]
    fn cursor_bytes_go_before_the_header() {
        let p = plan(BASE, MEMAT).unwrap();
        let x = p.patches.iter().find(|x| x.rva == 0x3b1b70).unwrap();
        // mov dword [rcx+0x74], 0  ->  mov dword [rcx-0x0c], 0
        assert_eq!(x.expect, &[0xC7, 0x41, 0x74, 0, 0, 0, 0]);
        assert_eq!(x.new, vec![0xC7, 0x41, 0xF4, 0, 0, 0, 0]);
    }

    #[test]
    fn number_sites_point_at_the_block() {
        let p = plan(BASE, MEMAT).unwrap();
        let pc = BASE + PC_RVA as u64;
        // lea rcx,[rdi+0xa8] (Magic, submenu 1)
        let x = p.patches.iter().find(|x| x.rva == 0x3f2b83).unwrap();
        let disp = i32::from_le_bytes(x.new[3..7].try_into().unwrap());
        assert_eq!(pc.wrapping_add(disp as i64 as u64), header(MEMAT, 1));
        // mov byte [rdi+rax*4+0x441], 5: submenu 8's row state
        let x = p.patches.iter().find(|x| x.rva == 0x3f2b70).unwrap();
        let disp = i32::from_le_bytes(x.new[3..7].try_into().unwrap());
        assert_eq!(pc.wrapping_add(disp as i64 as u64), header(MEMAT, 8) + 0x19);
    }

    /// A tiny interpreter for the stubs' own instructions: checks each stub gives what KH2's
    /// bytes gave, moved to the block.
    #[test]
    fn stubs_compute_the_blocks_addresses() {
        let p = plan(BASE, MEMAT).unwrap();
        let pc = BASE + PC_RVA as u64;
        let stub_at = |rva: u32| {
            let x = p.patches.iter().find(|x| x.rva == rva).unwrap();
            assert_eq!(x.new[0], 0xE9);
            let to = (BASE + rva as u64 + 5).wrapping_add(i32::from_le_bytes(x.new[1..5].try_into().unwrap()) as i64 as u64);
            (to - MEMAT - CAVES as u64) as usize
        };
        // Owner by number: rax = n -> rax = header(n) - 0x28.
        let s = &p.stubs[stub_at(0x3b1002)..];
        assert_eq!(&s[..3], &[0x48, 0x69, 0xC0]);
        let imul = i32::from_le_bytes(s[3..7].try_into().unwrap()) as u64;
        assert_eq!(&s[7..11], &[0x48, 0x03, 0x43, 0x08]);
        assert_eq!(&s[11..14], &[0x48, 0x81, 0xC0]);
        let add = i32::from_le_bytes(s[14..18].try_into().unwrap()) as i64 as u64;
        for n in 0..SUBMENUS as u64 {
            assert_eq!((n * imul).wrapping_add(pc).wrapping_add(add), header(MEMAT, n as usize) - OLD_FIRST as u64);
        }
        // and jumps back after KH2's 8 bytes
        let back = (MEMAT + CAVES as u64 + stub_at(0x3b1002) as u64 + 23).wrapping_add(i32::from_le_bytes(s[19..23].try_into().unwrap()) as i64 as u64);
        assert_eq!(s[18], 0xE9);
        assert_eq!(back, BASE + 0x3b100a);
        // lea rcx,[rbp+sub0]: the call is re-aimed at 0x1403B1B70
        let o = stub_at(0x3f27f2);
        let s = &p.stubs[o..];
        assert_eq!(&s[..3], &[0x48, 0x8D, 0x8D]);
        assert_eq!(pc.wrapping_add(i32::from_le_bytes(s[3..7].try_into().unwrap()) as i64 as u64), header(MEMAT, 0));
        let call_at = MEMAT + CAVES as u64 + o as u64 + 7;
        assert_eq!((call_at + 5).wrapping_add(i32::from_le_bytes(s[8..12].try_into().unwrap()) as i64 as u64), BASE + 0x3b1b70);
    }

    #[test]
    fn magic_sites_point_at_our_elements() {
        let p = plan(BASE, MEMAT).unwrap();
        let target = |rva: u32| {
            let x = p.patches.iter().find(|x| x.rva == rva).unwrap();
            (BASE + rva as u64 + 7).wrapping_add(i32::from_le_bytes(x.new[3..7].try_into().unwrap()) as i64 as u64)
        };
        let e = MEMAT + MAGIC_ELEMS as u64;
        assert_eq!(target(0x3c519d), e);
        assert_eq!(target(0x256a4), e + 0x48);
        assert_eq!(target(0x57760f), e + 0x18 + 9 * 0x50);
        assert_eq!(target(0x3c5de4), e + 0x48 + 9 * 0x50);
        assert_eq!(target(0x3c4843), MEMAT + MAGIC_ITEMS as u64);
        let x = p.patches.iter().find(|x| x.rva == 0x3c5966).unwrap();
        assert_eq!(BASE.wrapping_add(i32::from_le_bytes(x.new[4..8].try_into().unwrap()) as i64 as u64), e + 0x48);
        let mut m = vec![0u8; MEM];
        magic_data(&mut m);
        assert_eq!(&m[MAGIC_ORDER..MAGIC_ORDER + 9], &[0, 1, 2, 3, 6, 7, 8, 4, 5]);
        assert_eq!(u16::from_le_bytes([m[MAGIC_ITEMS + 12], m[MAGIC_ITEMS + 13]]), 84);
    }

    #[test]
    fn x86_encodings() {
        use x86::*;
        assert_eq!(lea(RCX, RDI, 0x28), vec![0x48, 0x8D, 0x8F, 0x28, 0, 0, 0]);
        assert_eq!(lea(RDX, RDI, 0x28), vec![0x48, 0x8D, 0x97, 0x28, 0, 0, 0]);
        assert_eq!(lea(RBX, RCX, 0x28), vec![0x48, 0x8D, 0x99, 0x28, 0, 0, 0]);
        assert_eq!(lea(RCX, RBP, 0x328), vec![0x48, 0x8D, 0x8D, 0x28, 3, 0, 0]);
        // lea rcx,[rcx+rax+0x28]: 48 8D 8C 01 28 00 00 00
        assert_eq!(lea_bi(RCX, RCX, RAX, 0x28), vec![0x48, 0x8D, 0x8C, 0x01, 0x28, 0, 0, 0]);
        // lea rcx,[rcx+rbx+0x28]: 48 8D 8C 19 ...
        assert_eq!(lea_bi(RCX, RCX, RBX, 0x28), vec![0x48, 0x8D, 0x8C, 0x19, 0x28, 0, 0, 0]);
        // lea rax,[rax+rcx+0x28]: 48 8D 84 08 ...
        assert_eq!(lea_bi(RAX, RAX, RCX, 0x28), vec![0x48, 0x8D, 0x84, 0x08, 0x28, 0, 0, 0]);
        // imul rax,rax,0xc0: 48 69 C0 C0 00 00 00
        assert_eq!(imul(RAX, RAX, 0xC0), vec![0x48, 0x69, 0xC0, 0xC0, 0, 0, 0]);
        assert_eq!(imul(RCX, RCX, 0xC0), vec![0x48, 0x69, 0xC9, 0xC0, 0, 0, 0]);
        // add rbx,0xc0: 48 81 C3 C0 00 00 00
        assert_eq!(add(RBX, 0xC0), vec![0x48, 0x81, 0xC3, 0xC0, 0, 0, 0]);
    }

    #[test]
    fn migrate_moves_headers_rows_cursors_and_the_stack() {
        let pc = BASE + PC_RVA as u64;
        let mut object = vec![0u8; 0xC00];
        for n in 0..SUBMENUS {
            let o = OLD_FIRST + n * OLD_STRIDE;
            object[o] = n as u8; // kind
            object[o + 8..o + 16].copy_from_slice(&pc.to_le_bytes()); // owner
            object[o + 0x10] = 3; // count
            object[o + 0x14 + 2] = 0x40 + n as u8; // row 0's command
            object[o + 0x14 + 7 * ROW + 2] = 0x80 + n as u8; // row 7's command
            object[o + OLD_CURSOR_ROW] = 2;
            object[o + OLD_CURSOR_CMD] = 0x41;
        }
        let sub1 = pc + (OLD_FIRST + OLD_STRIDE) as u64;
        object[STACK..STACK + 8].copy_from_slice(&(pc + OLD_FIRST as u64).to_le_bytes());
        object[STACK + 8..STACK + 16].copy_from_slice(&sub1.to_le_bytes());
        object[STACK + 16..STACK + 24].copy_from_slice(&0x1234u64.to_le_bytes());
        let mut block = vec![0xAAu8; BLOCK];
        migrate(&mut object, pc, &mut block, MEMAT);
        for n in 0..SUBMENUS {
            let h = n * SLOT + HEADER;
            assert_eq!(block[h], n as u8);
            assert_eq!(u64::from_le_bytes(block[h + 8..h + 16].try_into().unwrap()), pc);
            assert_eq!(block[h + 0x10], 3);
            assert_eq!(block[h + 0x14 + 2], 0x40 + n as u8);
            assert_eq!(block[h + 0x14 + 7 * ROW + 2], 0x80 + n as u8);
            assert_eq!(&block[h + 0x14 + 8 * ROW..h + 0x14 + 9 * ROW], &[0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0]);
            assert_eq!(block[h - 0x0C], 2);
            assert_eq!(block[h - 0x08], 0x41);
        }
        assert_eq!(u64::from_le_bytes(object[STACK..STACK + 8].try_into().unwrap()), header(MEMAT, 0));
        assert_eq!(u64::from_le_bytes(object[STACK + 8..STACK + 16].try_into().unwrap()), header(MEMAT, 1));
        assert_eq!(u64::from_le_bytes(object[STACK + 16..STACK + 24].try_into().unwrap()), 0x1234);
    }

    /// `MENU_DUMP=<file>`: writes the plan (for checking the stubs in an emulator).
    #[test]
    fn dump_for_the_emulator() {
        let Ok(path) = std::env::var("MENU_DUMP") else { return };
        let p = plan(BASE, MEMAT).unwrap();
        let mut out = format!("stubs {}\n", p.stubs.iter().map(|b| format!("{b:02x}")).collect::<String>());
        for x in p.patches {
            out += &format!("patch {:x} {}\n", x.rva, x.new.iter().map(|b| format!("{b:02x}")).collect::<String>());
        }
        std::fs::write(path, out).unwrap();
    }

    /// With the Steam exe at hand (`KH2_EXE`), every site's bytes are KH2's.
    #[test]
    fn sites_match_the_steam_exe() {
        let Ok(path) = std::env::var("KH2_EXE") else { return };
        let exe = std::fs::read(path).unwrap();
        let rva_to_off = |rva: u32| -> usize {
            let pe = u32::from_le_bytes(exe[0x3C..0x40].try_into().unwrap()) as usize;
            let n = u16::from_le_bytes(exe[pe + 6..pe + 8].try_into().unwrap()) as usize;
            let opt = u16::from_le_bytes(exe[pe + 20..pe + 22].try_into().unwrap()) as usize;
            let sec = pe + 24 + opt;
            for i in 0..n {
                let s = &exe[sec + 40 * i..sec + 40 * (i + 1)];
                let va = u32::from_le_bytes(s[12..16].try_into().unwrap());
                let size = u32::from_le_bytes(s[8..12].try_into().unwrap());
                let raw = u32::from_le_bytes(s[20..24].try_into().unwrap());
                if rva >= va && rva < va + size {
                    return (rva - va + raw) as usize;
                }
            }
            panic!("{rva:#x}");
        };
        for x in plan(BASE, MEMAT).unwrap().patches {
            let o = rva_to_off(x.rva);
            assert_eq!(&exe[o..o + x.expect.len()], x.expect, "{:#x}", x.rva);
        }
    }
}
