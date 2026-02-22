use iced_x86::{Mnemonic, OpKind};

use crate::{addr::Addr, fmt::AsList, third_cfg::CfgDb};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    St(u8),
}
impl Reg {
    pub fn size(&self) -> Size {
        match self {
            Reg::Eax => Size::U32,
            Reg::Ebx => Size::U32,
            Reg::Ecx => Size::U32,
            Reg::Edx => Size::U32,
            Reg::Esi => Size::U32,
            Reg::Edi => Size::U32,
            Reg::Ebp => Size::U32,
            Reg::Esp => Size::U32,
            Reg::Eip => Size::U32,
            Reg::St(_) => Size::F64,
        }
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
        // X87 C3
        const C3     =   1 << 8;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlagxGroup(Flagx);

impl std::fmt::Display for FlagxGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            x if x == Self::ALL => write!(f, "f:*"),
            x if x == Self::NOCARRY => write!(f, "f:!c"),
            x if x == Self::CARRY_OVERFOW => write!(f, "f:co"),
            x if x == Self::X87_C1 => write!(f, "f:c1"),
            x if x == Self::X87_C1_C2 => write!(f, "f:c1c2"),
            x if x == Self::X87_COM => write!(f, "f:x87com"),
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
    pub const CARRY_OVERFOW: Self = Self(Flagx::CARRY.union(Flagx::OVERFLOW));

    pub const X87_C1: Self = Self(Flagx::C1);
    pub const X87_C1_C2: Self = Self(Flagx::C1.union(Flagx::C2));
    pub const X87_COM: Self = Self(Flagx::C0.union(Flagx::C1).union(Flagx::C2).union(Flagx::C3));

    pub fn is_empty(self) -> bool {
        self.0.is_empty()
    }

    pub fn flags(self) -> Flagx {
        self.0
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
    ParityEven,
    ParityOdd,
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
            Condition::ParityEven => "parity_even",
            Condition::ParityOdd => "parity_odd",
        };
        write!(f, "{literal}")
    }
}

