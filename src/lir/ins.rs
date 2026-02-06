use crate::{
    addr::Addr,
    lir::{block::BlockId, flags::Flag, fmt::FmtList, io::IoValues, value::ValueId},
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    BitNot,
}

impl std::fmt::Display for UnOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnOp::BitNot => write!(f, "bitnot"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawSize {
    U8,
    U16,
    U32,
}

impl std::fmt::Display for RawSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RawSize::U8 => write!(f, "u8"),
            RawSize::U16 => write!(f, "u16"),
            RawSize::U32 => write!(f, "u32"),
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
    UnOp {
        op: UnOp,
        dst: ValueId,
        src: ValueId,
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

    Extract {
        dst: ValueId,
        src: ValueId,
        offset: u8,
    },

    Insert {
        dst: ValueId,
        base: ValueId,
        value: ValueId,
        offset: u8,
    },

    Cast {
        dst: ValueId,
        src: ValueId,
    },

    ExtractFlag {
        dst: ValueId,
        src: ValueId,
        flag: Flag,
    },

    Memset {
        dst_mem: ValueId,
        src_mem: ValueId,

        addr: ValueId,
        value: ValueId,
        count: ValueId,
    },

    Memcpy {
        dst_mem: ValueId,
        src_mem: ValueId,

        dst_addr: ValueId,
        src_addr: ValueId,
        count: ValueId,
        size: RawSize,
    },

    X87InitStack {
        dst: ValueId,
    },

    X87Push {
        dst_stack: ValueId,
        dst_flags: Option<ValueId>,
        src_stack: ValueId,
        value: ValueId,
    },

    X87Pop {
        dst_stack: ValueId,
        dst_flags: Option<ValueId>,
        src_stack: ValueId,
        dst: Option<ValueId>,
    },

    X87Peek {
        dst: ValueId,
        stack: ValueId,
        idx: u8,
    },

    X87StatusWord {
        dst: ValueId,
        flags: ValueId,
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
            InsKind::UnOp { op: _, dst, src } => {
                callback(dst);
                callback(src);
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
            InsKind::Extract {
                dst,
                src,
                offset: _,
            } => {
                callback(dst);
                callback(src);
            }
            InsKind::Insert {
                dst,
                base,
                value,
                offset: _,
            } => {
                callback(dst);
                callback(base);
                callback(value);
            }
            InsKind::Cast { dst, src } => {
                callback(dst);
                callback(src);
            }
            InsKind::ExtractFlag { dst, src, flag: _ } => {
                callback(dst);
                callback(src);
            }
            InsKind::Memset {
                dst_mem,
                src_mem,
                addr,
                value,
                count,
            } => {
                callback(dst_mem);
                callback(src_mem);
                callback(addr);
                callback(value);
                callback(count);
            }
            InsKind::Memcpy {
                dst_mem,
                src_mem,
                dst_addr,
                src_addr,
                count,
                size: _,
            } => {
                callback(dst_mem);
                callback(src_mem);
                callback(dst_addr);
                callback(src_addr);
                callback(count);
            }
            InsKind::X87InitStack { dst } => callback(dst),
            InsKind::X87Push {
                dst_stack,
                dst_flags,
                src_stack,
                value,
            } => {
                callback(dst_stack);
                if let Some(x) = dst_flags {
                    callback(x);
                }
                callback(src_stack);
                callback(value);
            }
            InsKind::X87Pop {
                dst_stack,
                dst_flags,
                src_stack,
                dst,
            } => {
                callback(dst_stack);
                if let Some(x) = dst_flags {
                    callback(x);
                }
                callback(src_stack);
                dst.into_iter().for_each(callback);
            }
            InsKind::X87Peek { dst, stack, idx: _ } => {
                callback(dst);
                callback(stack);
            }
            InsKind::X87StatusWord { dst, flags } => {
                callback(dst);
                callback(flags);
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
            InsKind::UnOp { op: _, dst: _, src } => callback(*src),
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
            InsKind::Extract {
                dst: _,
                src,
                offset: _,
            } => {
                callback(*src);
            }
            InsKind::Insert {
                dst: _,
                base,
                value,
                offset: _,
            } => {
                callback(*base);
                callback(*value);
            }
            InsKind::Cast { dst: _, src } => {
                callback(*src);
            }
            InsKind::ExtractFlag {
                dst: _,
                src,
                flag: _,
            } => {
                callback(*src);
            }
            InsKind::Memset {
                dst_mem: _,
                src_mem,
                addr,
                value,
                count,
            } => {
                callback(*src_mem);
                callback(*addr);
                callback(*value);
                callback(*count);
            }
            InsKind::Memcpy {
                dst_mem: _,
                src_mem,
                dst_addr,
                src_addr,
                count,
                size: _,
            } => {
                callback(*src_mem);
                callback(*src_addr);
                callback(*dst_addr);
                callback(*count);
            }
            InsKind::X87InitStack { dst: _ } => {}
            InsKind::X87Push {
                dst_stack: _,
                dst_flags: _,
                src_stack,
                value,
            } => {
                callback(*src_stack);
                callback(*value);
            }
            InsKind::X87Pop {
                dst_stack: _,
                dst_flags: _,
                src_stack,
                dst: _,
            } => {
                callback(*src_stack);
            }
            InsKind::X87Peek {
                dst: _,
                stack,
                idx: _,
            } => {
                callback(*stack);
            }
            InsKind::X87StatusWord { dst: _, flags } => callback(*flags),
        }
    }

    pub fn has_side_effects(&self) -> bool {
        match &self.kind {
            InsKind::Hole => false,
            InsKind::BinOp { .. } => false,
            InsKind::UnOp { .. } => false,
            InsKind::Uninit { .. } => false,
            InsKind::Unimpl { .. } => true,
            InsKind::Load { .. } => false,
            InsKind::Store { .. } => true,
            InsKind::Cond { .. } => false,
            InsKind::Call { .. } => true,
            InsKind::Extract { .. } => false,
            InsKind::Insert { .. } => false,
            InsKind::Cast { .. } => false,
            InsKind::ExtractFlag { .. } => false,
            InsKind::Memcpy { .. } => true,
            InsKind::Memset { .. } => true,
            // TODO: is it correct?
            InsKind::X87InitStack { .. } => true,
            InsKind::X87Push { .. } => true,
            InsKind::X87Pop { .. } => true,
            InsKind::X87Peek { .. } => true,
            InsKind::X87StatusWord { .. } => false,
        }
    }

    pub fn visit_result_values(&self, mut callback: impl FnMut(ValueId)) {
        match &self.kind {
            InsKind::Hole => {}
            InsKind::Uninit { dst } => callback(*dst),
            InsKind::BinOp { dst, flags, .. } => {
                callback(*dst);
                if let Some(flags) = flags {
                    callback(*flags)
                }
            }
            InsKind::UnOp { op: _, dst, src: _ } => callback(*dst),
            InsKind::Unimpl { dst } => callback(*dst),
            InsKind::Load { dst, .. } => callback(*dst),
            InsKind::Cond { dst, .. } => callback(*dst),
            InsKind::Store { dst_mem, .. } => callback(*dst_mem),
            InsKind::Call { result, .. } => result.values().for_each(callback),
            InsKind::Extract { dst, .. } => callback(*dst),
            InsKind::Insert { dst, .. } => callback(*dst),
            InsKind::Cast { dst, .. } => callback(*dst),
            InsKind::ExtractFlag { dst, .. } => callback(*dst),
            InsKind::Memset {
                dst_mem,
                src_mem: _,
                addr: _,
                value: _,
                count: _,
            } => callback(*dst_mem),
            InsKind::Memcpy {
                dst_mem,
                src_mem: _,
                dst_addr: _,
                src_addr: _,
                count: _,
                size: _,
            } => callback(*dst_mem),
            InsKind::X87InitStack { dst } => callback(*dst),
            InsKind::X87Push {
                dst_stack,
                dst_flags,
                src_stack: _,
                value: _,
            } => {
                if let Some(x) = dst_flags {
                    callback(*x);
                }
                callback(*dst_stack)
            }
            InsKind::X87Pop {
                dst_stack,
                dst_flags,
                src_stack: _,
                dst,
            } => {
                callback(*dst_stack);
                if let Some(x) = dst_flags {
                    callback(*x);
                }
                if let Some(dst) = dst {
                    callback(*dst);
                }
            }
            InsKind::X87Peek {
                dst,
                stack: _,
                idx: _,
            } => {
                callback(*dst);
            }
            InsKind::X87StatusWord { dst, flags: _ } => callback(*dst),
        }
    }

    pub fn instr_result(&self) -> heapless::Vec<ValueId, 16> {
        let mut res = heapless::Vec::<ValueId, 16>::new();
        self.visit_result_values(|v| res.push(v).unwrap());
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
            InsKind::UnOp { op, dst, src } => write!(f, "{dst} = {op} {src}"),
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
            InsKind::Extract { dst, src, offset } => {
                write!(f, "{dst} = extract {src}[{offset}..]")
            }
            InsKind::Insert {
                dst,
                base,
                value,
                offset,
            } => {
                write!(f, "{dst} = insert {base}[{offset}..] <- {value}")
            }
            InsKind::Cast { dst, src } => write!(f, "{dst} = cast {src}"),
            InsKind::ExtractFlag { dst, src, flag } => write!(f, "{dst} = flag.{flag} {src}"),
            InsKind::Memset {
                dst_mem,
                src_mem,
                addr,
                value,
                count,
            } => {
                write!(
                    f,
                    "{dst_mem} = __memset {src_mem} ({addr}, {value}, {count})"
                )
            }
            InsKind::Memcpy {
                dst_mem,
                src_mem,
                dst_addr,
                src_addr,
                count,
                size,
            } => {
                write!(
                    f,
                    "{dst_mem} = __memcpy {src_mem} ({dst_addr}, {src_addr}, {size}:{count})",
                )
            }
            InsKind::X87InitStack { dst } => write!(f, "{dst} = x87.init"),
            InsKind::X87Push {
                dst_stack,
                dst_flags,
                src_stack,
                value,
            } => write!(
                f,
                "{dst_stack}, {} = x87.push {src_stack} {value}",
                MaybeValueFmt(*dst_flags)
            ),
            InsKind::X87Pop {
                dst_stack,
                dst_flags,
                src_stack,
                dst,
            } => write!(
                f,
                "{dst_stack}, {}, {} = x87.pop {src_stack}",
                MaybeValueFmt(*dst_flags),
                MaybeValueFmt(*dst)
            ),
            InsKind::X87Peek { dst, stack, idx } => write!(f, "{dst} = x87.peek {stack}[{idx}]"),
            InsKind::X87StatusWord { dst, flags } => write!(f, "{dst} = x87.status_word {flags}"),
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
pub struct BranchTarget {
    pub block: BlockId,
    pub args: Vec<ValueId>,
}

impl std::fmt::Display for BranchTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.block, FmtList(&self.args))
    }
}

#[derive(Debug, Clone)]
pub enum TerminatorKind {
    Jump(JumpTarget),
    Brif {
        cond: ValueId,
        thenb: BranchTarget,
        elseb: BranchTarget,
    },
    Ret {
        adjust: u16,
        args: IoValues,
    },
    JumpTable {
        jump_addr: ValueId,
        entries: Vec<JumpTableEntry>,
    },
}

#[derive(Debug, Clone)]
pub struct JumpTableEntry {
    pub target: BlockId,
    pub args: Vec<ValueId>,
}

impl std::fmt::Display for JumpTableEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.target, FmtList(&self.args))
    }
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
            Self::JumpTable { jump_addr, entries } => {
                write!(f, "jumptable {jump_addr} {}", FmtList(entries))
            }
        }
    }
}

