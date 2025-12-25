use iced_x86::{Mnemonic, OpKind};

use crate::addr::Addr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reg {
    Eax,
    Ebx,
    Ecx,
    Edx,
    Esi,
    Edi,
    Ebp,
    Esp,
    Eip,
}
impl Reg {
    fn size(&self) -> Option<Size> {
        Some(Size::U32)
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Flagx: u16 {
        /// Carry flag
        const CARRY =    1 << 0;
        /// Zero flag
        const ZERO =     1 << 1;
        /// Sign flag
        const SIGN =     1 << 2;
        /// Overflow flag
        const OVERFLOW = 1 << 3;
        /// Parity flag
        const PARITY =   1 << 4;

        // X87 C0
        const C0     =   1 << 5;
        // X87 C1
        const C1     =   1 << 6;
        // X87 C2
        const C2     =   1 << 7;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlagxGroup(Flagx);

impl std::fmt::Display for FlagxGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            x if x == Self::ALL => write!(f, "f:*"),
            x if x == Self::NOCARRY => write!(f, "f:!c"),
            x if x == Self::X87_C1 => write!(f, "f:c1"),
            _ => {
                write!(f, "f:")?;
                for flag in self.0 {
                    write!(f, "{:?}", flag)?;
                }
                Ok(())
            }
        }
    }
}

impl FlagxGroup {
    pub const NONE: Self = Self(Flagx::empty());

    pub const ALL: Self = Self(
        Flagx::CARRY
            .union(Flagx::ZERO)
            .union(Flagx::SIGN)
            .union(Flagx::OVERFLOW)
            .union(Flagx::PARITY),
    );

    pub const NOCARRY: Self = Self(Self::ALL.0.difference(Flagx::CARRY));

    pub const X87_C1: Self = Self(Flagx::C1);

    pub fn is_empty(self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Equal,
    NotEqual,
    SignLess,
    UnsignedLess,
    SignedLessEqual,
    UnsignedLessEqual,
    SignedGreaterEqual,
    UnsignedGreaterEqual,
    SignedGreater,
    UnsignedGreater,
    Negative,
    Positive,
    Overflow,
    NoOverflow,
}

impl std::fmt::Display for Condition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let literal = match self {
            Condition::Equal => "==",
            Condition::NotEqual => "!=",
            Condition::SignLess => "s<",
            Condition::UnsignedLess => "u<",
            Condition::SignedLessEqual => "s<=",
            Condition::UnsignedLessEqual => "u<=",
            Condition::SignedGreaterEqual => "s>=",
            Condition::UnsignedGreaterEqual => "u>=",
            Condition::SignedGreater => "s>",
            Condition::UnsignedGreater => "u>",
            Condition::Negative => "-",
            Condition::Positive => "+",
            Condition::Overflow => "overflow",
            Condition::NoOverflow => "!overflow",
        };
        write!(f, "{literal}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    /// Carry flag
    Cf,
    /// Zero flag
    Zf,
    /// Sign flag
    Sf,
    /// Overflow flag
    Of,
}

impl std::fmt::Display for Flag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cf => write!(f, "cf"),
            Self::Zf => write!(f, "zf"),
            Self::Sf => write!(f, "sf"),
            Self::Of => write!(f, "of"),
        }
    }
}

impl std::fmt::Display for Reg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Reg::Eax => write!(f, "eax"),
            Reg::Ebx => write!(f, "ebx"),
            Reg::Ecx => write!(f, "ecx"),
            Reg::Edx => write!(f, "edx"),
            Reg::Esi => write!(f, "esi"),
            Reg::Edi => write!(f, "edi"),
            Reg::Ebp => write!(f, "ebp"),
            Reg::Esp => write!(f, "esp"),
            Reg::Eip => write!(f, "eip"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TempId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Imm {
    U8(u8),
    U16(u16),
    U32(u32),
}
impl Imm {
    fn new_u(val: u8, size: Size) -> Option<Self> {
        match size {
            Size::U8 => Some(Self::U8(val)),
            Size::U16 => Some(Self::U16(val.into())),
            Size::U32 => Some(Self::U32(val.into())),

            Size::U1 | Size::I8 | Size::I16 | Size::I32 | Size::F32 | Size::F64 => None,
        }
    }

    fn size(&self) -> Option<Size> {
        Some(match self {
            Imm::U8(_) => Size::U8,
            Imm::U16(_) => Size::U16,
            Imm::U32(_) => Size::U32,
        })
    }
}

impl std::fmt::Display for Imm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Imm::U8(x) => write!(f, "0x{:02x}", x),
            Imm::U16(x) => write!(f, "0x{:04x}", x),
            Imm::U32(x) => write!(f, "0x{:08x}", x),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    U1,
    U8,
    U16,
    U32,

    I8,
    I16,
    I32,

    F32,
    F64,
}
impl Size {
    fn to_bytes(&self) -> Option<u32> {
        match self {
            Size::U1 => None,

            Size::U8 => Some(1),
            Size::U16 => Some(2),
            Size::U32 => Some(4),

            Size::I8 => Some(1),
            Size::I16 => Some(2),
            Size::I32 => Some(4),

            Size::F32 => Some(4),
            Size::F64 => Some(8),
        }
    }
}

