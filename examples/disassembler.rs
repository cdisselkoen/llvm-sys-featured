//! Sample of disassembler library usage.
//!
//! The LLVM disassembler takes bytes and an assumed value for the program
//! counter, emitting instructions as strings.
//!
//! This example takes bytes on stdin and emits text instructions on stdout.

use std::ffi::CStr;
use std::io::{Result as IoResult, Read, Write, stdin, stdout};
use std::ptr;
use llvm_sys_featured::disassembler::{LLVMDisasmContextRef, LLVMCreateDisasm, LLVMDisasmDispose, LLVMDisasmInstruction};
use llvm_sys_featured::target::{LLVM_InitializeAllDisassemblers, LLVM_InitializeAllTargetInfos, LLVM_InitializeAllTargetMCs};

fn main() -> IoResult<()> {
    let disasm = unsafe {
        LLVM_InitializeAllTargetInfos();
        LLVM_InitializeAllTargetMCs();
        LLVM_InitializeAllDisassemblers();
        LLVMCreateDisasm("x86_64\0".as_ptr() as *const i8, ptr::null_mut(), 0, None, None)
    };
    if disasm.is_null() {
        eprintln!("Failed to create disassembler");
        return Ok(());
    }

    let mut data = Vec::<u8>::new();
    stdin().read_to_end(&mut data)?;
    let r = disassemble_bytes(&mut data, disasm, stdout());

    unsafe {
        LLVMDisasmDispose(disasm);
    }

    r
}

const PC_BASE_ADDR: u64 = 0;

fn disassemble_bytes<W: Write>(mut x: &mut [u8], disasm: LLVMDisasmContextRef, mut out: W) -> IoResult<()> {
    let mut pc = PC_BASE_ADDR;

    loop {
        let mut sbuf = [0i8; 128];
        let sz = unsafe {
            LLVMDisasmInstruction(disasm, x.as_mut_ptr(), x.len() as u64, pc, sbuf.as_mut_ptr() as *mut i8, sbuf.len())
        };
        if sz == 0 {
            break;
        }

        let instr_str = unsafe {
            CStr::from_ptr(sbuf.as_ptr())
        };
        write!(out, "{}\n", instr_str.to_string_lossy())?;

        pc += sz as u64;
        x = &mut x[sz..];
    }

    Ok(())
}
