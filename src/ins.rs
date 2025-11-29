use iced_x86::{Mnemonic, OpKind, Register};

use crate::addr::Addr;

#[derive(Debug)]
pub enum Instruction {
    CallNear(Addr),
    CallMem(Addr),
    Return(u16),
}

fn parse_call(instr: &iced_x86::Instruction) -> Option<Instruction> {
    // Check operand type
    match instr.op_kind(0) {
        OpKind::Memory => {
            // Absolute address: base = None, index = None
            let base = instr.memory_base();
            let index = instr.memory_index();
            let disp = instr.memory_displacement32();

            if base == Register::None && index == Register::None {
                // This is a static memory address call
                Some(Instruction::CallMem(Addr(disp)))
            } else {
                None
            }
        }

        OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64 => {
            Some(Instruction::CallNear(Addr(
                instr.near_branch_target().try_into().unwrap(),
            ))) // already absolute
        }

        _ => None,
    }
}

fn parse_ret(instr: &iced_x86::Instruction) -> Option<Instruction> {
    if instr.op_count() > 0 {
        match instr.op0_kind() {
            OpKind::Immediate16 => Some(Instruction::Return(instr.immediate16())),
            _ => None,
        }
    } else {
        Some(Instruction::Return(0))
    }
}

pub fn parse_instruction(instr: &iced_x86::Instruction) -> Option<Instruction> {
    match instr.mnemonic() {
        Mnemonic::Call => parse_call(instr),
        Mnemonic::Ret => parse_ret(instr),
        _ => None,
    }
}