impl std::fmt::Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::U1 => write!(f, "bool"),

            Size::U8 => write!(f, "u8"),
            Size::U16 => write!(f, "u16"),
            Size::U32 => write!(f, "u32"),

            Size::I8 => write!(f, "u8"),
            Size::I16 => write!(f, "u16"),
            Size::I32 => write!(f, "u32"),

            Size::F32 => write!(f, "f32"),
            Size::F64 => write!(f, "f64"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Temp {
    id: TempId,
    size: Size,
}

impl std::fmt::Display for Temp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "t{}", self.id.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    Reg(Reg),
    Imm(Imm),
    Temp(Temp),
    Flag(Flag),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Reg(reg) => write!(f, "{reg}"),
            Value::Imm(x) => write!(f, "{x}"),
            Value::Temp(x) => write!(f, "{x}"),
            Value::Flag(flag) => write!(f, "{flag}"),
        }
    }
}

impl Value {
    pub fn size(&self) -> Option<Size> {
        match self {
            Value::Reg(reg) => reg.size(),
            Value::Imm(imm) => imm.size(),
            Value::Temp(temp) => Some(temp.size),
            Value::Flag(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemSpace {
    /// normal memory
    Default,
    /// thread-local (TEB)
    Fs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Xor,
    Mul,
    BitAnd,
    BitOr,

    ShiftLeft,
    ShifRight,
    ShifArithRight,
}

impl std::fmt::Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Xor => write!(f, "xor"),
            BinOp::BitAnd => write!(f, "&"),
            BinOp::BitOr => write!(f, "|"),
            BinOp::ShiftLeft => write!(f, "<<"),
            BinOp::ShifRight => write!(f, ">>"),
            BinOp::ShifArithRight => write!(f, "a>>"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Instr {
    BinOp {
        op: BinOp,
        dst: Value,
        lhs: Value,
        rhs: Value,
        flags: FlagxGroup,
    },
    Assign {
        dst: Value,
        src: Value,
    },
    Load {
        dst: Value,
        addr: Value,
        space: MemSpace,
    },
    Store {
        addr: Value,
        src: Value,
        space: MemSpace,
    },
    Call {
        target: Value,
    },

    Not {
        dst: Value,
        src: Value,
    },

    ZeroExtend {
        dst: Value,
        src: Value,
    },

    SliceBytes {
        dst: Value,
        src: Value,
        start: u8,
    },

    SetBytes {
        dst: Value,
        base: Value,
        value: Value,
        start: u8,
    },

    Convert {
        dst: Value,
        src: Value,
    },

    X87Push {
        src: Value,
        flags: FlagxGroup,
    },

    X87Pop {
        /// None = just pop, Some = pop + store
        dst: Option<Value>,
        flags: FlagxGroup,
    },
}

impl std::fmt::Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BinOp {
                op,
                dst,
                lhs,
                rhs,
                flags,
            } => {
                if flags.is_empty() {
                    write!(f, "{dst} = {lhs} {op} {rhs}")
                } else {
                    write!(f, "{dst}, {flags} = {lhs} {op} {rhs}")
                }
            }
            Self::Assign { dst, src } => write!(f, "{dst} = {src}"),
            Self::Load { dst, addr, space } => {
                let size = dst.size().unwrap();
                write!(f, "{dst} = load {size} [{addr}]")?;
                if *space == MemSpace::Fs {
                    write!(f, " [fs]")?;
                }
                Ok(())
            }
            Self::Store { addr, src, space } => {
                let size = src.size().unwrap();
                write!(f, "[{addr}] <- store {size} {src}")?;
                if *space == MemSpace::Fs {
                    write!(f, " [fs]")?;
                }
                Ok(())
            }
            Self::Call { target } => {
                write!(f, "call {target}")
            }
            Self::Not { dst, src } => write!(f, "{dst} = not {src}"),
            Self::ZeroExtend { dst, src } => {
                let dst_size = dst.size().unwrap();
                write!(f, "{dst} = {dst_size}({src})")
            }
            Self::SliceBytes { dst, src, start } => {
                let end = u32::from(*start) + dst.size().unwrap().to_bytes().unwrap();

                write!(f, "{dst} = slice {src}[{start}..{end}]")
            }
            Self::SetBytes {
                dst,
                base,
                value,
                start,
            } => {
                let end = u32::from(*start) + value.size().unwrap().to_bytes().unwrap();
                write!(f, "{dst} = slice {base}[{start}..{end}] set {value}")
            }
            Self::Convert { dst, src } => {
                let src_size = src.size().unwrap();
                let dst_size = dst.size().unwrap();
                write!(f, "{dst} = {src_size}to{dst_size} {src}")
            }
            Self::X87Push { src, flags } => {
                if flags.is_empty() {
                    write!(f, "x87.push {src}")
                } else {
                    write!(f, "{flags} = x87.push {src}")
                }
            }
            Self::X87Pop { dst, flags } => {
                match (dst, flags.is_empty()) {
                    (Some(dst), true) => write!(f, "{dst} = ")?,
                    (Some(dst), false) => write!(f, "{dst}, {flags} = ")?,
                    (None, false) => write!(f, "{flags} = ")?,
                    (None, true) => write!(f, "_ = ")?,
                }

                write!(f, "x87.pop")
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AnnotatedInstr {
    pub addr: Addr,
    pub ins: Instr,
}

#[derive(Debug)]
pub struct LowerCtx {
    pub instrs: Vec<AnnotatedInstr>,
    temps: Vec<Temp>,
    addr: Addr,
}

impl std::fmt::Display for LowerCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for chunk in self.instrs.chunk_by(|a, b| a.addr == b.addr) {
            let (head, tail) = chunk.split_first().unwrap();

            writeln!(f, "{}: {}", chunk[0].addr, head.ins)?;

            for ins in tail {
                writeln!(f, "          {}", ins.ins)?;
            }
        }
        Ok(())
    }
}

impl LowerCtx {
    pub fn new() -> Self {
        Self {
            instrs: Vec::new(),
            temps: Vec::new(),
            addr: Addr(0),
        }
    }

    fn new_temp(&mut self, size: Size) -> Value {
        let id = self.temps.len() as u32;
        self.temps.push(Temp {
            id: TempId(id),
            size,
        });
        Value::Temp(Temp {
            id: TempId(id),
            size,
        })
    }

    fn emit(&mut self, ins: Instr) {
        self.instrs.push(AnnotatedInstr {
            ins,
            addr: self.addr,
        });
    }

    fn set_addr(&mut self, addr: Addr) {
        self.addr = addr;
    }
}

fn map_reg(reg: iced_x86::Register) -> Option<Reg> {
    use iced_x86::Register as R;
    match reg {
        R::EAX => Some(Reg::Eax),
        R::EBX => Some(Reg::Ebx),
        R::ECX => Some(Reg::Ecx),
        R::EDX => Some(Reg::Edx),
        R::ESI => Some(Reg::Esi),
        R::EDI => Some(Reg::Edi),
        R::EBP => Some(Reg::Ebp),
        R::ESP => Some(Reg::Esp),
        _ => None,
    }
}

fn mem_space(ins: &iced_x86::Instruction) -> MemSpace {
    match ins.memory_segment() {
        iced_x86::Register::FS => MemSpace::Fs,
        _ => MemSpace::Default,
    }
}

fn map_reg_unwrap(reg: iced_x86::Register, addr: Addr) -> Reg {
    match map_reg(reg) {
        Some(ok) => ok,
        None => panic!("failed to convert: {reg:?} at {addr}"),
    }
}

fn lower_mem_operand(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) -> (MemSpace, Value) {
    let scaled: Option<Value> = if ins.memory_index() == iced_x86::Register::None {
        None
    } else {
        let idx = Value::Reg(map_reg_unwrap(ins.memory_index(), Addr(ins.ip32())));
        let scale = ins.memory_index_scale();

        Some(if scale == 1 {
            idx
        } else {
            let scaled = ctx.new_temp(Size::U32);
            ctx.emit(Instr::BinOp {
                op: BinOp::Mul,
                dst: scaled.clone(),
                lhs: idx,
                rhs: Value::Imm(Imm::U32(scale)),
                flags: FlagxGroup::NONE,
            });
            scaled
        })
    };

    let scaled_and_base = if ins.memory_base() != iced_x86::Register::None {
        let base = Value::Reg(map_reg_unwrap(ins.memory_base(), Addr(ins.ip32())));
        Some(if let Some(scaled) = scaled {
            let scaled_and_base = ctx.new_temp(Size::U32);
            ctx.emit(Instr::BinOp {
                op: BinOp::Add,
                dst: scaled_and_base,
                lhs: scaled,
                rhs: base,
                flags: FlagxGroup::NONE,
            });
            scaled_and_base
        } else {
            base
        })
    } else {
        scaled
    };

    let disp = ins.memory_displacement32();
    let result = if disp != 0 {
        let disp = Value::Imm(Imm::U32(disp));

        Some(if let Some(scaled_and_base) = scaled_and_base {
            let res = ctx.new_temp(Size::U32);
            ctx.emit(Instr::BinOp {
                op: BinOp::Add,
                dst: res,
                lhs: scaled_and_base,
                rhs: disp,
                flags: FlagxGroup::NONE,
            });
            res
        } else {
            disp
        })
    } else {
        scaled_and_base
    };

    let memspace = mem_space(ins);
    if result.is_none() && memspace != MemSpace::Default {
        return (memspace, Value::Imm(Imm::U32(0)));
    }

    let Some(result) = result else {
        dbg!(ins.memory_base());
        dbg!(ins.memory_index());
        dbg!(ins.memory_displacement32());
        panic!("cannot create mov: {ins}");
    };

    (memspace, result)
}

fn memory_size_to_size(memory_size: iced_x86::MemorySize) -> Option<Size> {
    match memory_size {
        iced_x86::MemorySize::UInt8 => Some(Size::U8),
        iced_x86::MemorySize::UInt16 => Some(Size::U16),
        iced_x86::MemorySize::UInt32 => Some(Size::U32),

        iced_x86::MemorySize::Int8 => Some(Size::I8),
        iced_x86::MemorySize::Int16 => Some(Size::I16),
        iced_x86::MemorySize::Int32 => Some(Size::I32),

        iced_x86::MemorySize::Float32 => Some(Size::F32),
        iced_x86::MemorySize::Float64 => Some(Size::F64),

        iced_x86::MemorySize::DwordOffset => Some(Size::U32),
        _ => None,
    }
}

fn lower_memory(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) -> Operand {
    let Some(size) = memory_size_to_size(ins.memory_size()) else {
        panic!(
            "Unknown memory size: {:?} for {} at {}",
            ins.memory_size(),
            ins,
            Addr(ins.ip32())
        )
    };
    let (space, addr) = lower_mem_operand(ctx, ins);
    Operand::Memory { addr, space, size }
}

fn lower_mov(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    lower_bin_operation(ctx, ins, |ctx, ins, lhs, rhs| {
        if ins.mnemonic() == Mnemonic::Movzx {
            let target_size = lhs.size().unwrap();
            let res = ctx.new_temp(target_size);
            ctx.emit(Instr::ZeroExtend { dst: res, src: rhs });
            res
        } else {
            rhs
        }
    });
}

#[derive(Debug, Clone, Copy)]
enum Operand {
    Reg(Reg),
    SubReg {
        reg: Reg,
        lo: u8,
        size: Size,
    },
    Imm(Imm),
    Memory {
        addr: Value,
        space: MemSpace,
        size: Size,
    },
}

impl Operand {
    fn size(&self) -> Option<Size> {
        match self {
            Operand::Reg(reg) => reg.size(),
            Operand::SubReg { size, .. } => Some(*size),
            Operand::Imm(imm) => imm.size(),
            Operand::Memory { size, .. } => Some(*size),
        }
    }

    fn lower_load(self, ctx: &mut LowerCtx) -> Value {
        match self {
            Operand::Reg(reg) => Value::Reg(reg),
            Operand::Imm(imm) => Value::Imm(imm),
            Operand::Memory { addr, space, size } => {
                let tmp = ctx.new_temp(size);

                ctx.emit(Instr::Load {
                    dst: tmp.clone(),
                    addr,
                    space,
                });
                tmp
            }
            Operand::SubReg { reg, lo, size } => {
                let res = ctx.new_temp(size);
                ctx.emit(Instr::SliceBytes {
                    dst: res,
                    src: Value::Reg(reg),
                    start: lo,
                });
                res
            }
        }
    }

    fn lower_store(self, ctx: &mut LowerCtx, value: Value) {
        match self {
            Operand::Reg(reg) => {
                ctx.emit(Instr::Assign {
                    dst: Value::Reg(reg),
                    src: value,
                });
            }
            Operand::Imm(_) => panic!("cannot assign to immediate value"),
            Operand::Memory {
                addr,
                space,
                size: _,
            } => {
                ctx.emit(Instr::Store {
                    addr,
                    src: value,
                    space,
                });
            }
            Operand::SubReg { reg, lo, size } => {
                let value_size = value.size().unwrap();
                assert_eq!(value_size, size);

                let base = Value::Reg(reg);
                let tmp = ctx.new_temp(Size::U32);
                ctx.emit(Instr::SetBytes {
                    dst: tmp,
                    base,
                    value,
                    start: lo,
                });
                ctx.emit(Instr::Assign {
                    dst: base,
                    src: tmp,
                });
            }
        }
    }
}

fn lower_subregister(reg: iced_x86::Register) -> Option<Operand> {
    use iced_x86::Register;

    Some(match reg {
        Register::AL => Operand::SubReg {
            reg: Reg::Eax,
            lo: 0,
            size: Size::U8,
        },
        Register::AH => Operand::SubReg {
            reg: Reg::Eax,
            lo: 8,
            size: Size::U8,
        },
        Register::AX => Operand::SubReg {
            reg: Reg::Eax,
            lo: 8,
            size: Size::U16,
        },

        Register::BL => Operand::SubReg {
            reg: Reg::Ebx,
            lo: 0,
            size: Size::U8,
        },
        Register::BH => Operand::SubReg {
            reg: Reg::Ebx,
            lo: 8,
            size: Size::U8,
        },
        Register::BX => Operand::SubReg {
            reg: Reg::Ebx,
            lo: 8,
            size: Size::U16,
        },

        Register::CL => Operand::SubReg {
            reg: Reg::Ecx,
            lo: 0,
            size: Size::U8,
        },
        Register::CH => Operand::SubReg {
            reg: Reg::Ecx,
            lo: 8,
            size: Size::U8,
        },
        Register::CX => Operand::SubReg {
            reg: Reg::Ecx,
            lo: 8,
            size: Size::U16,
        },

        Register::DL => Operand::SubReg {
            reg: Reg::Edx,
            lo: 0,
            size: Size::U8,
        },
        Register::DH => Operand::SubReg {
            reg: Reg::Edx,
            lo: 8,
            size: Size::U8,
        },
        Register::DX => Operand::SubReg {
            reg: Reg::Edx,
            lo: 8,
            size: Size::U16,
        },

        _ => return None,
    })
}

fn lower_operand(ctx: &mut LowerCtx, ins: &iced_x86::Instruction, operand: u32) -> Operand {
    match ins.op_kind(operand) {
        OpKind::Register => {
            let operand = ins.op_register(operand);
            if let Some(op) = lower_subregister(operand) {
                return op;
            };

            Operand::Reg(map_reg_unwrap(operand, Addr(ins.ip32())))
        }
        OpKind::Immediate8 => Operand::Imm(Imm::U8(ins.immediate8())),
        OpKind::Immediate16 => Operand::Imm(Imm::U16(ins.immediate16())),
        OpKind::Immediate32 => Operand::Imm(Imm::U32(ins.immediate32())),
        OpKind::Immediate8to16 => Operand::Imm(Imm::U16(ins.immediate8to16() as _)),
        OpKind::Immediate8to32 => Operand::Imm(Imm::U32(ins.immediate8to32() as _)),
        OpKind::NearBranch32 => Operand::Imm(Imm::U32(ins.near_branch32())),
        OpKind::Memory => lower_memory(ctx, ins),
        x => panic!(
            "unknown kind: {x:?} in instruction {} at {}",
            ins,
            Addr(ins.ip32())
        ),
    }
}

fn lower_bin_operation(
    ctx: &mut LowerCtx,
    ins: &iced_x86::Instruction,
    bin_lower: impl FnOnce(&mut LowerCtx, &iced_x86::Instruction, Value, Value) -> Value,
) {
    let lhs = lower_operand(ctx, ins, 0);
    let lhs_loaded = lhs.clone().lower_load(ctx);
    let rhs = lower_operand(ctx, ins, 1).lower_load(ctx);

    let result = bin_lower(ctx, ins, lhs_loaded, rhs);
    lhs.lower_store(ctx, result);
}

fn lower_push(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let value = lower_operand(ctx, ins, 0).lower_load(ctx);

    let esp = Value::Reg(Reg::Esp);
    let new_esp = ctx.new_temp(Size::U32);

    ctx.emit(Instr::BinOp {
        op: BinOp::Sub,
        dst: new_esp,
        lhs: esp.clone(),
        rhs: Value::Imm(Imm::U32(value.size().unwrap().to_bytes().unwrap())),
        flags: FlagxGroup::NONE,
    });

    ctx.emit(Instr::Store {
        addr: new_esp,
        src: value,
        space: MemSpace::Default,
    });

    ctx.emit(Instr::Assign {
        dst: esp,
        src: new_esp,
    });
}

fn lower_pop(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let dst = lower_operand(ctx, ins, 0);

    let size = match &dst {
        Operand::Reg(r) => r.size().unwrap(),
        Operand::SubReg { size, .. } => *size,
        Operand::Memory { size, .. } => *size,
        Operand::Imm(_) => unreachable!(),
    };

    let byte_count = size.to_bytes().unwrap();

    let esp = Value::Reg(Reg::Esp);
    let old_esp = ctx.new_temp(Size::U32);
    ctx.emit(Instr::Assign {
        dst: old_esp,
        src: esp.clone(),
    });

    let value = ctx.new_temp(size);
    ctx.emit(Instr::Load {
        dst: value,
        addr: old_esp,
        space: MemSpace::Default,
    });

    dst.lower_store(ctx, value);

    let new_esp = ctx.new_temp(Size::U32);
    ctx.emit(Instr::BinOp {
        op: BinOp::Add,
        dst: new_esp,
        lhs: esp.clone(),
        rhs: Value::Imm(Imm::U32(byte_count)),
        flags: FlagxGroup::NONE,
    });

    ctx.emit(Instr::Assign {
        dst: esp,
        src: new_esp,
    });
}

fn lower_call(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let target = lower_operand(ctx, ins, 0).lower_load(ctx);
    ctx.emit(Instr::Call { target });
}

fn bin_size(lhs: &Value, rhs: &Value) -> Size {
    let lhs = lhs.size().unwrap();
    let rhs = rhs.size().unwrap();
    if lhs == rhs {
        lhs
    } else {
        panic!("Different sizes: {lhs} and {rhs}")
    }
}

fn lower_binary_bit_op(ctx: &mut LowerCtx, ins: &iced_x86::Instruction, op: BinOp) {
    lower_bin_operation(ctx, ins, |ctx, _ins, lhs, rhs| {
        let tmp = ctx.new_temp(bin_size(&lhs, &rhs));
        ctx.emit(Instr::BinOp {
            op,
            dst: tmp,
            lhs,
            rhs,
            flags: FlagxGroup::ALL,
        });
        tmp
    });
}

fn lower_bin_set_flags(ctx: &mut LowerCtx, ins: &iced_x86::Instruction, op: BinOp) {
    lower_bin_operation(ctx, ins, |ctx, _ins, lhs, rhs| {
        let res = ctx.new_temp(bin_size(&lhs, &rhs));
        ctx.emit(Instr::BinOp {
            op,
            dst: res,
            lhs,
            rhs,
            flags: FlagxGroup::ALL,
        });
        res
    });
}

fn lower_shift(ctx: &mut LowerCtx, ins: &iced_x86::Instruction, op: BinOp) {
    let dst_operand = lower_operand(ctx, ins, 0);
    let lhs = dst_operand.lower_load(ctx);
    let rhs = lower_operand(ctx, ins, 1).lower_load(ctx);

    // Mask shift count: x86 masks by 0x1F for 32-bit operands
    let rhs_size = rhs.size().unwrap();
    let masked_count = emit_bin(
        ctx,
        BinOp::BitAnd,
        rhs,
        Value::Imm(Imm::new_u(0x1F, rhs_size).unwrap()),
    );

    let result = emit_bin_with_flags(ctx, op, lhs, masked_count, FlagxGroup::ALL);
    dst_operand.lower_store(ctx, result);
}

fn lower_cmp(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let lhs = lower_operand(ctx, ins, 0).lower_load(ctx);
    let rhs = lower_operand(ctx, ins, 1).lower_load(ctx);

    let tmp = ctx.new_temp(bin_size(&lhs, &rhs));
    ctx.emit(Instr::BinOp {
        op: BinOp::Sub,
        dst: tmp,
        lhs,
        rhs,
        flags: FlagxGroup::ALL,
    });
}

fn lower_test(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let lhs = lower_operand(ctx, ins, 0).lower_load(ctx);
    let rhs = lower_operand(ctx, ins, 1).lower_load(ctx);

    let tmp = ctx.new_temp(bin_size(&lhs, &rhs));
    ctx.emit(Instr::BinOp {
        op: BinOp::BitAnd,
        dst: tmp,
        lhs,
        rhs,
        flags: FlagxGroup::ALL,
    });
}

fn lower_sete_setne(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let lhs = lower_operand(ctx, ins, 0);

    let src = if ins.mnemonic() == Mnemonic::Sete {
        Value::Flag(Flag::Zf)
    } else {
        emit_not(ctx, Value::Flag(Flag::Zf))
    };

    let tmp = ctx.new_temp(Size::U8);
    ctx.emit(Instr::ZeroExtend { dst: tmp, src });
    lhs.lower_store(ctx, tmp);
}

fn lower_fild(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let value = lower_operand(ctx, ins, 0).lower_load(ctx);
    let value = emit_convert(ctx, value, Size::F64);
    ctx.emit(Instr::X87Push {
        src: value,
        flags: FlagxGroup::X87_C1,
    })
}

fn lower_fstp(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let dest = lower_operand(ctx, ins, 0);

    let value = ctx.new_temp(Size::F64);
    ctx.emit(Instr::X87Pop {
        dst: Some(value),
        flags: FlagxGroup::X87_C1,
    });

    let size = dest.size().unwrap();
    let value = emit_convert(ctx, value, size);
    dest.lower_store(ctx, value);
}

#[derive(Debug, Clone)]
pub enum Terminator {
    Cond {
        cond: Condition,
        then_bb: Value,
        else_bb: Value,
    },
    Jump {
        target: Value,
    },
    Ret {
        stack_adjust: u16,
    },
}

impl std::fmt::Display for Terminator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Terminator::Cond {
                cond,
                then_bb,
                else_bb,
            } => write!(f, "if {cond} then {then_bb} else {else_bb}"),
            Self::Jump { target } => write!(f, "jump {target}"),
            Self::Ret { stack_adjust } => write!(f, "ret {stack_adjust}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnnotatedTerminator {
    pub addr: Addr,
    pub inner: Terminator,
}

fn lower_jmp_x(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) -> Terminator {
    let target = lower_operand(ctx, ins, 0).lower_load(ctx);

    let cond = match ins.mnemonic() {
        Mnemonic::Je => Condition::Equal,     // ZF = 1
        Mnemonic::Jne => Condition::NotEqual, // ZF = 0

        Mnemonic::Ja => Condition::UnsignedGreater, // CF = 0 && ZF = 0
        Mnemonic::Jae => Condition::UnsignedGreaterEqual, // CF = 0

        Mnemonic::Jb => Condition::UnsignedLess, // CF = 1
        Mnemonic::Jbe => Condition::UnsignedLessEqual, // CF = 1 || ZF = 1

        Mnemonic::Jg => Condition::SignedGreater, // ZF = 0 && SF == OF
        Mnemonic::Jge => Condition::SignedGreaterEqual, // SF == OF

        Mnemonic::Jl => Condition::SignLess,         // SF != OF
        Mnemonic::Jle => Condition::SignedLessEqual, // ZF = 1 || SF != OF

        Mnemonic::Jo => Condition::Overflow,    // OF = 1
        Mnemonic::Jno => Condition::NoOverflow, // OF = 0

        Mnemonic::Js => Condition::Negative,  // SF = 1
        Mnemonic::Jns => Condition::Positive, // SF = 0

        _ => panic!("unknown ins: {ins}"),
    };

    Terminator::Cond {
        cond,
        then_bb: target,
        else_bb: Value::Imm(Imm::U32(ins.next_ip32())),
    }
}

fn lower_jmp(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) -> Terminator {
    let target = lower_operand(ctx, ins, 0).lower_load(ctx);
    Terminator::Jump { target }
}

fn lower_lea(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let lhs = lower_operand(ctx, ins, 0).lower_load(ctx);
    let (_, addr) = lower_mem_operand(ctx, ins);

    ctx.emit(Instr::Assign {
        dst: lhs,
        src: addr,
    });
}

fn lower_inc_dec(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let operand = lower_operand(ctx, ins, 0);
    let lhs = operand.lower_load(ctx);
    let size = lhs.size().unwrap();
    let imm = Imm::new_u(1, size).unwrap();

    let op = if ins.mnemonic() == Mnemonic::Inc {
        BinOp::Add
    } else {
        BinOp::Sub
    };

    let new = emit_bin_with_flags(ctx, op, lhs, Value::Imm(imm), FlagxGroup::NOCARRY);
    operand.lower_store(ctx, new);
}

fn lower_neg(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let operand = lower_operand(ctx, ins, 0);
    let value = operand.lower_load(ctx);
    let size = value.size().unwrap();

    let lhs = Value::Imm(Imm::new_u(0, size).unwrap());
    let new = emit_bin_with_flags(ctx, BinOp::Sub, lhs, value, FlagxGroup::ALL);

    operand.lower_store(ctx, new);
}

fn lower_not(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let operand = lower_operand(ctx, ins, 0);
    let value = operand.lower_load(ctx);
    let size = value.size().unwrap();

    let res = ctx.new_temp(size);
    ctx.emit(Instr::Not { dst: res, src: res });

    operand.lower_store(ctx, res);
}

fn _lower_enter(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let frame_size = ins.immediate16();
    let nesting = ins.immediate8();

    if nesting != 0 {
        panic!(
            "ENTER with nesting != 0 not supported at {}",
            Addr(ins.ip32())
        );
    }

    // push ebp
    let ebp = Value::Reg(Reg::Ebp);
    let esp = Value::Reg(Reg::Esp);

    let new_esp = ctx.new_temp(Size::U32);
    ctx.emit(Instr::BinOp {
        op: BinOp::Sub,
        dst: new_esp,
        lhs: esp.clone(),
        rhs: Value::Imm(Imm::U32(4)),
        flags: FlagxGroup::NONE,
    });

    ctx.emit(Instr::Store {
        addr: new_esp,
        src: ebp.clone(),
        space: MemSpace::Default,
    });

    ctx.emit(Instr::Assign {
        dst: esp.clone(),
        src: new_esp,
    });

    // mov ebp, esp
    ctx.emit(Instr::Assign {
        dst: ebp.clone(),
        src: esp.clone(),
    });

    // sub esp, imm16
    if frame_size != 0 {
        let esp_after_alloc = ctx.new_temp(Size::U32);
        ctx.emit(Instr::BinOp {
            op: BinOp::Sub,
            dst: esp_after_alloc,
            lhs: esp.clone(),
            rhs: Value::Imm(Imm::U32(frame_size.into())),
            flags: FlagxGroup::NONE,
        });

        ctx.emit(Instr::Assign {
            dst: esp,
            src: esp_after_alloc,
        });
    }
}

fn lower_leave(ctx: &mut LowerCtx, _ins: &iced_x86::Instruction) {
    let esp = Value::Reg(Reg::Esp);
    let ebp = Value::Reg(Reg::Ebp);

    // mov esp, ebp
    ctx.emit(Instr::Assign {
        dst: esp.clone(),
        src: ebp.clone(),
    });

    // pop ebp
    let old_esp = ctx.new_temp(Size::U32);
    ctx.emit(Instr::Assign {
        dst: old_esp,
        src: esp.clone(),
    });

    let new_ebp = ctx.new_temp(Size::U32);
    ctx.emit(Instr::Load {
        dst: new_ebp,
        addr: old_esp,
        space: MemSpace::Default,
    });

    ctx.emit(Instr::Assign {
        dst: ebp,
        src: new_ebp,
    });

    let esp_after_pop = ctx.new_temp(Size::U32);
    ctx.emit(Instr::BinOp {
        op: BinOp::Add,
        dst: esp_after_pop,
        lhs: esp,
        rhs: Value::Imm(Imm::U32(4)),
        flags: FlagxGroup::NONE,
    });

    ctx.emit(Instr::Assign {
        dst: Value::Reg(Reg::Esp),
        src: esp_after_pop,
    });
}

fn lower_flag_as_int(ctx: &mut LowerCtx, flag: Flag, size: Size) -> Value {
    let tmp = ctx.new_temp(size);
    ctx.emit(Instr::ZeroExtend {
        dst: tmp,
        src: Value::Flag(flag),
    });
    tmp
}

fn lower_sbb(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    lower_bin_operation(ctx, ins, |ctx, _ins, lhs, rhs| {
        let size = bin_size(&lhs, &rhs);

        // CF as integer (0 or 1)
        let cf_int = lower_flag_as_int(ctx, Flag::Cf, size);

        // rhs + CF
        let rhs_plus_cf = ctx.new_temp(size);
        ctx.emit(Instr::BinOp {
            op: BinOp::Add,
            dst: rhs_plus_cf,
            lhs: rhs,
            rhs: cf_int,
            flags: FlagxGroup::NONE,
        });

        // lhs - (rhs + CF)
        let res = ctx.new_temp(size);
        ctx.emit(Instr::BinOp {
            op: BinOp::Sub,
            dst: res,
            lhs,
            rhs: rhs_plus_cf,
            flags: FlagxGroup::ALL,
        });

        res
    });
}

fn emit_not(ctx: &mut LowerCtx, src: Value) -> Value {
    let res = ctx.new_temp(Size::U1);
    ctx.emit(Instr::Not { dst: res, src });
    res
}

fn emit_bin(ctx: &mut LowerCtx, op: BinOp, lhs: Value, rhs: Value) -> Value {
    emit_bin_with_flags(ctx, op, lhs, rhs, FlagxGroup::NONE)
}

fn emit_bin_with_flags(
    ctx: &mut LowerCtx,
    op: BinOp,
    lhs: Value,
    rhs: Value,
    flags: FlagxGroup,
) -> Value {
    let res = ctx.new_temp(Size::U1);
    ctx.emit(Instr::BinOp {
        op,
        dst: res,
        lhs,
        rhs,
        flags,
    });
    res
}

fn emit_convert(ctx: &mut LowerCtx, value: Value, size: Size) -> Value {
    let res = ctx.new_temp(size);
    ctx.emit(Instr::Convert {
        dst: res,
        src: value,
    });
    res
}

#[derive(Debug)]
pub struct Block {
    pub addr: Addr,
    pub instr: Vec<AnnotatedInstr>,
    pub terminator: AnnotatedTerminator,
}

impl std::fmt::Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for chunk in self.instr.chunk_by(|a, b| a.addr == b.addr) {
            let (head, tail) = chunk.split_first().unwrap();

            writeln!(f, "{}: {}", chunk[0].addr, head.ins)?;

            for ins in tail {
                writeln!(f, "          {}", ins.ins)?;
            }
        }

        let Some(last) = self.instr.last() else {
            writeln!(f, "{}: {}", self.terminator.addr, self.terminator.inner)?;
            return Ok(());
        };
        if last.addr == self.terminator.addr {
            writeln!(f, "          {}", self.terminator.inner)?;
        } else {
            writeln!(f, "{}: {}", self.terminator.addr, self.terminator.inner)?;
        }

        Ok(())
    }
}

fn lower_ins(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) -> Option<Terminator> {
    ctx.set_addr(Addr(ins.ip32()));
    match ins.mnemonic() {
        Mnemonic::Push => lower_push(ctx, ins),
        Mnemonic::Pop => lower_pop(ctx, ins),
        Mnemonic::Leave => lower_leave(ctx, ins),

        Mnemonic::Call => lower_call(ctx, ins),
        Mnemonic::Mov | Mnemonic::Movzx => lower_mov(ctx, ins),
        Mnemonic::Cmp => lower_cmp(ctx, ins),
        Mnemonic::Test => lower_test(ctx, ins),
        Mnemonic::Je // zf = 1
        | Mnemonic::Jne // zf = 0
        | Mnemonic::Ja // jump if above (unsigned) (cf = 0 and zf = 0)
        | Mnemonic::Jae // jump if aboce or eq (unsigned) (cf = 0)
        | Mnemonic::Jb // jump if below (unsigned) cf = 1
        | Mnemonic::Jbe // jump if below or eq (unsigned) cf = 1 or zf = 1
        | Mnemonic::Jg // jump if greater (signed) (zf = 0 and sf = of) 
        | Mnemonic::Jge // jump if greater or equal (sf = of) 
        | Mnemonic::Jl // jump if less (signed) (sf <> of) 
        | Mnemonic::Jle // 	Jump if less or equal (signed) (zf = 1 or sf <> of)
        | Mnemonic::Jo //  of = 1
        | Mnemonic::Jno // of = 0
        | Mnemonic::Js // sf = 1
        | Mnemonic::Jns //  sf = 0
            => return Some(lower_jmp_x(ctx, ins)),

        Mnemonic::Jmp => return Some(lower_jmp(ctx, ins)),

        Mnemonic::Add => lower_bin_set_flags(ctx, ins, BinOp::Add),
        Mnemonic::Sub => lower_bin_set_flags(ctx, ins, BinOp::Sub),
        Mnemonic::Sbb => lower_sbb(ctx, ins),

        Mnemonic::Or => lower_binary_bit_op(ctx, ins, BinOp::BitOr),
        Mnemonic::And => lower_binary_bit_op(ctx, ins, BinOp::BitAnd),
        Mnemonic::Xor => lower_binary_bit_op(ctx, ins, BinOp::Xor),

        Mnemonic::Shl => lower_shift(ctx, ins, BinOp::ShiftLeft),
        Mnemonic::Shr => lower_shift(ctx, ins, BinOp::ShifRight),
        Mnemonic::Sar => lower_shift(ctx, ins, BinOp::ShifArithRight),

        Mnemonic::Neg => lower_neg(ctx, ins),
        Mnemonic::Not => lower_not(ctx, ins),

        Mnemonic::Lea => lower_lea(ctx, ins),
        Mnemonic::Inc | Mnemonic::Dec => lower_inc_dec(ctx, ins),

        Mnemonic::Ret => return Some(lower_ret(ctx, ins)),

        Mnemonic::Sete | Mnemonic::Setne => lower_sete_setne(ctx, ins),

        Mnemonic::Fild => lower_fild(ctx, ins),
        Mnemonic::Fstp => lower_fstp(ctx, ins),

        _ => {
            eprintln!("{}", ctx);
            panic!("unknown instruction: {} at {}", ins, Addr(ins.ip32()))
        }
    }
    None
}

fn lower_ret(_ctx: &mut LowerCtx, ins: &iced_x86::Instruction) -> Terminator {
    let adjust = if ins.op_count() > 0 {
        ins.immediate16()
    } else {
        0
    };

    Terminator::Ret {
        stack_adjust: adjust,
    }
}

pub fn lower_block(code: &[u8], block_addr: Addr) -> Block {
    let mut ctx = LowerCtx::new();
    let decoder = iced_x86::Decoder::with_ip(
        32,
        code,
        block_addr.0.into(),
        iced_x86::DecoderOptions::NONE,
    );

    for ins in decoder {
        if let Some(terminator) = lower_ins(&mut ctx, &ins) {
            return Block {
                addr: block_addr,
                instr: ctx.instrs,
                terminator: AnnotatedTerminator {
                    addr: Addr(ins.ip32()),
                    inner: terminator,
                },
            };
        }
    }

    panic!("reached end of code")
}
