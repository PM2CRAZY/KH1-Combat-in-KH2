//! A small BDX assembler: writes the encoding [`crate::disasm`] reads and [`crate::Vm`] runs, so
//! the build step can make KH2 scripts of its own (KH1's spells as KH2 spells).
//!
//! Instruction word: `op | sub << 4 | ssub << 6`, then 16-bit operands (`docs/magic-model.md`).
//! Code addresses count 16-bit words from byte 16 of the file; branches and calls are relative to
//! the next instruction.

use std::collections::HashMap;

/// Address modes of the push / pop / memcpy instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The thread's stack frame: `sp + imm`.
    Sp = 0,
    /// The script's shared work memory: `wp + imm`.
    Wp = 1,
    /// Through the pointer at `sp + 0`: `*(sp) + imm`.
    SpD = 2,
}

#[derive(Default)]
pub struct Asm {
    code: Vec<u16>,
    labels: HashMap<String, usize>,
    /// (word index of the operand, word index of the next instruction, label).
    fixups: Vec<(usize, usize, String)>,
    /// (word index of a 32-bit immediate, label): a label's absolute pc as an int.
    absolute: Vec<(usize, String)>,
}

impl Asm {
    pub fn new() -> Asm {
        Asm::default()
    }

    fn w(&mut self, v: u16) -> &mut Self {
        self.code.push(v);
        self
    }

    fn op(&mut self, op: u16, sub: u16, ssub: u16) -> &mut Self {
        self.w(op | sub << 4 | ssub << 6)
    }

    pub fn label(&mut self, name: &str) -> &mut Self {
        let at = self.code.len();
        assert!(self.labels.insert(name.to_string(), at).is_none(), "label {name} twice");
        self
    }

    /// `push imm` (an int).
    pub fn push_i(&mut self, v: i32) -> &mut Self {
        self.op(0, 0, 0).w(v as u32 as u16).w((v as u32 >> 16) as u16)
    }

    /// `push.s imm` (a float).
    pub fn push_f(&mut self, v: f32) -> &mut Self {
        let b = v.to_bits();
        self.op(0, 1, 0).w(b as u16).w((b >> 16) as u16)
    }

    /// The pc of `label` as an int (a thread's start, for `trap_magic_start_thread`).
    pub fn push_pc(&mut self, label: &str) -> &mut Self {
        self.op(0, 0, 0);
        self.absolute.push((self.code.len(), label.to_string()));
        self.w(0).w(0)
    }

    /// `push.<mode> imm`: the address.
    pub fn push_a(&mut self, m: Mode, imm: u16) -> &mut Self {
        self.op(0, 2, m as u16).w(imm)
    }

    /// `push.d.<mode> imm`: the word at the address.
    pub fn push_v(&mut self, m: Mode, imm: u16) -> &mut Self {
        self.op(0, 3, m as u16).w(imm)
    }

    /// `pop.<mode> imm`.
    pub fn pop(&mut self, m: Mode, imm: u16) -> &mut Self {
        self.op(1, 0, m as u16).w(imm)
    }

    /// `memcpy.<mode> size, imm`: `size` bytes from the popped address to the mode's.
    pub fn memcpy(&mut self, m: Mode, size: u16, imm: u16) -> &mut Self {
        self.op(2, 0, m as u16).w(size).w(imm)
    }

    /// `push.d.pop imm`: the word at the popped address + imm.
    pub fn push_dpop(&mut self, imm: u16) -> &mut Self {
        self.op(3, 0, 0).w(imm)
    }

    /// `store`: pops a value, then an address, and writes the value there.
    pub fn store(&mut self) -> &mut Self {
        self.w(4)
    }

    /// Integer unary ops by name.
    pub fn un(&mut self, name: &str) -> &mut Self {
        let (sub, ssub) = match name {
            "cvt.s.w" => (1, 1),
            "cvt.w.s" => (0, 0),
            "neg" => (0, 2),
            "not" => (0, 3),
            "abs" => (0, 5),
            "sltz" => (0, 6),
            "slez" => (0, 7),
            "seqz" => (0, 8),
            "snez" => (0, 9),
            "sgez" => (0, 10),
            "sgtz" => (0, 11),
            "neg.s" => (1, 2),
            "abs.s" => (1, 5),
            "sltz.s" => (1, 6),
            "slez.s" => (1, 7),
            "seqz.s" => (1, 8),
            "snez.s" => (1, 9),
            "sgez.s" => (1, 10),
            "sgtz.s" => (1, 11),
            _ => panic!("unary {name}"),
        };
        self.op(5, sub, ssub)
    }

