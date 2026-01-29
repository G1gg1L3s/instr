use crate::{
    addr::Addr,
    lir::{block::BlockId, fmt::FmtList, io::IoValues, value::ValueId},
};

use std::ops::{Index, IndexMut};

#[derive(Debug, Clone)]
pub struct Instrs(Vec<Ins>);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct InsId(u16);

impl Default for Instrs {
    fn default() -> Self {
        Self::new()
    }
}

impl Instrs {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn add(&mut self, value: Ins) -> InsId {
        let id = self.0.len().try_into().expect("to much values");
        self.0.push(value);
        InsId(id)
    }

    pub fn keys(&self) -> impl Iterator<Item = InsId> + use<> {
        let max = self.0.len().try_into().unwrap();
        InsKeys(0..max)
    }

    pub fn values(&self) -> impl Iterator<Item = &Ins> {
        self.0.iter()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Ins> {
        self.0.iter_mut()
    }

    pub fn iter(&self) -> impl Iterator<Item = (InsId, &Ins)> {
        self.keys().zip(self.0.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (InsId, &mut Ins)> {
        self.keys().zip(self.0.iter_mut())
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

    Condition(Condition),
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
            BinOp::Condition(cond) => write!(f, "{cond}"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum InsKind {
    Hole,
    BinOp {
        op: BinOp,
        dst: ValueId,
        lhs: ValueId,
        rhs: ValueId,
        flags: Option<ValueId>,
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
        result: IoValues,
        target: CallTarget,
        args: IoValues,
    },
}

#[derive(Debug, Clone)]
pub struct Ins {
    pub addr: Option<Addr>,
    pub kind: InsKind,
}

impl Ins {
    pub fn new(addr: impl Into<Option<Addr>>, kind: InsKind) -> Self {
        Self {
            addr: addr.into(),
            kind,
        }
    }

    pub fn without_address(kind: InsKind) -> Self {
        Self { addr: None, kind }
    }

    pub fn patch_replace_value(&mut self, from: ValueId, to: ValueId) {
        self.visit_values_mut(|val| {
            if *val == from {
                *val = to;
            }
        });
    }

    pub fn visit_values_mut(&mut self, mut callback: impl FnMut(&mut ValueId)) {
        match &mut self.kind {
            InsKind::Hole => {}
            InsKind::BinOp {
                op: _,
                dst,
                lhs,
                rhs,
                flags,
            } => {
                callback(dst);
                callback(lhs);
                callback(rhs);
                flags.as_mut().map(callback);
            }
            InsKind::Uninit { dst } => callback(dst),
            InsKind::Unimpl { dst } => callback(dst),
            InsKind::Load {
                dst,
                addr,
                mem,
                space: _,
            } => {
                callback(dst);
                callback(addr);
                callback(mem);
            }
            InsKind::Store {
                dst_mem,
                src_mem,
                addr,
                value,
                space: _,
            } => {
                callback(dst_mem);
                callback(src_mem);
                callback(addr);
                callback(value);
            }
            InsKind::Cond {
                dst,
                flags,
                cond: _,
            } => {
                callback(dst);
                callback(flags);
            }
            InsKind::Call {
                result,
                target,
                args,
            } => {
                for arg in result.values_mut().chain(args.values_mut()) {
                    callback(arg);
                }

                match target {
                    CallTarget::Known { addr: _ } => {}
                    CallTarget::Unknown { addr } => callback(addr),
                }
            }
        }
    }

    pub fn visit_arg_values(&self, mut callback: impl FnMut(ValueId)) {
        match &self.kind {
            InsKind::Hole => {}
            InsKind::BinOp {
                op: _,
                dst: _,
                lhs,
                rhs,
                flags: _,
            } => {
                callback(*lhs);
                callback(*rhs);
            }
            InsKind::Uninit { .. } => {}
            InsKind::Unimpl { .. } => {}
            InsKind::Load {
                dst: _,
                addr,
                mem,
                space: _,
            } => {
                callback(*addr);
                callback(*mem);
            }
            InsKind::Store {
                dst_mem: _,
                src_mem,
                addr,
                value,
                space: _,
            } => {
                callback(*src_mem);
                callback(*addr);
                callback(*value);
            }
            InsKind::Cond {
                dst: _,
                flags,
                cond: _,
            } => {
                callback(*flags);
            }
            InsKind::Call {
                result: _,
                target,
                args,
            } => {
                for arg in args.values() {
                    callback(arg);
                }

                match target {
                    CallTarget::Known { addr: _ } => {}
                    CallTarget::Unknown { addr } => callback(*addr),
                }
            }
        }
    }

    pub fn has_side_effects(&self) -> bool {
        match &self.kind {
            InsKind::Hole => false,
            InsKind::BinOp { .. } => false,
            InsKind::Uninit { .. } => false,
            InsKind::Unimpl { .. } => true,
            InsKind::Load { .. } => false,
            InsKind::Store { .. } => true,
            InsKind::Cond { .. } => false,
            InsKind::Call { .. } => true,
        }
    }

    pub fn instr_result(&self) -> heapless::Vec<ValueId, 16> {
        let mut res = heapless::Vec::<ValueId, 16>::new();
        match &self.kind {
            InsKind::Hole => {}
            InsKind::Uninit { dst } => res.push(*dst).unwrap(),
            InsKind::BinOp { dst, flags, .. } => {
                res.push(*dst).unwrap();
                if let Some(flags) = flags {
                    res.push(*flags).unwrap()
                }
            }
            InsKind::Unimpl { dst } => res.push(*dst).unwrap(),
            InsKind::Load { dst, .. } => res.push(*dst).unwrap(),
            InsKind::Cond { dst, .. } => res.push(*dst).unwrap(),
            InsKind::Store { dst_mem, .. } => res.push(*dst_mem).unwrap(),
            InsKind::Call { result, .. } => res.extend(result.values()),
        }

        res
    }
}

impl std::fmt::Display for Ins {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            InsKind::Hole => write!(f, "hole"),
            InsKind::BinOp {
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
            InsKind::Uninit { dst } => write!(f, "{dst} = ???"),
            InsKind::Unimpl { dst } => write!(f, "{dst} = unimplemented"),
            InsKind::Load {
                dst,
                addr,
                space,
                mem,
            } => write!(f, "{dst} = load {mem} {space}[{addr}]"),
            InsKind::Store {
                dst_mem,
                src_mem,
                addr,
                value: src,
                space,
            } => write!(f, "{dst_mem} = store {src_mem} {space}[{addr}] <- {src}"),
            InsKind::Cond {
                dst,
                flags: src,
                cond,
            } => write!(f, "{dst} = cond({cond}) {src}"),
            InsKind::Call {
                result,
                target,
                args,
            } => {
                write!(f, "({}) = call {target}({})", result, args)
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
    Unknown { addr: ValueId, args: IoValues },
    Tailcall { addr: Addr, args: IoValues },
}

impl std::fmt::Display for JumpTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JumpTarget::Known { block, args } => write!(f, "{block}{}", FmtList(args)),
            JumpTarget::Unknown { addr: val, args } => write!(f, "?{val}({args})"),
            JumpTarget::Tailcall { addr, args } => write!(f, "tailcall {addr}({args})"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TerminatorKind {
    Jump(JumpTarget),
    Brif {
        cond: ValueId,
        thenb: JumpTarget,
        elseb: JumpTarget,
    },
    Ret {
        adjust: u16,
        args: IoValues,
    },
}

#[derive(Debug, Clone)]
pub struct Terminator {
    pub kind: TerminatorKind,
    pub addr: Option<Addr>,
}

impl std::fmt::Display for TerminatorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Jump(t) => write!(f, "jump {t}"),
            Self::Brif { cond, thenb, elseb } => {
                write!(f, "brif {cond} then {thenb} else {elseb}",)
            }
            Self::Ret { adjust, args } => {
                write!(f, "ret stack:{} ({})", adjust, args)
            }
        }
    }
}

impl Terminator {
    fn visit_target(jp: &JumpTarget, mut callback: impl FnMut(ValueId)) {
        match jp {
            JumpTarget::Known { block: _, args } => {
                args.iter().copied().map(callback).count();
            }
            JumpTarget::Unknown { addr, args } => {
                callback(*addr);
                args.values().for_each(callback);
            }
            JumpTarget::Tailcall { addr: _, args } => {
                args.values().for_each(callback);
            }
        }
    }

    fn visit_target_mut(jp: &mut JumpTarget, mut callback: impl FnMut(&mut ValueId)) {
        match jp {
            JumpTarget::Known { block: _, args } => {
                args.iter_mut().map(callback).count();
            }
            JumpTarget::Unknown { addr, args } => {
                callback(addr);
                args.values_mut().for_each(callback);
            }
            JumpTarget::Tailcall { addr: _, args } => {
                args.values_mut().for_each(callback);
            }
        }
    }

    pub fn visit_values(&self, mut callback: impl FnMut(ValueId)) {
        match &self.kind {
            TerminatorKind::Jump(jp) => {
                Self::visit_target(jp, callback);
            }
            TerminatorKind::Brif { cond, thenb, elseb } => {
                callback(*cond);
                Self::visit_target(thenb, &mut callback);
                Self::visit_target(elseb, &mut callback);
            }
            TerminatorKind::Ret { adjust: _, args } => {
                args.values().for_each(callback);
            }
        }
    }

    pub fn visit_values_mut(&mut self, mut callback: impl FnMut(&mut ValueId)) {
        match &mut self.kind {
            TerminatorKind::Jump(jp) => {
                Self::visit_target_mut(jp, callback);
            }
            TerminatorKind::Brif { cond, thenb, elseb } => {
                callback(cond);
                Self::visit_target_mut(thenb, &mut callback);
                Self::visit_target_mut(elseb, &mut callback);
            }
            TerminatorKind::Ret { adjust: _, args } => {
                args.values_mut().map(callback).count();
            }
        }
    }

    pub fn visit_jumps_mut(&mut self, mut callback: impl FnMut(BlockId, &mut Vec<ValueId>)) {
        match &mut self.kind {
            TerminatorKind::Jump(JumpTarget::Known { block, args }) => callback(*block, args),
            TerminatorKind::Jump(JumpTarget::Unknown { .. }) => {}
            TerminatorKind::Jump(JumpTarget::Tailcall { .. }) => {}
            TerminatorKind::Brif {
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
            TerminatorKind::Ret { .. } => {}
        }
    }

    pub fn visit_jumps(&self, mut callback: impl FnMut(BlockId, &Vec<ValueId>)) {
        match &self.kind {
            TerminatorKind::Jump(JumpTarget::Known { block, args }) => callback(*block, args),
            TerminatorKind::Jump(JumpTarget::Unknown { .. }) => {}
            TerminatorKind::Jump(JumpTarget::Tailcall { .. }) => {}
            TerminatorKind::Brif {
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
            TerminatorKind::Ret { .. } => {}
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
