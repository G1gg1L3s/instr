use std::collections::HashMap;

use crate::lir::{
    analysis::def_use,
    func::SsaFunction,
    ins::{BinOp, InsId, InsKind, Instrs},
    value::{Imm, Value, ValueId, Values},
};

pub fn run(func: &mut SsaFunction) -> bool {
    log::trace!("> Reassociate pass on {}", func.addr);
    let def_use = def_use::compute(func);
    let mut changed = false;
    for ins_id in func.ins.keys() {
        changed |= reassoc_ins(ins_id, &mut func.ins, &mut func.values, &def_use);
    }
    changed
}

fn reassoc_ins(
    ins_id: InsId,
    ins: &mut Instrs,
    values: &mut Values,
    def_use: &HashMap<ValueId, def_use::ValueSource>,
) -> bool {
    let &InsKind::BinOp {
        op: op2 @ (BinOp::Add | BinOp::Sub),
        dst,
        lhs,
        rhs,
        flags: None,
    } = &ins[ins_id].kind
    else {
        return false;
    };

    let Some((base2, c2, imm_val_id)) = to_base_and_imm(values, lhs, op2, rhs) else {
        return false;
    };

    let (Signed::Pos(base_val) | Signed::Neg(base_val)) = base2;

    let def_ins = match def_use.get(&base_val) {
        Some(def_use::ValueSource::Ins(ins_id)) => *ins_id,
        Some(def_use::ValueSource::Param { .. }) => {
            return false;
        }
        _ => return false,
    };

    let &InsKind::BinOp {
        op: op1 @ (BinOp::Add | BinOp::Sub),
        lhs: a,
        rhs: b,
        flags: None,
        dst: dst_1,
    } = &ins[def_ins].kind
    else {
        return false;
    };

    let Some((base1, c1, _)) = to_base_and_imm(values, a, op1, b) else {
        return false;
    };

    log::trace!(">> base1: {base1:?}, c1: {c1:?}, base2: {base2:?}, c2: {c2:?}");

    let (base1, c1) = if let Signed::Neg(_) = base2 {
        (base1.neg(), c1.neg())
    } else {
        (base1, c1)
    };

    let combined = merge_assoc(c1, c2);

    log::trace!(">> combied: {combined:?}");

    let (res_op, res_lhs, res_rhs) = match (base1, combined) {
        (Signed::Pos(base), Signed::Pos(imm)) => {
            values[imm_val_id] = Value::Imm(imm);
            (BinOp::Add, base, imm_val_id)
        }
        (Signed::Pos(base), Signed::Neg(imm)) => {
            values[imm_val_id] = Value::Imm(imm);
            (BinOp::Sub, base, imm_val_id)
        }
        (Signed::Neg(base), Signed::Pos(imm)) => {
            values[imm_val_id] = Value::Imm(imm);
            (BinOp::Sub, imm_val_id, base)
        }
        (Signed::Neg(_), Signed::Neg(_)) => return false,
    };

    log::trace!(
        ">> Optimising: {dst} = {lhs} {op2} {rhs} and {dst_1} = {a} {op1} {b} => {dst} = {res_lhs} {res_op} {res_rhs}"
    );

    ins[ins_id].kind = InsKind::BinOp {
        op: res_op,
        dst,
        lhs: res_lhs,
        rhs: res_rhs,
        flags: None,
    };

    true
}

#[derive(Debug)]
enum Signed<T> {
    Pos(T),
    Neg(T),
}

impl<T> Signed<T> {
    fn neg(self) -> Signed<T> {
        match self {
            Signed::Pos(x) => Signed::Neg(x),
            Signed::Neg(x) => Signed::Pos(x),
        }
    }
}

