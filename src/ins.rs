use iced_x86::{Mnemonic, OpKind, Register};

use crate::addr::Addr;

/// Represents x86 jump conditions and how they are evaluated.
#[derive(Debug)]
pub enum Condition {
    // Equality / Inequality
    /// `Equal (JE/JZ)` - Jump if ZF=1 (zero flag set, result of previous operation was zero)
    Equal,
    /// `NotEqual (JNE/JNZ)` - Jump if ZF=0 (zero flag clear)
    NotEqual,

    // Unsigned comparisons
    /// `Above (JA/JNBE)` - Jump if CF=0 and ZF=0 (unsigned >)
    Above,
    /// `Above or Equal (JAE/JNB/JNC)` - Jump if CF=0 (unsigned >=)
    AboveEqual,
    /// `Below (JB/JNAE/JC)` - Jump if CF=1 (unsigned <)
    Below,
    /// `Below or Equal (JBE/JNA)` - Jump if CF=1 or ZF=1 (unsigned <=)
    BelowEqual,

    // Signed comparisons
    /// `Greater (JG/JNLE)` - Jump if ZF=0 and SF=OF (signed >)
    Greater,
    /// `Greater or Equal (JGE/JNL)` - Jump if SF=OF (signed >=)
    GreaterEqual,
    /// `Less (JL/JNGE)` - Jump if SF≠OF (signed <)
    Less,
    /// `Less or Equal (JLE/JNG)` - Jump if ZF=1 or SF≠OF (signed <=)
    LessEqual,

    // Overflow / sign / parity
    /// `Overflow (JO)` - Jump if OF=1
    Overflow,
    /// `Not Overflow (JNO)` - Jump if OF=0
    NotOverflow,
    /// `Sign (JS)` - Jump if SF=1 (negative)
    Sign,
    /// `Not Sign (JNS)` - Jump if SF=0 (non-negative)
    NotSign,
    /// `Parity Even (JP/JPE)` - Jump if PF=1 (parity even)
    ParityEven,
    /// `Parity Odd (JNP/JPO)` - Jump if PF=0 (parity odd)
    ParityOdd,

    // CX/ECX/RCX loop checks
    /// `CX/ECX/RCX Zero (JCXZ/JECXZ/JRCXZ)` - Jump if the counter register is zero
    CxZero,
}

#[derive(Debug)]
pub enum Instruction {
    CallNear(Addr),
    CallMem(Addr),
    Return(u16),
    JumpNear(Addr),
    JumpMem(Addr),
    JumpConditionalNear(Addr, Condition),
    JumpConditionalMem(Addr, Condition),
    JumpTable(),
    JumpReg(),
    Loop(Addr), // optional: could also store CX type
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

fn jump_condition(mn: Mnemonic) -> Option<Condition> {
    match mn {
        Mnemonic::Je => Some(Condition::Equal),
        Mnemonic::Jne => Some(Condition::NotEqual),
        Mnemonic::Ja => Some(Condition::Above),
        Mnemonic::Jae => Some(Condition::AboveEqual),
        Mnemonic::Jb => Some(Condition::Below),
        Mnemonic::Jbe => Some(Condition::BelowEqual),
        Mnemonic::Jg => Some(Condition::Greater),
        Mnemonic::Jge => Some(Condition::GreaterEqual),
        Mnemonic::Jl => Some(Condition::Less),
        Mnemonic::Jle => Some(Condition::LessEqual),
        Mnemonic::Jo => Some(Condition::Overflow),
        Mnemonic::Jno => Some(Condition::NotOverflow),
        Mnemonic::Js => Some(Condition::Sign),
        Mnemonic::Jns => Some(Condition::NotSign),
        Mnemonic::Jp => Some(Condition::ParityEven),
        Mnemonic::Jnp => Some(Condition::ParityOdd),
        Mnemonic::Jcxz | Mnemonic::Jecxz | Mnemonic::Jrcxz => Some(Condition::CxZero),
        _ => None,
    }
}

fn parse_jump(instr: &iced_x86::Instruction) -> Option<Instruction> {
    let target = match instr.op_kind(0) {
        OpKind::Memory => {
            let base = instr.memory_base();
            let index = instr.memory_index();
            let disp = instr.memory_displacement32();
            if base == Register::None && index == Register::None {
                Some(Addr(disp))
            } else if instr.mnemonic() == Mnemonic::Jmp {
                return Some(Instruction::JumpTable());
            } else {
                None
            }
        }
        OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64 => {
            Some(Addr(instr.near_branch_target().try_into().unwrap()))
        }

        OpKind::Register if instr.mnemonic() == Mnemonic::Jmp => {
            return Some(Instruction::JumpReg());
        }

        _ => None,
    }?;

    match instr.mnemonic() {
        Mnemonic::Jmp => Some(Instruction::JumpNear(target)),
        Mnemonic::Loop | Mnemonic::Loope | Mnemonic::Loopne => Some(Instruction::Loop(target)),
        _ => jump_condition(instr.mnemonic())
            .map(|cond| Instruction::JumpConditionalNear(target, cond)),
    }
}

pub fn parse_instruction(instr: &iced_x86::Instruction) -> Option<Instruction> {
    match instr.mnemonic() {
        Mnemonic::Call => parse_call(instr),
        Mnemonic::Ret => parse_ret(instr),
        Mnemonic::Jmp
        | Mnemonic::Je
        | Mnemonic::Jne
        | Mnemonic::Ja
        | Mnemonic::Jae
        | Mnemonic::Jb
        | Mnemonic::Jbe
        | Mnemonic::Jg
        | Mnemonic::Jge
        | Mnemonic::Jl
        | Mnemonic::Jle
        | Mnemonic::Jo
        | Mnemonic::Jno
        | Mnemonic::Js
        | Mnemonic::Jns
        | Mnemonic::Jp
        | Mnemonic::Jnp
        | Mnemonic::Jcxz
        | Mnemonic::Jecxz
        | Mnemonic::Jrcxz
        | Mnemonic::Loop
        | Mnemonic::Loope
        | Mnemonic::Loopne => parse_jump(instr),
        _ => None,
    }
}