    /// Binary ops by name (`add` … `lor`, `add.s` … `div.s`).
    pub fn bin(&mut self, name: &str) -> &mut Self {
        let ints = ["add", "sub", "mul", "div", "mod", "and", "or", "xor", "sll", "sra", "land", "lor"];
        let floats = ["add.s", "sub.s", "mul.s", "div.s", "mod.s"];
        if let Some(i) = ints.iter().position(|n| *n == name) {
            self.op(6, 0, i as u16)
        } else if let Some(i) = floats.iter().position(|n| *n == name) {
            self.op(6, 1, i as u16)
        } else {
            panic!("binary {name}")
        }
    }

    fn branch(&mut self, ssub: u16, label: &str) -> &mut Self {
        self.op(7, 0, ssub);
        let at = self.code.len();
        self.fixups.push((at, at + 1, label.to_string()));
        self.w(0)
    }

    pub fn b(&mut self, label: &str) -> &mut Self {
        self.branch(0, label)
    }

    /// Branch if the popped value is zero.
    pub fn beqz(&mut self, label: &str) -> &mut Self {
        self.branch(1, label)
    }

    /// Branch if the popped value isn't zero.
    pub fn bnez(&mut self, label: &str) -> &mut Self {
        self.branch(2, label)
    }

    /// `jal frame=<caller's frame in words, header included> -> label`.
    pub fn jal(&mut self, frame: u16, label: &str) -> &mut Self {
        self.op(8, 0, frame);
        let at = self.code.len();
        self.fixups.push((at, at + 1, label.to_string()));
        self.w(0)
    }

    pub fn halt(&mut self) -> &mut Self {
        self.op(9, 0, 0)
    }
    pub fn exit(&mut self) -> &mut Self {
        self.op(9, 0, 1)
    }
    pub fn ret(&mut self) -> &mut Self {
        self.op(9, 0, 2)
    }
    pub fn drop(&mut self) -> &mut Self {
        self.op(9, 0, 3)
    }
    pub fn dup(&mut self) -> &mut Self {
        self.op(9, 0, 5)
    }

    /// `syscall` by the trap's name (`crate::traps`).
    pub fn sys(&mut self, name: &str) -> &mut Self {
        let (t, i, _, _) = crate::traps::TRAPS.iter().find(|r| r.2 == name).unwrap_or_else(|| panic!("trap {name}"));
        self.op(10, 0, *t as u16).w(*i)
    }

    /// Our code appended to another script's (`base`, a whole BDX file) after its last word, its
    /// header kept (so every pc and code address in it still holds) but for the sizes (each the
    /// larger of the two) and the trigger table, which must have as many entries as `base`'s and
    /// now points at `triggers`' labels. `externs` names pcs in `base`'s code that our branches may
    /// reach (its own trigger entries, say).
    pub fn finish_onto(&self, base: &[u8], work: u32, stack: u32, temp: u32, triggers: &[(i32, &str)], externs: &[(&str, u32)]) -> Result<Vec<u8>, crate::BdxError> {
        let p = crate::Program::parse(base)?;
        if p.triggers.len() != triggers.len() || base.len() % 2 != 0 {
            return Err(crate::BdxError::Format(format!("base has {} triggers, ours {}", p.triggers.len(), triggers.len())));
        }
        let start = (base.len() - 16) / 2;
        let pc = |label: &str| -> Result<usize, crate::BdxError> {
            if let Some(i) = self.labels.get(label) {
                return Ok(start + i);
            }
            externs.iter().find(|e| e.0 == label).map(|e| e.1 as usize).ok_or_else(|| crate::BdxError::Format(format!("no label {label}")))
        };
        let mut code = self.code.clone();
        for (at, next, label) in &self.fixups {
            let rel = pc(label)? as i64 - (start + next) as i64;
            code[*at] = i16::try_from(rel).map_err(|_| crate::BdxError::Format("branch out of range".into()))? as u16;
        }
        for (at, label) in &self.absolute {
            let v = pc(label)? as u32;
            code[*at] = v as u16;
            code[*at + 1] = (v >> 16) as u16;
        }
        let mut out = base.to_vec();
        for (k, v) in [work.max(p.work_size), stack.max(p.stack_size), temp.max(p.temp_size)].into_iter().enumerate() {
            out[16 + 4 * k..20 + 4 * k].copy_from_slice(&v.to_le_bytes());
        }
        for (k, (key, label)) in triggers.iter().enumerate() {
            let at = 0x1C + 8 * k;
            out[at..at + 4].copy_from_slice(&key.to_le_bytes());
            out[at + 4..at + 8].copy_from_slice(&(pc(label)? as u32).to_le_bytes());
        }
        for w in code {
            out.extend_from_slice(&w.to_le_bytes());
        }
        Ok(out)
    }

