use crate::lir::{block::BlockId, fmt::FmtList, value::ValueId};

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

#[derive(Debug, Clone, Copy)]
pub enum Ins {
    BinOp {
        op: BinOp,
        dst: ValueId,
        lhs: ValueId,
        rhs: ValueId,
    },
    Unimpl,
}

impl std::fmt::Display for Ins {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ins::BinOp { op, dst, lhs, rhs } => write!(f, "{dst} = {lhs} {op} {rhs}"),
            Ins::Unimpl => todo!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Terminator {
    Jump {
        target: BlockId,
        args: Vec<ValueId>,
    },
    Brif {
        cond: ValueId,
        thenb: BlockId,
        then_args: Vec<ValueId>,
        elseb: BlockId,
        else_args: Vec<ValueId>,
    },
    JumpUnknown {
        value: ValueId,
    },
    BrifUnknown {
        cond: ValueId,
        thenb: ValueId,
        elseb: ValueId,
    },
    Ret {
        size: u16,
    },
}

impl std::fmt::Display for Terminator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Terminator::Jump { target, args } => write!(f, "jump {target}{}", FmtList(args)),
            Terminator::Brif {
                cond,
                thenb,
                then_args,
                elseb,
                else_args,
            } => write!(
                f,
                "brif {cond} then {thenb}{} else {elseb}{}",
                FmtList(then_args),
                FmtList(else_args)
            ),
            Terminator::JumpUnknown { value } => {
                write!(f, "jump? {value}")
            }
            Terminator::BrifUnknown { cond, thenb, elseb } => {
                write!(f, "brif? {cond} then {thenb} else {elseb}")
            }
            Terminator::Ret { size } => write!(f, "ret {size}"),
        }
    }
}
