use crate::{
    addr::Addr,
    lir::{block::BlockId, fmt::FmtList, ty::Ty, value::ValueId},
};

use std::ops::{Index, IndexMut};

#[derive(Debug, Clone)]
pub struct Instrs(Vec<Ins>);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct InsId(u16);

impl Instrs {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn add(&mut self, value: Ins) -> InsId {
        let id = self.0.len().try_into().expect("to much values");
        self.0.push(value);
        InsId(id)
    }

    pub fn keys(&self) -> impl Iterator<Item = InsId> {
        let max = self.0.len().try_into().unwrap();
        InsKeys(0..max)
    }
}

#[derive(Debug, Clone)]
pub struct InsKeys(std::ops::Range<u16>);

impl Iterator for InsKeys {
    type Item = InsId;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(InsId)
    }
}

impl Index<InsId> for Instrs {
    type Output = Ins;

    fn index(&self, index: InsId) -> &Self::Output {
        self.0.index(usize::from(index.0))
    }
}

impl IndexMut<InsId> for Instrs {
    fn index_mut(&mut self, index: InsId) -> &mut Self::Output {
        self.0.index_mut(usize::from(index.0))
    }
}

impl std::fmt::Debug for InsId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ins{}", self.0)
    }
}

impl std::fmt::Display for InsId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
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
        }
    }
}

#[derive(Debug, Clone)]
pub enum Ins {
    BinOp {
        op: BinOp,
        dst: ValueId,
        lhs: ValueId,
        rhs: ValueId,
        flags: Option<ValueId>,
    },
    Const {
        dst: ValueId,
        val: Imm,
    },
    Uninit {
        dst: ValueId,
    },
    Unimpl {
        dst: ValueId,
    },
    Load {
        dst: ValueId,
        addr: ValueId,
        mem: ValueId,
        space: MemSpace,
    },
    Store {
        dst_mem: ValueId,
        src_mem: ValueId,
        addr: ValueId,
        value: ValueId,
        space: MemSpace,
    },
    Cond {
        dst: ValueId,
        flags: ValueId,
        cond: Condition,
    },
    Call {
        result: Vec<ValueId>,
        target: CallTarget,
        args: Vec<ValueId>,
    },
}

impl std::fmt::Display for Ins {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ins::BinOp {
                op,
                dst,
                lhs,
                rhs,
                flags,
            } => {
                if let Some(flags) = flags {
                    write!(f, "{dst}, {flags} = {lhs} {op} {rhs}")
                } else {
                    write!(f, "{dst} = {lhs} {op} {rhs}")
                }
            }
            Ins::Uninit { dst } => write!(f, "{dst} = ???"),
            Ins::Const { dst, val } => write!(f, "{dst} = const {val}"),
            Ins::Unimpl { dst } => write!(f, "{dst} = unimplemented"),
            Ins::Load {
                dst,
                addr,
                space,
                mem,
            } => write!(f, "{dst} = load {mem} {space}[{addr}]"),
            Ins::Store {
                dst_mem,
                src_mem,
                addr,
                value: src,
                space,
            } => write!(f, "{dst_mem} = store {src_mem} {space}[{addr}] <- {src}"),
            Ins::Cond {
                dst,
                flags: src,
                cond,
            } => write!(f, "{dst} = cond({cond}) {src}"),
            Ins::Call {
                result,
                target,
                args,
            } => {
                write!(f, "{} = call {target}{}", FmtList(result), FmtList(args))
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CallTarget {
    Known { addr: Addr },
    Unknown { addr: ValueId },
}

impl std::fmt::Display for CallTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallTarget::Known { addr } => write!(f, "func_{}", addr),
            CallTarget::Unknown { addr } => write!(f, "?{addr}"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum JumpTarget {
    Known { block: BlockId, args: Vec<ValueId> },
    Unknown { addr: ValueId },
}

impl std::fmt::Display for JumpTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JumpTarget::Known { block, args } => write!(f, "{block}{}", FmtList(args)),
            JumpTarget::Unknown { addr: val } => write!(f, "?{val}"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Terminator {
    Jump(JumpTarget),
    Brif {
        cond: ValueId,
        thenb: JumpTarget,
        elseb: JumpTarget,
    },
    Ret {
        adjust: u16,
        args: Vec<ValueId>,
    },
}

impl std::fmt::Display for Terminator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Terminator::Jump(t) => write!(f, "jump {t}"),
            Terminator::Brif { cond, thenb, elseb } => {
                write!(f, "brif {cond} then {thenb} else {elseb}",)
            }
            Terminator::Ret { adjust, args } => {
                write!(f, "ret stack:{} {}", adjust, FmtList(args))
            }
        }
    }
}

impl Terminator {
    pub fn visit_jumps_mut(&mut self, mut callback: impl FnMut(BlockId, &mut Vec<ValueId>)) {
        match self {
            Terminator::Jump(JumpTarget::Known { block, args }) => callback(*block, args),
            Terminator::Jump(JumpTarget::Unknown { .. }) => {}
            Terminator::Brif {
                cond: _,
                thenb,
                elseb,
            } => {
                if let JumpTarget::Known { block, args } = thenb {
                    callback(*block, args);
                }

                if let JumpTarget::Known { block, args } = elseb {
                    callback(*block, args);
                }
            }
            Terminator::Ret { .. } => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Imm {
    U8(u8),
    U16(u16),
    U32(u32),
}

impl Imm {
    pub fn ty(self) -> Ty {
        match self {
            Imm::U8(_) => Ty::U8,
            Imm::U16(_) => Ty::U16,
            Imm::U32(_) => Ty::U32,
        }
    }
}

impl std::fmt::Display for Imm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Imm::U8(x) => write!(f, "{x}.u8"),
            Imm::U16(x) => write!(f, "{x}.u16"),
            Imm::U32(x) => write!(f, "{x}.u32"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemSpace {
    Default,
    Fs,
}

impl std::fmt::Display for MemSpace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemSpace::Default => Ok(()),
            MemSpace::Fs => write!(f, "fs:"),
        }
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