    /// The file: a 16-byte name, the work / stack / temp sizes, the trigger table (key, label),
    /// then the code.
    pub fn finish(&self, name: &str, work: u32, stack: u32, temp: u32, triggers: &[(i32, &str)]) -> Vec<u8> {
        // Words from byte 16: 6 for the three sizes, 4 per trigger and 4 for the end entry.
        let base = 6 + 4 * (triggers.len() + 1);
        let pc = |label: &str| -> usize { base + *self.labels.get(label).unwrap_or_else(|| panic!("no label {label}")) };
        let mut code = self.code.clone();
        for (at, next, label) in &self.fixups {
            let rel = pc(label) as i64 - (base + next) as i64;
            code[*at] = i16::try_from(rel).expect("branch out of range") as u16;
        }
        for (at, label) in &self.absolute {
            let v = pc(label) as u32;
            code[*at] = v as u16;
            code[*at + 1] = (v >> 16) as u16;
        }
        let mut out = Vec::with_capacity(16 + 2 * (base + code.len()));
        let mut n = name.as_bytes().to_vec();
        n.resize(16, 0);
        out.extend_from_slice(&n);
        for v in [work, stack, temp] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for (key, label) in triggers {
            out.extend_from_slice(&key.to_le_bytes());
            out.extend_from_slice(&(pc(label) as u32).to_le_bytes());
        }
        out.extend_from_slice(&[0u8; 8]);
        for w in code {
            out.extend_from_slice(&w.to_le_bytes());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_the_disassembler() {
        let mut a = Asm::new();
        a.label("start").pop(Mode::Sp, 0).push_v(Mode::Sp, 0).push_i(-1).bin("add").push_f(0.5).pop(Mode::Wp, 4);
        a.label("loop").push_v(Mode::Wp, 4).beqz("end").halt().b("loop").label("end").push_pc("start").drop().sys("trap_frametime").drop().ret();
        let bytes = a.finish("test", 16, 512, 512, &[(20, "start")]);
        let p = crate::Program::parse(&bytes).unwrap();
        assert_eq!(p.triggers, vec![(20, 14)]);
        let text = crate::disasm::disassemble(&p);
        assert!(text.contains("   14  pop.sp      0"), "{text}");
        assert!(text.contains("push       -1"), "{text}");
        assert!(text.contains("beqz       -> "), "{text}");
        assert!(text.contains("syscall    0:3  ; trap_frametime"), "{text}");
        // The branch back lands on the loop's first instruction.
        let lp = text.lines().find(|l| l.contains("push.d.wp")).unwrap().trim().split_whitespace().next().unwrap().to_string();
        assert!(text.contains(&format!("b          -> {lp}")), "{text}");
        // Onto another script: its code where it was, ours after it, its entry reachable.
        let mut b = Asm::new();
        b.label("ours").bnez("theirs").ret();
        let both = b.finish_onto(&bytes, 64, 512, 512, &[(20, "ours")], &[("theirs", 14)]).unwrap();
        let q = crate::Program::parse(&both).unwrap();
        assert_eq!(q.work_size, 64);
        let ours = (bytes.len() as u32 - 16) / 2;
        assert_eq!(q.triggers, vec![(20, ours)]);
        assert!(crate::disasm::disassemble(&q).contains("bnez       -> 14"));
    }
}
