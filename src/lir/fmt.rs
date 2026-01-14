use std::{collections::HashMap, fmt::Display};

use crate::lir::{
    block::Block,
    flags::FlagsGroup,
    func::SsaFunction,
    ins::{InsId, JumpTarget, Terminator},
    ty::Ty,
    value::{Value, ValueId},
};

use super::ins::Ins;

pub struct FmtList<'a, T>(pub &'a [T]);

impl<'a, T: std::fmt::Display> std::fmt::Display for FmtList<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() > 0 {
            write!(f, "(")?;
            for (i, arg) in self.0.iter().enumerate() {
                if i != 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{arg}")?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FuncFmt<'a> {
    func: &'a SsaFunction,
}

pub struct InsFmt<'a> {
    fmt: FuncFmt<'a>,
    ins: InsId,
}

pub struct ValueFmt<'a> {
    fmt: FuncFmt<'a>,
    val: ValueId,
}

pub struct ValuesFmt<'a> {
    fmt: FuncFmt<'a>,
    vals: &'a [ValueId],
}

pub struct TyFmt {
    ty: Option<Ty>,
}

impl<'a> Display for InsFmt<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ins = &self.fmt.func.ins[self.ins];
        match ins {
            Ins::BinOp {
                op,
                dst,
                lhs,
                rhs,
                flags,
            } => {
                let dst = self.fmt.val(*dst);
                let lhs = self.fmt.val(*lhs);
                let rhs = self.fmt.val(*rhs);

                if let Some(flags) = flags {
                    let flags = self.fmt.val(*flags);
                    write!(f, "{dst}, {flags} = {lhs} {op} {rhs}")
                } else {
                    write!(f, "{dst} = {lhs} {op} {rhs}")
                }
            }
            Ins::Uninit { dst } => {
                write!(f, "{} = ???", self.fmt.val(*dst),)
            }
            Ins::Const { dst, val } => write!(f, "{} = const {}", self.fmt.val(*dst), val),
            Ins::Unimpl { dst } => write!(f, "{} = unimplemented", self.fmt.val(*dst)),
            Ins::Load {
                dst,
                addr,
                mem,
                space,
            } => {
                write!(
                    f,
                    "{} = load.{} {} {space}[{}]",
                    self.fmt.val(*dst),
                    self.fmt.ty(*dst),
                    self.fmt.val(*mem),
                    self.fmt.val(*addr)
                )
            }
            Ins::Store {
                dst_mem,
                src_mem,
                addr,
                value,
                space,
            } => {
                write!(
                    f,
                    "{} = store.{} {} {}[{}] <- {}",
                    self.fmt.val(*dst_mem),
                    self.fmt.ty(*value),
                    self.fmt.val(*src_mem),
                    space,
                    self.fmt.val(*addr),
                    self.fmt.val(*value)
                )
            }
            Ins::Cond {
                dst,
                flags: src,
                cond,
            } => write!(
                f,
                "{} = cond({cond}) {}",
                self.fmt.val(*dst),
                self.fmt.val(*src)
            ),
            Ins::Call {
                result,
                target,
                args,
            } => write!(
                f,
                "{} = call {}{}",
                self.fmt.vals(result),
                target,
                self.fmt.vals(args)
            ),
        }
    }
}

impl<'a> Display for ValueFmt<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = &self.fmt.func.values[self.val];
        match val {
            Value::Invalid => write!(f, "invalid{}", self.val.id()),
            Value::Temp {
                ty: Ty::Flags(flags),
            } => write!(f, "{}#{}", FlagsGroup::new(*flags), self.val.id()),
            Value::Temp { .. } | Value::Alias { .. } => write!(f, "{}", self.val),
            Value::Mem => write!(f, "mem{}", self.val.id()),
        }
    }
}

impl<'a> Display for ValuesFmt<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.vals.len() > 0 {
            write!(f, "(")?;
            for (i, arg) in self.vals.iter().enumerate() {
                if i != 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", self.fmt.val(*arg))?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl Display for TyFmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.ty {
            Some(x) => write!(f, "{x}"),
            None => write!(f, "???"),
        }
    }
}

impl<'a> FuncFmt<'a> {
    pub fn new(func: &'a SsaFunction) -> Self {
        Self { func }
    }

    pub fn ins(self, ins: InsId) -> InsFmt<'a> {
        InsFmt { fmt: self, ins }
    }

    pub fn val(self, val: ValueId) -> ValueFmt<'a> {
        ValueFmt { fmt: self, val }
    }

    pub fn vals(self, vals: &'a [ValueId]) -> ValuesFmt<'a> {
        ValuesFmt { fmt: self, vals }
    }

    pub fn ty(self, val: ValueId) -> TyFmt {
        TyFmt {
            ty: self.func.val_ty(val),
        }
    }
}

impl<'a> Display for FuncFmt<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "function {}():", self.func.addr)?;

        let aliases = self.func.inverse_aliases();

        for (_, b) in self.func.blocks.iter() {
            fmt_block(*self, &aliases, b, f)?;
        }

        Ok(())
    }
}

fn fmt_block(
    fmt: FuncFmt<'_>,
    aliases: &HashMap<ValueId, Vec<ValueId>>,
    block: &Block,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(
        f,
        "{}{}:  # {}",
        block.id,
        fmt.vals(&block.params),
        block.addr
    )?;

    for param in &block.params {
        maybe_fmt_alias(fmt, aliases, f, *param)?;
    }

    for ins in &block.ins {
        writeln!(f, "    {}", fmt.ins(*ins))?;
        for result in fmt.func.instr_result(*ins) {
            maybe_fmt_alias(fmt, aliases, f, result)?;
        }
    }

    if let Some(term) = &block.terminator {
        write!(f, "    ")?;
        fmt_terminator(fmt, term, f)?;
        writeln!(f)?;
    } else {
        writeln!(f, "    <no terminator>")?;
    }

    writeln!(f)?;
    Ok(())
}

fn fmt_terminator(
    fmt: FuncFmt<'_>,
    term: &Terminator,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    match term {
        Terminator::Jump(target) => {
            write!(f, "jump ")?;
            format_target(fmt, f, target)?;
            Ok(())
        }
        Terminator::Brif { cond, thenb, elseb } => {
            write!(f, "brif {} then ", fmt.val(*cond))?;
            format_target(fmt, f, thenb)?;
            write!(f, " else ")?;
            format_target(fmt, f, elseb)?;
            Ok(())
        }
        Terminator::Ret { adjust, args } => {
            if args.len() == 0 {
                write!(f, "ret stack:{adjust}")
            } else {
                write!(f, "ret {} stack:{}", fmt.vals(args), adjust)
            }
        }
    }
}

fn format_target(
    fmt: FuncFmt<'_>,
    f: &mut std::fmt::Formatter<'_>,
    target: &JumpTarget,
) -> Result<(), std::fmt::Error> {
    Ok(match target {
        JumpTarget::Known { block, args } => {
            write!(f, "{}{}", block, fmt.vals(args))?;
        }
        JumpTarget::Unknown { addr } => {
            write!(f, "?{}", fmt.val(*addr))?;
        }
    })
}

fn maybe_fmt_alias(
    fmt: FuncFmt<'_>,
    aliases: &HashMap<ValueId, Vec<ValueId>>,
    f: &mut std::fmt::Formatter<'_>,
    result: ValueId,
) -> Result<(), std::fmt::Error> {
    Ok(if let Some(aliases) = aliases.get(&result) {
        for alias in aliases {
            writeln!(f, "    {} -> {}", fmt.val(*alias), fmt.val(result))?;
        }
    })
}
