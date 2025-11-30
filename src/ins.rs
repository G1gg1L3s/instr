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
pub struct Mem {
    pub is_base: bool,
    pub is_index: bool,
    pub disp: u32,
    pub scale: u32,
}

#[derive(Debug)]
pub enum Op {
    Addr(Addr),
    Mem(Mem),
}

#[derive(Debug)]
pub enum Instruction {
    CallNear(Addr),
    // TODO: use op
    CallMem(Addr),
    Return(u16),
    Jump(Op),
    JumpConditional(Op, Condition),
    // TODO: condition type
    Loop(Addr),
    PushImm(u32),
    MovImm(u64),
}

#[derive(Debug)]
pub struct BinaryInstruction {
    pub addr: Addr,
    pub instr: Instruction,
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
    let op = match instr.op_kind(0) {
        OpKind::Memory => {
            let is_base = instr.memory_base() != Register::None;
            let is_index = instr.memory_index() != Register::None;
            let disp = instr.memory_displacement32();
            let scale = instr.memory_index_scale();
            Op::Mem(Mem {
                is_base,
                is_index,
                disp,
                scale,
            })
        }
        OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64 => {
            Op::Addr(Addr(instr.near_branch_target().try_into().unwrap()))
        }

        _ => return None,
    };

    match instr.mnemonic() {
        Mnemonic::Loop | Mnemonic::Loope | Mnemonic::Loopne => Some(Instruction::Loop(Addr(
            instr.near_branch_target().try_into().unwrap(),
        ))),
        Mnemonic::Jmp => Some(Instruction::Jump(op)),
        _ => jump_condition(instr.mnemonic()).map(|cond| Instruction::JumpConditional(op, cond)),
    }
}

fn parse_push(instr: &iced_x86::Instruction) -> Option<Instruction> {
    // push imm
    match instr.op_kind(0) {
        OpKind::Immediate8
        | OpKind::Immediate16
        | OpKind::Immediate32
        | OpKind::Immediate8to16
        | OpKind::Immediate8to32 => {
            let imm = instr.immediate32();
            return Some(Instruction::PushImm(imm));
        }

        _ => {}
    }

    None
}

fn parse_mov(instr: &iced_x86::Instruction) -> Option<Instruction> {
    // src must be immediate (any immediate form allowed)
    match instr.op1_kind() {
        OpKind::Immediate8
        | OpKind::Immediate16
        | OpKind::Immediate32
        | OpKind::Immediate64
        | OpKind::Immediate8to16
        | OpKind::Immediate8to32
        | OpKind::Immediate8to64 => {
            let imm = instr.immediate64();
            Some(Instruction::MovImm(imm))
        }
        _ => None,
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
        Mnemonic::Push => parse_push(instr),
        Mnemonic::Mov => parse_mov(instr),
        _ => None,
    }
}

pub struct Decoder<'a> {
    decoder: iced_x86::Decoder<'a>,
}

impl<'a> Decoder<'a> {
    pub fn new(data: &'a [u8], addr: Addr) -> Self {
        let decoder =
            iced_x86::Decoder::with_ip(32, data, addr.0.into(), iced_x86::DecoderOptions::NONE);

        Self { decoder }
    }

    pub fn position(&self) -> usize {
        self.decoder.position()
    }

    pub fn address(&self) -> Addr {
        Addr::from_u64_assert(self.decoder.ip())
    }
}

impl<'a> std::iter::Iterator for Decoder<'a> {
    type Item = BinaryInstruction;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if !self.decoder.can_decode() {
                return None;
            }
            let instr = self.decoder.decode();
            if let Some(i) = parse_instruction(&instr) {
                return Some(BinaryInstruction {
                    addr: Addr::from_u64_assert(instr.ip()),
                    instr: i,
                });
            }
        }
    }
}