impl Condition {
    pub fn required_flags(self) -> Flagx {
        use Condition as C;

        match self {
            C::Equal | C::NotEqual => Flagx::ZERO,
            C::UnsignedLess | C::UnsignedGreaterEqual => Flagx::CARRY,
            C::UnsignedLessEqual | C::UnsignedGreater => Flagx::CARRY | Flagx::ZERO,
            C::SignLess | C::SignedGreaterEqual => Flagx::SIGN | Flagx::OVERFLOW,
            C::SignedLessEqual | C::SignedGreater => Flagx::ZERO | Flagx::SIGN | Flagx::OVERFLOW,
            C::Negative | C::Positive => Flagx::SIGN,
            C::Overflow | C::NoOverflow => Flagx::OVERFLOW,
            C::ParityEven | C::ParityOdd => Flagx::PARITY,
        }
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
            Reg::St(i) => write!(f, "st({i})"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TempId(u16);

impl std::fmt::Display for TempId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "t{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Temp {
    pub size: Size,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vars {
    temps: Vec<Temp>,
}

impl Vars {
    fn new() -> Self {
        Self { temps: vec![] }
    }

    fn new_temp(&mut self, size: Size) -> Value {
        let id = self.temps.len().try_into().expect("too much temps");
        self.temps.push(Temp { size });
        Value::Temp(TempId(id))
    }

    fn temp(&self, id: TempId) -> Temp {
        let idx = usize::from(id.0);
        self.temps[idx]
    }

    fn size(&self, value: Value) -> Size {
        match value {
            Value::Reg(reg) => reg.size(),
            Value::Imm(imm) => imm.size(),
            Value::Temp(temp) => self.temp(temp).size,
            Value::Flag(_) => Size::U1,
            Value::X87StatusWord => Size::U16,
            Value::X87ControlWord => Size::U16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Imm {
    U8(u8),
    U16(u16),
    U32(u32),
    F64(f64),
}
impl Imm {
    fn new_u(val: u8, size: Size) -> Option<Self> {
        match size {
            Size::U8 => Some(Self::U8(val)),
            Size::U16 => Some(Self::U16(val.into())),
            Size::U32 => Some(Self::U32(val.into())),

            Size::U1
            | Size::I8
            | Size::I16
            | Size::I32
            | Size::F32
            | Size::F64
            | Size::U64
            | Size::I64 => None,
        }
    }

    pub fn size(&self) -> Size {
        match self {
            Imm::U8(_) => Size::U8,
            Imm::U16(_) => Size::U16,
            Imm::U32(_) => Size::U32,
            Imm::F64(_) => Size::F64,
        }
    }
}

impl std::fmt::Display for Imm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Imm::U8(x) => write!(f, "0x{:x}", x),
            Imm::U16(x) => write!(f, "0x{:x}", x),
            Imm::U32(x) => write!(f, "0x{:x}", x),
            Imm::F64(x) => write!(f, "{x}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    U1,
    U8,
    U16,
    U32,
    U64,

    I8,
    I16,
    I32,
    I64,

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
            Size::U64 => Some(8),

            Size::I8 => Some(1),
            Size::I16 => Some(2),
            Size::I32 => Some(4),
            Size::I64 => Some(8),

            Size::F32 => Some(4),
            Size::F64 => Some(8),
        }
    }

    fn to_signed(self) -> Option<Self> {
        match self {
            Size::U1 => None,
            Size::U8 | Size::I8 => Some(Size::I8),
            Size::U16 | Size::I16 => Some(Size::I16),
            Size::U32 | Size::I32 => Some(Size::I32),
            Size::U64 | Size::I64 => Some(Size::I64),
            Size::F32 | Size::F64 => Some(self),
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
            Size::U64 => write!(f, "u64"),

            Size::I8 => write!(f, "i8"),
            Size::I16 => write!(f, "i16"),
            Size::I32 => write!(f, "i32"),
            Size::I64 => write!(f, "i64"),

            Size::F32 => write!(f, "f32"),
            Size::F64 => write!(f, "f64"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Reg(Reg),
    Imm(Imm),
    Temp(TempId),
    Flag(Flag),
    X87StatusWord,
    X87ControlWord,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Reg(reg) => write!(f, "{reg}"),
            Value::Imm(x) => write!(f, "{x}"),
            Value::Temp(x) => write!(f, "{x}"),
            Value::Flag(flag) => write!(f, "{flag}"),
            Value::X87StatusWord => write!(f, "__x87_status_word"),
            Value::X87ControlWord => write!(f, "__x87_control_word"),
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
    Mulu,
    Muls,
    Div,
    Xor,
    BitAnd,
    BitOr,

    ShiftLeft,
    ShifRight,
    ShifArithRight,

    Atan2,
}

impl std::fmt::Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mulu => write!(f, "u*"),
            BinOp::Muls => write!(f, "s*"),
            BinOp::Div => write!(f, "/"),
            BinOp::Xor => write!(f, "xor"),
            BinOp::BitAnd => write!(f, "&"),
            BinOp::BitOr => write!(f, "|"),
            BinOp::ShiftLeft => write!(f, "<<"),
            BinOp::ShifRight => write!(f, ">>"),
            BinOp::ShifArithRight => write!(f, "a>>"),
            BinOp::Atan2 => write!(f, "atan2"),
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Sqrt,
    Sin,
    Cos,
    Abs,
}

impl std::fmt::Display for UnOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnOp::Sqrt => write!(f, "sqrt"),
            UnOp::Sin => write!(f, "sin"),
            UnOp::Cos => write!(f, "cos"),
            UnOp::Abs => write!(f, "abs"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Instr {
    BinOp {
        op: BinOp,
        dst: Option<Value>,
        lhs: Value,
        rhs: Value,
        flags: FlagxGroup,
    },
    UnOp {
        op: UnOp,
        dst: Option<Value>,
        src: Value,
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

    Nop,

    X87Push {
        src: Value,
        flags: FlagxGroup,
    },

    X87Pop {
        /// None = just pop, Some = pop + store
        dst: Option<Value>,
        flags: FlagxGroup,
    },

    Memset {
        addr: Value,
        value: Value,
        count: Value,
    },

    Memcpy {
        dst_addr: Value,
        src_addr: Value,
        count: Value,
        size: Size,
    },

    Unknown(iced_x86::Instruction),
}

#[derive(Debug, Clone, Copy)]
pub struct AnnotatedInstr {
    pub addr: Addr,
    pub ins: Instr,
}

impl Instr {
    pub fn fmt<'a>(&'a self, vars: &'a Vars) -> InstrPrinter<'a> {
        InstrPrinter { vars, ins: self }
    }
}

pub struct InstrPrinter<'a> {
    vars: &'a Vars,
    ins: &'a Instr,
}

impl<'a> std::fmt::Display for InstrPrinter<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.ins {
            Instr::BinOp {
                op,
                dst,
                lhs,
                rhs,
                flags,
            } => {
                match dst {
                    Some(dst) => write!(f, "{dst}"),
                    None => write!(f, "_"),
                }?;

                if flags.is_empty() {
                    write!(f, " = {lhs} {op} {rhs}")
                } else {
                    write!(f, ", {flags} = {lhs} {op} {rhs}")
                }
            }
            Instr::UnOp { op, dst, src, flags }=> {
               match dst {
                    Some(dst) => write!(f, "{dst}"),
                    None => write!(f, "_"),
                }?;

                if flags.is_empty() {
                    write!(f, " = {op} {src}")
                } else {
                    write!(f, ", {flags} = {op} {src}")
                }
            }
            Instr::Assign { dst, src } => write!(f, "{dst} = {src}"),
            Instr::Load { dst, addr, space } => {
                let size = self.vars.size(*dst);
                write!(f, "{dst} = load {size} [{addr}]")?;
                if *space == MemSpace::Fs {
                    write!(f, " [fs]")?;
                }
                Ok(())
            }
            Instr::Store { addr, src, space } => {
                let size = self.vars.size(*src);
                write!(f, "[{addr}] <- store {size} {src}")?;
                if *space == MemSpace::Fs {
                    write!(f, " [fs]")?;
                }
                Ok(())
            }
            Instr::Call { target } => {
                write!(f, "call {target}")
            }
            Instr::Not { dst, src } => write!(f, "{dst} = not {src}"),
            Instr::SliceBytes { dst, src, start } => {
                let end = u32::from(*start) + self.vars.size(*dst).to_bytes().unwrap();

                write!(f, "{dst} = slice {src}[{start}..{end}]")
            }
            Instr::SetBytes {
                dst,
                base,
                value,
                start,
            } => {
                let end = u32::from(*start) + self.vars.size(*value).to_bytes().unwrap();
                write!(f, "{dst} = slice {base}[{start}..{end}] set {value}")
            }
            Instr::Convert { dst, src } => {
                let src_size = self.vars.size(*src);
                let dst_size = self.vars.size(*dst);
                write!(f, "{dst} = {src_size}to{dst_size} {src}")
            }
            Instr::Nop => write!(f, "nop"),
            Instr::X87Push { src, flags } => {
                if flags.is_empty() {
                    write!(f, "x87.push {src}")
                } else {
                    write!(f, "{flags} = x87.push {src}")
                }
            }
            Instr::X87Pop { dst, flags } => {
                match (dst, flags.is_empty()) {
                    (Some(dst), true) => write!(f, "{dst} = ")?,
                    (Some(dst), false) => write!(f, "{dst}, {flags} = ")?,
                    (None, false) => write!(f, "{flags} = ")?,
                    (None, true) => write!(f, "_ = ")?,
                }

                write!(f, "x87.pop")
            }
            Instr::Memset { addr, value, count } => write!(f, "__memset({addr}, {value}, {count})"),
            Instr::Memcpy {
                dst_addr,
                src_addr,
                count,
                size,
            } => write!(f, "__memcpy_{size}({dst_addr}, {src_addr}, {count})"),
            Instr::Unknown(i) => write!(f, "unknown ({i})"),
        }
    }
}

#[derive(Debug)]
pub struct LowerCtx {
    instrs: Vec<AnnotatedInstr>,
    vars: Vars,
    addr: Addr,
}

impl std::fmt::Display for LowerCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for chunk in self.instrs.chunk_by(|a, b| a.addr == b.addr) {
            let (head, tail) = chunk.split_first().unwrap();

            writeln!(f, "{}: {}", chunk[0].addr, head.ins.fmt(&self.vars))?;

            for ins in tail {
                writeln!(f, "          {}", ins.ins.fmt(&self.vars))?;
            }
        }
        Ok(())
    }
}

impl Default for LowerCtx {
    fn default() -> Self {
        Self::new()
    }
}

impl LowerCtx {
    pub fn new() -> Self {
        Self {
            instrs: Vec::new(),
            vars: Vars::new(),
            addr: Addr(0),
        }
    }

    fn new_temp(&mut self, size: Size) -> Value {
        self.vars.new_temp(size)
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

    fn size(&self, value: Value) -> Size {
        self.vars.size(value)
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

        R::ST0 => Some(Reg::St(0)),
        R::ST1 => Some(Reg::St(1)),
        R::ST2 => Some(Reg::St(2)),
        R::ST3 => Some(Reg::St(3)),
        R::ST4 => Some(Reg::St(4)),
        R::ST5 => Some(Reg::St(5)),
        R::ST6 => Some(Reg::St(6)),
        R::ST7 => Some(Reg::St(7)),

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

fn split_i32(u: u32) -> (bool, u32) {
    let i = u as i32;

    if i >= 0 {
        (true, i as u32)
    } else {
        (false, i.wrapping_abs() as u32)
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
                op: BinOp::Mulu,
                dst: Some(scaled),
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
                dst: Some(scaled_and_base),
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
        Some(if let Some(scaled_and_base) = scaled_and_base {
            let (positive, magnitude) = split_i32(disp);
            if positive {
                emit_bin(
                    ctx,
                    BinOp::Add,
                    scaled_and_base,
                    Value::Imm(Imm::U32(magnitude)),
                )
            } else {
                emit_bin(
                    ctx,
                    BinOp::Sub,
                    scaled_and_base,
                    Value::Imm(Imm::U32(magnitude)),
                )
            }
        } else {
            Value::Imm(Imm::U32(disp))
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
        iced_x86::MemorySize::Int64 => Some(Size::I64),

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
    let lhs = lower_operand(ctx, ins, 0);
    let rhs = lower_operand(ctx, ins, 1).lower_load(ctx);

    let rhs = match ins.mnemonic() {
        Mnemonic::Movzx => {
            let target_size = lhs.size();
            let res = ctx.new_temp(target_size);
            ctx.emit(Instr::Convert { dst: res, src: rhs });
            res
        }
        Mnemonic::Movsx => {
            let target_size = lhs.size();
            let target_size_signed = target_size.to_signed().unwrap();
            let res_signed = ctx.new_temp(target_size_signed);
            let res = ctx.new_temp(target_size);
            ctx.emit(Instr::Convert {
                dst: res_signed,
                src: rhs,
            });
            ctx.emit(Instr::Convert {
                dst: res,
                src: res_signed,
            });
            res
        }
        _ => rhs,
    };

    lhs.lower_store(ctx, rhs);
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
    fn size(&self) -> Size {
        match self {
            Operand::Reg(reg) => reg.size(),
            Operand::SubReg { size, .. } => *size,
            Operand::Imm(imm) => imm.size(),
            Operand::Memory { size, .. } => *size,
        }
    }

    fn lower_load(self, ctx: &mut LowerCtx) -> Value {
        match self {
            Operand::Reg(reg) => Value::Reg(reg),
            Operand::Imm(imm) => Value::Imm(imm),
            Operand::Memory { addr, space, size } => {
                let tmp = ctx.new_temp(size);

                ctx.emit(Instr::Load {
                    dst: tmp,
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
                let value_size = ctx.size(value);
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

        Register::BP => Operand::SubReg {
            reg: Reg::Ebp,
            lo: 0,
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
    let lhs_loaded = lhs.lower_load(ctx);
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
        dst: Some(new_esp),
        lhs: esp,
        rhs: Value::Imm(Imm::U32(ctx.size(value).to_bytes().unwrap())),
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
        Operand::Reg(r) => r.size(),
        Operand::SubReg { size, .. } => *size,
        Operand::Memory { size, .. } => *size,
        Operand::Imm(_) => unreachable!(),
    };

    let byte_count = size.to_bytes().unwrap();

    let esp = Value::Reg(Reg::Esp);
    let old_esp = ctx.new_temp(Size::U32);
    ctx.emit(Instr::Assign {
        dst: old_esp,
        src: esp,
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
        dst: Some(new_esp),
        lhs: esp,
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

fn bin_size(ctx: &mut LowerCtx, lhs: Value, rhs: Value) -> Size {
    let lhs = ctx.size(lhs);
    let rhs = ctx.size(rhs);
    if lhs == rhs {
        lhs
    } else {
        panic!("Different sizes: {lhs} and {rhs}")
    }
}

fn new_temp_with_bin_size(ctx: &mut LowerCtx, lhs: Value, rhs: Value) -> Value {
    let size = bin_size(ctx, lhs, rhs);
    ctx.new_temp(size)
}

fn lower_binary_bit_op(ctx: &mut LowerCtx, ins: &iced_x86::Instruction, op: BinOp) {
    lower_bin_operation(ctx, ins, |ctx, _ins, lhs, rhs| {
        let tmp = new_temp_with_bin_size(ctx, lhs, rhs);
        ctx.emit(Instr::BinOp {
            op,
            dst: Some(tmp),
            lhs,
            rhs,
            flags: FlagxGroup::ALL,
        });
        tmp
    });
}

fn lower_bin_set_flags(ctx: &mut LowerCtx, ins: &iced_x86::Instruction, op: BinOp) {
    lower_bin_operation(ctx, ins, |ctx, _ins, lhs, rhs| {
        let res = new_temp_with_bin_size(ctx, lhs, rhs);
        ctx.emit(Instr::BinOp {
            op,
            dst: Some(res),
            lhs,
            rhs,
            flags: FlagxGroup::ALL,
        });
        res
    });
}

fn lower_mul(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let rhs_op = lower_operand(ctx, ins, 0);
    let rhs = rhs_op.lower_load(ctx);
    let size = ctx.size(rhs);

    let (eax_op, edx_op) = match size {
        Size::U8 => (
            Operand::SubReg {
                reg: Reg::Eax,
                lo: 0,
                size: Size::U8,
            },
            Operand::SubReg {
                reg: Reg::Edx,
                lo: 0,
                size: Size::U8,
            },
        ),
        Size::U16 => (
            Operand::SubReg {
                reg: Reg::Eax,
                lo: 0,
                size: Size::U16,
            },
            Operand::SubReg {
                reg: Reg::Edx,
                lo: 0,
                size: Size::U16,
            },
        ),
        Size::U32 => (Operand::Reg(Reg::Eax), Operand::Reg(Reg::Edx)),
        _ => unreachable!(),
    };

    let lhs = eax_op.lower_load(ctx);
    let wide = match size {
        Size::U8 => Size::U16,
        Size::U16 => Size::U32,
        Size::U32 => Size::U64,
        _ => unreachable!(),
    };
    let lhs_wide = emit_convert(ctx, lhs, wide);
    let rhs_wide = emit_convert(ctx, rhs, wide);

    let full = emit_bin_with_flags(
        ctx,
        BinOp::Mulu,
        lhs_wide,
        rhs_wide,
        FlagxGroup::CARRY_OVERFOW,
    );

    // Low / high parts
    let low = ctx.new_temp(size);
    ctx.emit(Instr::SliceBytes {
        dst: low,
        src: full,
        start: 0,
    });

    let high = ctx.new_temp(size);
    ctx.emit(Instr::SliceBytes {
        dst: high,
        src: full,
        start: size.to_bytes().unwrap() as u8,
    });

    match size {
        Size::U8 => {
            // AX := AL ∗ SRC;
            eax_op.lower_store(ctx, low);
        }
        Size::U32 | Size::U16 => {
            // DX:AX := AX ∗ SRC;
            // EDX:EAX := EAX ∗ SRC
            edx_op.lower_store(ctx, high);
            eax_op.lower_store(ctx, low);
        }
        _ => unreachable!(),
    }
}

fn lower_imul(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let (dst, lhs, rhs) = match ins.op_count() {
        2 => {
            let lhs = lower_operand(ctx, ins, 0);
            let rhs = lower_operand(ctx, ins, 1);
            (lhs, lhs, rhs)
        }
        3 => {
            let dst = lower_operand(ctx, ins, 0);
            let lhs = lower_operand(ctx, ins, 1);
            let rhs = lower_operand(ctx, ins, 2);
            (dst, lhs, rhs)
        }
        x => panic!("unknown op count {x} for {ins} at {}", ctx.addr),
    };

    let lhs_signed_size = lhs.size().to_signed().unwrap();
    let rhs_signed_size = rhs.size().to_signed().unwrap();

    let lhs_val = lhs.lower_load(ctx);
    let lhs_val = emit_convert(ctx, lhs_val, lhs_signed_size);

    let rhs_val = rhs.lower_load(ctx);
    let rhs_val = emit_convert(ctx, rhs_val, rhs_signed_size);

    let result = emit_bin_with_flags(
        ctx,
        BinOp::Muls,
        lhs_val,
        rhs_val,
        FlagxGroup::CARRY_OVERFOW,
    );

    let dst_size = dst.size();
    let result = emit_convert(ctx, result, dst_size);
    dst.lower_store(ctx, result);
}

fn lower_shift(ctx: &mut LowerCtx, ins: &iced_x86::Instruction, op: BinOp) {
    let dst_operand = lower_operand(ctx, ins, 0);
    let lhs = dst_operand.lower_load(ctx);
    let rhs = lower_operand(ctx, ins, 1).lower_load(ctx);

    // Mask shift count: x86 masks by 0x1F for 32-bit operands
    let rhs_size = ctx.size(rhs);
    let masked_count = emit_bin(
        ctx,
        BinOp::BitAnd,
        rhs,
        Value::Imm(Imm::new_u(0x1F, rhs_size).unwrap()),
    );

    // Size is unchecked because rhs is always u8
    let result = emit_bin_with_flags_unchecked_size(ctx, op, lhs, masked_count, FlagxGroup::ALL);
    dst_operand.lower_store(ctx, result);
}

fn lower_cmp(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let lhs = lower_operand(ctx, ins, 0).lower_load(ctx);
    let rhs = lower_operand(ctx, ins, 1).lower_load(ctx);

    ctx.emit(Instr::BinOp {
        op: BinOp::Sub,
        dst: None,
        lhs,
        rhs,
        flags: FlagxGroup::ALL,
    });
}

fn lower_test(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let lhs = lower_operand(ctx, ins, 0).lower_load(ctx);
    let rhs = lower_operand(ctx, ins, 1).lower_load(ctx);

    ctx.emit(Instr::BinOp {
        op: BinOp::BitAnd,
        dst: None,
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
    ctx.emit(Instr::Convert { dst: tmp, src });
    lhs.lower_store(ctx, tmp);
}

fn lower_fld(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let value = lower_operand(ctx, ins, 0).lower_load(ctx);
    let value = emit_convert(ctx, value, Size::F64);
    ctx.emit(Instr::X87Push {
        src: value,
        flags: FlagxGroup::X87_C1,
    })
}

fn lower_fst(ctx: &mut LowerCtx, ins: &iced_x86::Instruction, fpop: Fpop) {
    let dest = lower_operand(ctx, ins, 0);

    let value = if let Fpop::Yes = fpop {
        let value = ctx.new_temp(Size::F64);
        ctx.emit(Instr::X87Pop {
            dst: Some(value),
            flags: FlagxGroup::X87_C1,
        });
        value
    } else {
        Value::Reg(Reg::St(0))
    };

    let size = dest.size();
    let value = emit_convert(ctx, value, size);
    dest.lower_store(ctx, value);
}

enum Fpop {
    No,
    Yes,
}

enum FRev {
    No,
    Yes,
}

fn lower_fbin(ctx: &mut LowerCtx, ins: &iced_x86::Instruction, op: BinOp, fpop: Fpop, frev: FRev) {
    let (lhs, rhs) = match ins.op_count() {
        1 => {
            let rhs = lower_operand(ctx, ins, 0).lower_load(ctx);
            let rhs = emit_convert(ctx, rhs, Size::F64);
            let st0 = Value::Reg(Reg::St(0));
            (st0, rhs)
        }

        2 => {
            // Always load because lhs can only be register (or rhs if frev),
            // so it's okay to assign latter
            let lhs = lower_operand(ctx, ins, 0).lower_load(ctx);
            let rhs = lower_operand(ctx, ins, 1).lower_load(ctx);
            (lhs, rhs)
        }

        x => panic!("unkown operands {}: {} at {}", x, ins, Addr(ins.ip32())),
    };

    let (dst, lhs, rhs) = if let FRev::Yes = frev {
        (lhs, rhs, lhs)
    } else {
        (lhs, lhs, rhs)
    };

    let result = emit_bin_with_flags(ctx, op, lhs, rhs, FlagxGroup::X87_C1);
    ctx.emit(Instr::Assign { dst, src: result });

    if let Fpop::Yes = fpop {
        ctx.emit(Instr::X87Pop {
            dst: None,
            flags: FlagxGroup::NONE,
        });
    }
}

fn lower_fbin_func(ctx: &mut LowerCtx, ins: &iced_x86::Instruction, op: BinOp) {
    let (lhs, rhs) = match ins.op_count() {
        0 => {
            let st0 = Value::Reg(Reg::St(0));
            let st1 = Value::Reg(Reg::St(1));
            (st1, st0)
        }
        x => panic!("unkown operands {}: {} at {}", x, ins, Addr(ins.ip32())),
    };

 

    let result = emit_bin_with_flags(ctx, op, lhs, rhs, FlagxGroup::X87_C1);
    ctx.emit(Instr::Assign { dst: lhs, src: result });

    ctx.emit(Instr::X87Pop {
        dst: None,
        flags: FlagxGroup::NONE,
    });
}


fn lower_funary(ctx: &mut LowerCtx, ins: &iced_x86::Instruction, op: UnOp, flags: FlagxGroup) {
    match ins.op_count() {
        0 => {
            let st0 = Value::Reg(Reg::St(0));
            ctx.emit(Instr::UnOp { op: op, dst: Some(st0), src: st0, flags });
        }

        x => panic!("unkown operands {}: {} at {}", x, ins, Addr(ins.ip32())),
    }
}


fn lower_fxch(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let (lhs, rhs) = match ins.op_count() {
        1 => (Value::Reg(Reg::St(0)), Value::Reg(Reg::St(1))),
        2 => (
            lower_operand(ctx, ins, 0).lower_load(ctx),
            lower_operand(ctx, ins, 1).lower_load(ctx),
        ),
        _ => unreachable!(),
    };

    let temp = ctx.new_temp(Size::F64);
    ctx.emit(Instr::Assign {
        dst: temp,
        src: lhs,
    });
    ctx.emit(Instr::Assign { dst: lhs, src: rhs });
    ctx.emit(Instr::Assign {
        dst: rhs,
        src: temp,
    });
}

fn lower_fcom(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let (lhs, rhs) = match ins.op_count() {
        0 => (Value::Reg(Reg::St(0)), Value::Reg(Reg::St(1))),
        1 => {
            let rhs = lower_operand(ctx, ins, 0).lower_load(ctx);
            let rhs = emit_convert(ctx, rhs, Size::F64);
            let lhs = Value::Reg(Reg::St(0));
            (lhs, rhs)
        }
        x => todo!("{x} operands"),
    };

    emit_bin_with_flags(ctx, BinOp::Sub, lhs, rhs, FlagxGroup::X87_COM);

    match ins.mnemonic() {
        Mnemonic::Fcomp => {
            ctx.emit(Instr::X87Pop {
                dst: None,
                flags: FlagxGroup::NONE,
            });
        }
        Mnemonic::Fcompp | Mnemonic::Fucompp => {
            ctx.emit(Instr::X87Pop {
                dst: None,
                flags: FlagxGroup::NONE,
            });
            ctx.emit(Instr::X87Pop {
                dst: None,
                flags: FlagxGroup::NONE,
            });
        }
        _ => {}
    }
}

fn lower_fnstsw(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let dest = lower_operand(ctx, ins, 0);
    dest.lower_store(ctx, Value::X87StatusWord);
}


fn lower_fnstcw(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let dest = lower_operand(ctx, ins, 0);
    dest.lower_store(ctx, Value::X87ControlWord);
}

fn lower_fldcw(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let val = lower_operand(ctx, ins, 0).lower_load(ctx);
    ctx.emit(Instr::Assign { dst: Value::X87ControlWord, src: val });
}

fn lower_fld_const(ctx: &mut LowerCtx, _ins: &iced_x86::Instruction, constant: f64) {
    ctx.emit(Instr::X87Push {
        src: Value::Imm(Imm::F64(constant)),
        flags: FlagxGroup::X87_C1,
    })
}

#[derive(Debug, Clone)]
pub enum Terminator {
    Cond {
        addr: Addr,
        cond: Condition,
        then_bb: Value,
        else_bb: Value,
    },
    Jump {
        addr: Addr,
        target: Value,
    },
    Ret {
        addr: Addr,
        stack_adjust: u16,
    },
    Fallthrough {
        next: Addr,
    },
    JumpTable {
        addr: Addr,
        jump_addr: Value,
        entries: Vec<Addr>,
    },
}

impl std::fmt::Display for Terminator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Terminator::Cond {
                cond,
                then_bb,
                else_bb,
                ..
            } => write!(f, "if {cond} then {then_bb} else {else_bb}"),
            Self::Jump { target, .. } => write!(f, "jump {target}"),
            Self::Ret { stack_adjust, .. } => write!(f, "ret {stack_adjust}"),
            Self::Fallthrough { next, .. } => write!(f, "fallthrough {next}"),
            Self::JumpTable {
                addr: _,
                jump_addr,
                entries,
            } => write!(f, "jumptable {jump_addr} {}", AsList(entries)),
        }
    }
}

impl Terminator {
    pub fn addr(&self) -> Option<Addr> {
        match self {
            Terminator::Cond { addr, .. }
            | Terminator::Jump { addr, .. }
            | Terminator::Ret { addr, .. } => Some(*addr),
            Terminator::Fallthrough { .. } => None,
            Terminator::JumpTable { addr, .. } => Some(*addr),
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

        Mnemonic::Jp => Condition::ParityEven, // PF = 1
        Mnemonic::Jnp => Condition::ParityOdd, // PF = 0

        _ => panic!("unknown ins: {ins}"),
    };

    Terminator::Cond {
        addr: Addr(ins.ip32()),
        cond,
        then_bb: target,
        else_bb: Value::Imm(Imm::U32(ins.next_ip32())),
    }
}

fn lower_jmp(cfg_db: &CfgDb, ctx: &mut LowerCtx, ins: &iced_x86::Instruction) -> Terminator {
    detect_log_jump_table(ins);
    let ins_addr = Addr(ins.ip32());

    let target = lower_operand(ctx, ins, 0).lower_load(ctx);

    if let Some(jump_table) = cfg_db.get_jump_table(ins_addr) {
        Terminator::JumpTable {
            addr: ins_addr,
            jump_addr: target,
            entries: jump_table.entries.clone(),
        }
    } else {
        Terminator::Jump {
            target,
            addr: ins_addr,
        }
    }
}

fn detect_log_jump_table(ins: &iced_x86::Instruction) {
    if ins.op_kind(0) == OpKind::Memory
        && ins.memory_index() != iced_x86::Register::None
        && ins.memory_index_scale() == 4
    {
        let disp = ins.memory_displacement32();
        if disp != 0 {
            log::debug!(
                ">> Instruction {}: {} looks like jump table",
                Addr(ins.ip32()),
                ins
            );
        }
    }
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
    let size = ctx.size(lhs);
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
    let size = ctx.size(value);

    let lhs = Value::Imm(Imm::new_u(0, size).unwrap());
    let new = emit_bin_with_flags(ctx, BinOp::Sub, lhs, value, FlagxGroup::ALL);

    operand.lower_store(ctx, new);
}

fn lower_not(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let operand = lower_operand(ctx, ins, 0);
    let value = operand.lower_load(ctx);
    let size = ctx.size(value);

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
        dst: Some(new_esp),
        lhs: esp,
        rhs: Value::Imm(Imm::U32(4)),
        flags: FlagxGroup::NONE,
    });

    ctx.emit(Instr::Store {
        addr: new_esp,
        src: ebp,
        space: MemSpace::Default,
    });

    ctx.emit(Instr::Assign {
        dst: esp,
        src: new_esp,
    });

    // mov ebp, esp
    ctx.emit(Instr::Assign { dst: ebp, src: esp });

    // sub esp, imm16
    if frame_size != 0 {
        let esp_after_alloc = ctx.new_temp(Size::U32);
        ctx.emit(Instr::BinOp {
            op: BinOp::Sub,
            dst: Some(esp_after_alloc),
            lhs: esp,
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
    ctx.emit(Instr::Assign { dst: esp, src: ebp });

    // pop ebp
    let old_esp = ctx.new_temp(Size::U32);
    ctx.emit(Instr::Assign {
        dst: old_esp,
        src: esp,
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
        dst: Some(esp_after_pop),
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
    ctx.emit(Instr::Convert {
        dst: tmp,
        src: Value::Flag(flag),
    });
    tmp
}

fn lower_sbb(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    lower_bin_operation(ctx, ins, |ctx, _ins, lhs, rhs| {
        let size = bin_size(ctx, lhs, rhs);

        // CF as integer (0 or 1)
        let cf_int = lower_flag_as_int(ctx, Flag::Cf, size);

        // rhs + CF
        let rhs_plus_cf = ctx.new_temp(size);
        ctx.emit(Instr::BinOp {
            op: BinOp::Add,
            dst: Some(rhs_plus_cf),
            lhs: rhs,
            rhs: cf_int,
            flags: FlagxGroup::NONE,
        });

        // lhs - (rhs + CF)
        let res = ctx.new_temp(size);
        ctx.emit(Instr::BinOp {
            op: BinOp::Sub,
            dst: Some(res),
            lhs,
            rhs: rhs_plus_cf,
            flags: FlagxGroup::ALL,
        });

        res
    });
}

fn stos_elem_size(mnemonic: Mnemonic) -> Option<(Size, u32)> {
    match mnemonic {
        Mnemonic::Stosb => Some((Size::U8, 1)),
        Mnemonic::Stosw => Some((Size::U16, 2)),
        Mnemonic::Stosd => Some((Size::U32, 4)),
        _ => None,
    }
}

fn movs_elem_size(mnemonic: Mnemonic) -> Option<(Size, u32)> {
    match mnemonic {
        Mnemonic::Movsb => Some((Size::U8, 1)),
        Mnemonic::Movsw => Some((Size::U16, 2)),
        Mnemonic::Movsd => Some((Size::U32, 4)),
        _ => None,
    }
}
fn lower_stos(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let (elem_size, stride) = stos_elem_size(ins.mnemonic()).expect("not a STOS instruction");

    let edi = Value::Reg(Reg::Edi);
    let ecx = Value::Reg(Reg::Ecx);

    // Value comes from AL / AX / EAX depending on size
    let value = match elem_size {
        Size::U8 => lower_subregister(iced_x86::Register::AL)
            .unwrap()
            .lower_load(ctx),
        Size::U16 => lower_subregister(iced_x86::Register::AX)
            .unwrap()
            .lower_load(ctx),
        Size::U32 => Value::Reg(Reg::Eax),
        _ => unreachable!(),
    };

    // =========================
    // Non-REP STOS*
    // =========================
    if !ins.has_rep_prefix() {
        ctx.emit(Instr::Store {
            addr: edi,
            src: value,
            space: MemSpace::Default,
        });

        // EDI += stride
        let new_edi = ctx.new_temp(Size::U32);
        ctx.emit(Instr::BinOp {
            op: BinOp::Add,
            dst: Some(new_edi),
            lhs: edi,
            rhs: Value::Imm(Imm::U32(stride)),
            flags: FlagxGroup::NONE,
        });
        ctx.emit(Instr::Assign {
            dst: edi,
            src: new_edi,
        });

        return;
    }

    // =========================
    // REP STOS* → Memset
    // =========================

    ctx.emit(Instr::Memset {
        addr: edi,
        value,
        count: ecx,
    });

    // byte_count = ECX * stride
    let byte_count = ctx.new_temp(Size::U32);
    ctx.emit(Instr::BinOp {
        op: BinOp::Mulu,
        dst: Some(byte_count),
        lhs: ecx,
        rhs: Value::Imm(Imm::U32(stride)),
        flags: FlagxGroup::NONE,
    });

    // EDI += ECX * stride
    let new_edi = ctx.new_temp(Size::U32);
    ctx.emit(Instr::BinOp {
        op: BinOp::Add,
        dst: Some(new_edi),
        lhs: edi,
        rhs: byte_count,
        flags: FlagxGroup::NONE,
    });
    ctx.emit(Instr::Assign {
        dst: edi,
        src: new_edi,
    });

    // ECX = 0
    ctx.emit(Instr::Assign {
        dst: ecx,
        src: Value::Imm(Imm::U32(0)),
    });
}

fn lower_movs(ctx: &mut LowerCtx, ins: &iced_x86::Instruction) {
    let (elem_size, stride) = movs_elem_size(ins.mnemonic()).expect("not a MOVS instruction");

    let esi = Value::Reg(Reg::Esi);
    let edi = Value::Reg(Reg::Edi);
    let ecx = Value::Reg(Reg::Ecx);

    // =========================
    // Non-REP MOVS*
    // =========================
    if !ins.has_rep_prefix() {
        // tmp = [ESI]
        let tmp = ctx.new_temp(elem_size);
        ctx.emit(Instr::Load {
            dst: tmp,
            addr: esi,
            space: MemSpace::Default,
        });

        // [EDI] = tmp
        ctx.emit(Instr::Store {
            addr: edi,
            src: tmp,
            space: MemSpace::Default,
        });

        // ESI += stride
        let new_esi = ctx.new_temp(Size::U32);
        ctx.emit(Instr::BinOp {
            op: BinOp::Add,
            dst: Some(new_esi),
            lhs: esi,
            rhs: Value::Imm(Imm::U32(stride)),
            flags: FlagxGroup::NONE,
        });
        ctx.emit(Instr::Assign {
            dst: esi,
            src: new_esi,
        });

        // EDI += stride
        let new_edi = ctx.new_temp(Size::U32);
        ctx.emit(Instr::BinOp {
            op: BinOp::Add,
            dst: Some(new_edi),
            lhs: edi,
            rhs: Value::Imm(Imm::U32(stride)),
            flags: FlagxGroup::NONE,
        });
        ctx.emit(Instr::Assign {
            dst: edi,
            src: new_edi,
        });

        return;
    }

    // =========================
    // REP MOVS* → Memcpy
    // =========================

    ctx.emit(Instr::Memcpy {
        dst_addr: edi,
        src_addr: esi,
        count: ecx, // element count
        size: elem_size,
    });

    // byte_count = ECX * stride
    let byte_count = ctx.new_temp(Size::U32);
    ctx.emit(Instr::BinOp {
        op: BinOp::Mulu,
        dst: Some(byte_count),
        lhs: ecx,
        rhs: Value::Imm(Imm::U32(stride)),
        flags: FlagxGroup::NONE,
    });

    // EDI += ECX * stride
    let new_edi = ctx.new_temp(Size::U32);
    ctx.emit(Instr::BinOp {
        op: BinOp::Add,
        dst: Some(new_edi),
        lhs: edi,
        rhs: byte_count,
        flags: FlagxGroup::NONE,
    });
    ctx.emit(Instr::Assign {
        dst: edi,
        src: new_edi,
    });

    // ESI += ECX * stride
    let new_esi = ctx.new_temp(Size::U32);
    ctx.emit(Instr::BinOp {
        op: BinOp::Add,
        dst: Some(new_esi),
        lhs: esi,
        rhs: byte_count,
        flags: FlagxGroup::NONE,
    });
    ctx.emit(Instr::Assign {
        dst: esi,
        src: new_esi,
    });

    // ECX = 0
    ctx.emit(Instr::Assign {
        dst: ecx,
        src: Value::Imm(Imm::U32(0)),
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
    let lhs_size = ctx.size(lhs);
    let rhs_size = ctx.size(rhs);
    if lhs_size != rhs_size {
        panic!(
            "mismtach of binary operation sizes: {lhs_size} != {rhs_size}, for {lhs} {op} {rhs} at {}",
            ctx.addr
        );
    }

    emit_bin_with_flags_unchecked_size(ctx, op, lhs, rhs, flags)
}

fn emit_bin_with_flags_unchecked_size(
    ctx: &mut LowerCtx,
    op: BinOp,
    lhs: Value,
    rhs: Value,
    flags: FlagxGroup,
) -> Value {
    let lhs_size = ctx.size(lhs);
    let res = ctx.new_temp(lhs_size);
    ctx.emit(Instr::BinOp {
        op,
        dst: Some(res),
        lhs,
        rhs,
        flags,
    });
    res
}

fn emit_convert(ctx: &mut LowerCtx, value: Value, size: Size) -> Value {
    let last_size = ctx.size(value);
    if last_size == size {
        value
    } else {
        let res = ctx.new_temp(size);
        ctx.emit(Instr::Convert {
            dst: res,
            src: value,
        });
        res
    }
}

#[derive(Debug)]
pub struct Block {
    addr: Addr,
    instr: Vec<AnnotatedInstr>,
    vars: Vars,
    terminator: Terminator,
    size: u32,
}

impl std::fmt::Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for chunk in self.instr.chunk_by(|a, b| a.addr == b.addr) {
            let (head, tail) = chunk.split_first().unwrap();

            writeln!(f, "{}: {}", chunk[0].addr, head.ins.fmt(&self.vars))?;

            for ins in tail {
                writeln!(f, "          {}", ins.ins.fmt(&self.vars))?;
            }
        }

        let Some(last) = self.instr.last() else {
            if let Some(addr) = self.terminator.addr() {
                writeln!(f, "{}: {}", addr, self.terminator)?;
            } else {
                writeln!(f, "        : {}", self.terminator)?;
            }

            return Ok(());
        };
        if Some(last.addr) == self.terminator.addr() {
            writeln!(f, "          {}", self.terminator)?;
        } else if let Some(addr) = self.terminator.addr() {
            writeln!(f, "{}: {}", addr, self.terminator)?;
        } else {
            writeln!(f, "        : {}", self.terminator)?;
        }

        Ok(())
    }
}

impl Block {
    pub fn len_u32(&self) -> u32 {
        self.size
    }

    pub fn len(&self) -> usize {
        self.size as _
    }

    pub fn contains(&self, addr: Addr) -> bool {
        let end = self.addr + self.size;
        (self.addr..end).contains(&addr)
    }

    pub fn asm_fmt<'a>(&'a self, code: &'a [u8]) -> AsmBlockFmt<'a> {
        AsmBlockFmt { block: self, code }
    }

    pub fn addr(&self) -> Addr {
        self.addr
    }

    pub fn instr(&self) -> &[AnnotatedInstr] {
        &self.instr
    }

    pub fn terminator(&self) -> &Terminator {
        &self.terminator
    }

    pub fn temp(&self, id: TempId) -> Temp {
        self.vars.temp(id)
    }
}

pub struct AsmBlockFmt<'a> {
    block: &'a Block,
    code: &'a [u8],
}

impl<'a> std::fmt::Display for AsmBlockFmt<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut decoder = iced_x86::Decoder::with_ip(
            32,
            self.code,
            self.block.addr.0.into(),
            iced_x86::DecoderOptions::NONE,
        );
        let vars = &self.block.vars;

        for chunk in self.block.instr.chunk_by(|a, b| a.addr == b.addr) {
            let (head, tail) = chunk.split_first().unwrap();

            let asm_ins = decoder.decode();
            assert_eq!(head.addr, Addr(asm_ins.ip32()));
            let asm_ins = asm_ins.to_string();

            writeln!(
                f,
                "    {}: {:32} | {}",
                head.addr,
                asm_ins,
                head.ins.fmt(vars)
            )?;

            for ins in tail {
                writeln!(f, "    {:42 } | {}", " ", ins.ins.fmt(vars))?;
            }
        }

        let last_ins = self.block.instr.last();
        let terminator_addr = self.block.terminator.addr();

        match (last_ins, terminator_addr) {
            (Some(ins), Some(term)) if ins.addr == term => {
                writeln!(f, "    {:42 } | {}", " ", self.block.terminator)?;
            }
            (_, Some(addr)) => {
                let asm_ins = decoder.decode();
                assert_eq!(Addr(asm_ins.ip32()), addr);
                let asm_ins = asm_ins.to_string();
                writeln!(
                    f,
                    "    {}: {:32} | {}",
                    addr, asm_ins, self.block.terminator
                )?;
            }
            (Some(_), None) => {
                writeln!(f, "    {:42 } | {}", " ", self.block.terminator)?;
            }
            (None, None) => {
                writeln!(f, "              | <empty>")?;
            }
        }

        Ok(())
    }
}

fn lower_int3(_ctx: &mut LowerCtx, ins: &iced_x86::Instruction) -> Terminator {
    // TODO: proper terminator

    Terminator::Jump {
        addr: Addr(ins.ip32()),
        target: Value::Imm(Imm::U32(ins.ip32())),
    }
}

fn lower_ins(
    cfg_db: &CfgDb,
    ctx: &mut LowerCtx,
    ins: &iced_x86::Instruction,
) -> Option<Terminator> {
    ctx.set_addr(Addr(ins.ip32()));
    match ins.mnemonic() {
        Mnemonic::Push => lower_push(ctx, ins),
        Mnemonic::Pop => lower_pop(ctx, ins),
        Mnemonic::Leave => lower_leave(ctx, ins),

        Mnemonic::Call => lower_call(ctx, ins),
        Mnemonic::Mov | Mnemonic::Movzx | Mnemonic::Movsx => lower_mov(ctx, ins),
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
        | Mnemonic::Jp //  pf = 1
        | Mnemonic::Jnp //  pf = 0
            => return Some(lower_jmp_x(ctx, ins)),

        Mnemonic::Jmp => return Some(lower_jmp(cfg_db, ctx, ins)),

        Mnemonic::Add => lower_bin_set_flags(ctx, ins, BinOp::Add),
        Mnemonic::Sub => lower_bin_set_flags(ctx, ins, BinOp::Sub),
        Mnemonic::Sbb => lower_sbb(ctx, ins),

        Mnemonic::Or => lower_binary_bit_op(ctx, ins, BinOp::BitOr),
        Mnemonic::And => lower_binary_bit_op(ctx, ins, BinOp::BitAnd),
        Mnemonic::Xor => lower_binary_bit_op(ctx, ins, BinOp::Xor),

        Mnemonic::Mul => lower_mul(ctx, ins),
        Mnemonic::Imul => lower_imul(ctx, ins),

        Mnemonic::Shl => lower_shift(ctx, ins, BinOp::ShiftLeft),
        Mnemonic::Shr => lower_shift(ctx, ins, BinOp::ShifRight),
        Mnemonic::Sar => lower_shift(ctx, ins, BinOp::ShifArithRight),

        Mnemonic::Neg => lower_neg(ctx, ins),
        Mnemonic::Not => lower_not(ctx, ins),

        Mnemonic::Lea => lower_lea(ctx, ins),
        Mnemonic::Inc | Mnemonic::Dec => lower_inc_dec(ctx, ins),

        Mnemonic::Ret => return Some(lower_ret(ctx, ins)),

        Mnemonic::Sete | Mnemonic::Setne => lower_sete_setne(ctx, ins),

        Mnemonic::Fld | Mnemonic::Fild => lower_fld(ctx, ins),

        Mnemonic::Fst => lower_fst(ctx, ins, Fpop::No),
        Mnemonic::Fstp => lower_fst(ctx, ins, Fpop::Yes),

        Mnemonic::Fadd => lower_fbin(ctx, ins, BinOp::Add, Fpop::No, FRev::No),
        Mnemonic::Faddp => lower_fbin(ctx, ins, BinOp::Add, Fpop::Yes, FRev::No),
        
        Mnemonic::Fsub => lower_fbin(ctx, ins, BinOp::Sub, Fpop::No, FRev::No),
        Mnemonic::Fsubp => lower_fbin(ctx, ins, BinOp::Sub, Fpop::Yes, FRev::No),
        Mnemonic::Fsubr => lower_fbin(ctx, ins, BinOp::Sub, Fpop::No, FRev::Yes),

        Mnemonic::Fmul => lower_fbin(ctx, ins, BinOp::Mulu, Fpop::No, FRev::No),
        Mnemonic::Fmulp => lower_fbin(ctx, ins, BinOp::Mulu, Fpop::Yes, FRev::No),

        Mnemonic::Fdiv => lower_fbin(ctx, ins, BinOp::Div, Fpop::No, FRev::No),
        Mnemonic::Fdivp => lower_fbin(ctx, ins, BinOp::Div, Fpop::Yes, FRev::No),
        Mnemonic::Fdivr => lower_fbin(ctx, ins, BinOp::Div, Fpop::No, FRev::Yes),

        Mnemonic::Fpatan => lower_fbin_func(ctx, ins, BinOp::Atan2),

        Mnemonic::Fxch => lower_fxch(ctx, ins),
        Mnemonic::Fcom | Mnemonic::Fcomp | Mnemonic::Fcompp | Mnemonic::Fucompp => lower_fcom(ctx, ins),

        Mnemonic::Fnstsw => lower_fnstsw(ctx, ins),
        Mnemonic::Fnstcw => lower_fnstcw(ctx, ins),
        Mnemonic::Fldcw => lower_fldcw(ctx, ins),

        Mnemonic::Fld1 => lower_fld_const(ctx, ins, 1.0),
        Mnemonic::Fldl2e => lower_fld_const(ctx, ins, std::f64::consts::E.log2()),

        Mnemonic::Fsqrt => lower_funary(ctx, ins, UnOp::Sqrt, FlagxGroup::X87_C1),
        Mnemonic::Fsin => lower_funary(ctx, ins, UnOp::Sin, FlagxGroup::X87_C1_C2),
        Mnemonic::Fcos => lower_funary(ctx, ins, UnOp::Cos, FlagxGroup::X87_C1_C2),
        Mnemonic::Fabs => lower_funary(ctx, ins, UnOp::Abs, FlagxGroup::X87_C1),

        Mnemonic::Stosb | Mnemonic::Stosw | Mnemonic::Stosd => lower_stos(ctx, ins),
        Mnemonic::Movsb | Mnemonic::Movsw | Mnemonic::Movsd => lower_movs(ctx, ins),

        Mnemonic::Nop => ctx.emit(Instr::Nop),

        Mnemonic::Int3 => return Some(lower_int3(ctx, ins)),

        #[cfg(feature = "unknown-ins")]
        _ => ctx.emit(Instr::Unknown(*ins)),

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
        addr: Addr(ins.ip32()),
        stack_adjust: adjust,
    }
}

pub fn lower_block(cfg_db: &CfgDb, code: &[u8], block_addr: Addr) -> Block {
    let mut ctx = LowerCtx::new();
    let mut decoder = iced_x86::Decoder::with_ip(
        32,
        code,
        block_addr.0.into(),
        iced_x86::DecoderOptions::NONE,
    );

    for ins in &mut decoder {
        if let Some(terminator) = lower_ins(cfg_db, &mut ctx, &ins) {
            let size: u32 = ins.next_ip32().checked_sub(block_addr.0).unwrap();

            return Block {
                addr: block_addr,
                instr: ctx.instrs,
                vars: ctx.vars,
                terminator,
                size,
            };
        }
    }

    Block {
        addr: block_addr,
        instr: ctx.instrs,
        vars: ctx.vars,
        terminator: Terminator::Fallthrough {
            next: Addr(decoder.ip() as _),
        },
        size: code.len() as _,
    }
}

pub enum BlockOrPadding {
    Block(Block),
    Padding { next: Addr },
}

pub fn lower_maybe_block(cfg_db: &CfgDb, code: &[u8], block_addr: Addr) -> BlockOrPadding {
    let mut decoder = iced_x86::Decoder::with_ip(
        32,
        code,
        block_addr.0.into(),
        iced_x86::DecoderOptions::NONE,
    );

    let first = decoder.decode();
    if first.mnemonic() == Mnemonic::Int3 {
        loop {
            let decoded = decoder.decode();
            if decoded.mnemonic() != Mnemonic::Int3 {
                break BlockOrPadding::Padding {
                    next: Addr(decoded.ip32()),
                };
            }
        }
    } else {
        BlockOrPadding::Block(lower_block(cfg_db, code, block_addr))
    }
}