impl Terminator {
    fn visit_jump_target(jp: &JumpTarget, mut callback: impl FnMut(ValueId)) {
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
                Self::visit_jump_target(jp, callback);
            }
            TerminatorKind::Brif { cond, thenb, elseb } => {
                callback(*cond);
                thenb.args.iter().for_each(|v| callback(*v));
                elseb.args.iter().for_each(|v| callback(*v));
            }
            TerminatorKind::Ret { adjust: _, args } => {
                args.values().for_each(callback);
            }
            TerminatorKind::JumpTable { jump_addr, entries } => {
                callback(*jump_addr);
                entries
                    .iter()
                    .flat_map(|e| &e.args)
                    .for_each(|v| callback(*v));
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
                thenb.args.iter_mut().for_each(|v| callback(v));
                elseb.args.iter_mut().for_each(|v| callback(v));
            }
            TerminatorKind::Ret { adjust: _, args } => {
                args.values_mut().map(callback).count();
            }
            TerminatorKind::JumpTable { jump_addr, entries } => {
                callback(jump_addr);
                entries
                    .iter_mut()
                    .flat_map(|e| &mut e.args)
                    .for_each(|v| callback(v));
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
                callback(thenb.block, &mut thenb.args);
                callback(elseb.block, &mut elseb.args);
            }
            TerminatorKind::Ret { .. } => {}
            TerminatorKind::JumpTable {
                jump_addr: _,
                entries,
            } => {
                entries
                    .iter_mut()
                    .for_each(|e| callback(e.target, &mut e.args));
            }
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
                callback(thenb.block, &thenb.args);
                callback(elseb.block, &elseb.args);
            }
            TerminatorKind::Ret { .. } => {}
            TerminatorKind::JumpTable {
                jump_addr: _,
                entries,
            } => {
                entries.iter().for_each(|e| callback(e.target, &e.args));
            }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

struct MaybeValueFmt(Option<ValueId>);

impl std::fmt::Display for MaybeValueFmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(x) => x.fmt(f),
            None => write!(f, "_"),
        }
    }
}
