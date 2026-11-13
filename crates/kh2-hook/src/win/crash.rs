//! Crash report: when KH2 faults, the log gets the fault's code and address (as exe offsets,
//! Steam addresses `0x140000000 + offset`) and the exe return addresses on the stack, so a crash
//! names the function it happened in. A vectored handler that only reads and logs, then lets
//! KH2's own handling go on; it logs the first few faults only. Nothing runs per frame.

use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

#[link(name = "kernel32")]
extern "system" {
    fn AddVectoredExceptionHandler(first: u32, handler: unsafe extern "system" fn(*mut Pointers) -> i32) -> *mut c_void;
}

#[repr(C)]
struct Record {
    code: u32,
    flags: u32,
    next: *mut Record,
    address: usize,
    params: u32,
    info: [usize; 15],
}

#[repr(C)]
struct Pointers {
    record: *mut Record,
    context: *mut u8,
}

static BASE: AtomicUsize = AtomicUsize::new(0);
static SIZE: AtomicUsize = AtomicUsize::new(0);
static LOGGED: AtomicUsize = AtomicUsize::new(0);
const CONTINUE_SEARCH: i32 = 0;

unsafe extern "system" fn handler(p: *mut Pointers) -> i32 {
    let r = &*(*p).record;
    // Access violations, illegal instructions, divide by zero, stack overflow; not the
    // exceptions programs raise on purpose (C++ throws, debugger messages).
    if !matches!(r.code, 0xC000_0005 | 0xC000_001D | 0xC000_0094 | 0xC000_00FD | 0xC000_0096) {
        return CONTINUE_SEARCH;
    }
    if LOGGED.fetch_add(1, Ordering::Relaxed) >= 3 {
        return CONTINUE_SEARCH;
    }
    let (base, size) = (BASE.load(Ordering::Relaxed), SIZE.load(Ordering::Relaxed));
    let rel = |a: usize| if a >= base && a < base + size { format!("exe+{:#x} (0x{:x})", a - base, 0x1_4000_0000usize + a - base) } else { format!("{a:#x}") };
    let ctx = (*p).context;
    let rsp = std::ptr::read_unaligned(ctx.add(0x98) as *const usize);
    let regs: Vec<String> = [("rax", 0x78), ("rcx", 0x80), ("rdx", 0x88), ("rbx", 0x90), ("rsi", 0xA8), ("rdi", 0xB0), ("r8", 0xB8), ("r9", 0xC0)]
        .iter()
        .map(|(n, o)| format!("{n}={:#x}", std::ptr::read_unaligned(ctx.add(*o) as *const usize)))
        .collect();
    let access = if r.code == 0xC000_0005 && r.params >= 2 { format!(" ({} {:#x})", if r.info[0] == 1 { "writing" } else if r.info[0] == 8 { "running" } else { "reading" }, r.info[1]) } else { String::new() };
    crate::win::log_line_try(&format!("CRASH {:#010x} at {}{access}; {}", r.code, rel(r.address), regs.join(" ")));
    // The first 16 stack words as they are (saved registers and locals of the faulting
    // function: a value its caller passed is often among them).
    let mut raw = Vec::new();
    for i in 0..16 {
        let mut v = 0usize;
        let mut read = 0usize;
        if super::sys::ReadProcessMemory(super::sys::GetCurrentProcess(), (rsp + i * 8) as *const c_void, &mut v as *mut usize as *mut c_void, 8, &mut read) == 0 || read != 8 {
            break;
        }
        raw.push(format!("{v:#x}"));
    }
    crate::win::log_line_try(&format!("CRASH rsp={rsp:#x} words: {}", raw.join(" ")));
    // Exe return addresses on the stack (a code address just after a call).
    let mut found = Vec::new();
    for i in 0..512 {
        let at = rsp + i * 8;
        let mut v = 0usize;
        let mut read = 0usize;
        if super::sys::ReadProcessMemory(super::sys::GetCurrentProcess(), at as *const c_void, &mut v as *mut usize as *mut c_void, 8, &mut read) == 0 || read != 8 {
            break;
        }
        if v > base + 0x1000 && v < base + size {
            found.push(rel(v));
            if found.len() >= 16 {
                break;
            }
        }
    }
    crate::win::log_line_try(&format!("CRASH stack (exe addresses): {}", found.join(", ")));
    CONTINUE_SEARCH
}

/// Installs the handler; `base` is the exe's load address.
pub unsafe fn install(base: usize) {
    // SizeOfImage from the PE header.
    let pe = base + std::ptr::read_unaligned((base + 0x3C) as *const u32) as usize;
    let size = std::ptr::read_unaligned((pe + 0x18 + 0x38) as *const u32) as usize;
    BASE.store(base, Ordering::Relaxed);
    SIZE.store(size, Ordering::Relaxed);
    AddVectoredExceptionHandler(0, handler);
}