fn to_base_and_imm(
    values: &Values,
    lhs: ValueId,
    op: BinOp,
    rhs: ValueId,
) -> Option<(Signed<ValueId>, Signed<Imm>, ValueId)> {
    use Signed::*;

    let lhs_val = &values[lhs];
    let rhs_val = &values[rhs];

    let (base, imm, imm_val_id) = match (lhs_val, op, rhs_val) {
        (Value::Imm(c), BinOp::Add, _) => (Pos(rhs), Pos(*c), lhs),
        (Value::Imm(c), BinOp::Sub, _) => (Neg(rhs), Pos(*c), lhs),

        (_, BinOp::Add, Value::Imm(c)) => (Pos(lhs), Pos(*c), rhs),
        (_, BinOp::Sub, Value::Imm(c)) => (Pos(lhs), Neg(*c), rhs),

        _ => return None,
    };
    Some((base, imm, imm_val_id))
}

#[derive(Debug, PartialEq)]
enum AssocImm {
    U8(u8),
    U16(u16),
    U32(u32),
}

impl From<Imm> for AssocImm {
    fn from(value: Imm) -> Self {
        match value {
            Imm::Bool(_) => todo!(),
            Imm::U8(x) => Self::U8(x),
            Imm::U16(x) => Self::U16(x),
            Imm::U32(x) => Self::U32(x),
        }
    }
}

impl From<AssocImm> for Imm {
    fn from(value: AssocImm) -> Self {
        match value {
            AssocImm::U8(x) => Self::U8(x),
            AssocImm::U16(x) => Self::U16(x),
            AssocImm::U32(x) => Self::U32(x),
        }
    }
}

impl AssocImm {
    fn add(self, rhs: Self) -> Self {
        match (self, rhs) {
            (AssocImm::U8(a), AssocImm::U8(b)) => AssocImm::U8(a.wrapping_add(b)),
            (AssocImm::U16(a), AssocImm::U16(b)) => AssocImm::U16(a.wrapping_add(b)),
            (AssocImm::U32(a), AssocImm::U32(b)) => AssocImm::U32(a.wrapping_add(b)),
            (a, b) => panic!("types not equal: {a:?} vs {b:?}"),
        }
    }

    fn sub(self, rhs: Self) -> Self {
        match (self, rhs) {
            (AssocImm::U8(a), AssocImm::U8(b)) => AssocImm::U8(a - b),
            (AssocImm::U16(a), AssocImm::U16(b)) => AssocImm::U16(a - b),
            (AssocImm::U32(a), AssocImm::U32(b)) => AssocImm::U32(a - b),
            (a, b) => panic!("types not equal: {a:?} vs {b:?}"),
        }
    }
}

impl std::cmp::PartialOrd for AssocImm {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (AssocImm::U8(a), AssocImm::U8(b)) => a.partial_cmp(b),
            (AssocImm::U16(a), AssocImm::U16(b)) => a.partial_cmp(b),
            (AssocImm::U32(a), AssocImm::U32(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

fn merge_assoc(c1: Signed<Imm>, c2: Signed<Imm>) -> Signed<Imm> {
    match (c1, c2) {
        (Signed::Pos(x), Signed::Pos(y)) => {
            let sum = AssocImm::from(x).add(AssocImm::from(y));
            Signed::Pos(sum.into())
        }
        (Signed::Pos(x), Signed::Neg(y)) => {
            let x = AssocImm::from(x);
            let y = AssocImm::from(y);
            if x < y {
                Signed::Neg(y.sub(x).into())
            } else {
                Signed::Pos(x.sub(y).into())
            }
        }
        (Signed::Neg(x), Signed::Pos(y)) => {
            let x = AssocImm::from(x);
            let y = AssocImm::from(y);
            if y > x {
                Signed::Pos(y.sub(x).into())
            } else {
                Signed::Neg(x.sub(y).into())
            }
        }
        (Signed::Neg(x), Signed::Neg(y)) => {
            let sum = AssocImm::from(x).add(AssocImm::from(y));
            Signed::Neg(sum.into())
        }
    }
}
