use std::{collections::HashMap, fmt::Display};

use crate::lir::{
    block::Block,
    func::SsaFunction,
    ins::InsId,
    value::{Value, ValueId},
};

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

impl<'a> Display for InsFmt<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ins = &self.fmt.func.ins[self.ins];
        match ins {
            super::ins::Ins::BinOp { op, dst, lhs, rhs } => {
                write!(
                    f,
                    "{} = {} {} {}",
                    self.fmt.val(*dst),
                    self.fmt.val(*lhs),
                    op,
                    self.fmt.val(*rhs),
                )
            }
            super::ins::Ins::Uninit { dst } => {
                write!(f, "{} = ???", self.fmt.val(*dst),)
            }
            super::ins::Ins::Unimpl => write!(f, "unimplemented"),
        }
    }
}

impl<'a> Display for ValueFmt<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = &self.fmt.func.values[self.val];
        match val {
            Value::Invalid => write!(f, "invalid{}", self.val.id()),
            Value::Temp { .. } | Value::Alias { .. } => write!(f, "{}", self.val),
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
    writeln!(f, "{}{}:", block.id, fmt.vals(&block.params))?;

    for param in &block.params {
        maybe_fmt_alias(fmt, aliases, f, *param)?;
    }

    for ins in &block.ins {
        writeln!(f, "    {}", fmt.ins(*ins))?;
        if let Some(result) = fmt.func.instr_result(*ins) {
            maybe_fmt_alias(fmt, aliases, f, result)?;
        }
    }
    writeln!(f)?;
    Ok(())
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
