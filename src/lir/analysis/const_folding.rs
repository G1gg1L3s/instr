use crate::lir::{
    func::SsaFunction,
    ins::{BinOp, Ins, InsKind},
    value::{Imm, Value, Values},
};

pub fn run(func: &mut SsaFunction) -> bool {
    log::trace!("> Folding constants on {}", func.addr);
    let mut changed = false;

    for (_, ins) in func.ins.iter_mut() {
        changed |= fold_ins(ins, &mut func.values);
    }

    changed
}

fn fold_ins(ins: &mut Ins, values: &mut Values) -> bool {
    let InsKind::BinOp {
        op,
        dst,
        lhs,
        rhs,
        flags: None,
    } = &ins.kind
    else {
        return false;
    };

    if lhs == rhs && matches!(op, BinOp::Xor | BinOp::Sub) {
        let Some(imm) = values.val_ty(*lhs).map(|ty| ty.imm(0)) else {
            log::warn!("Cannot perform optimisation on {lhs} {op} {rhs} because lhs has no type");
            return false;
        };
        log::trace!(">> Folding {dst} = {lhs} {op} {rhs} into {dst} = {imm}");
        values[*dst] = Value::Imm(imm);
        ins.kind = InsKind::Hole;
        return true;
    }

    let lhs_val = &values[*lhs];
    let rhs_val = &values[*rhs];

    if let (Value::Imm(lhs_imm), Value::Imm(rhs_imm)) = (lhs_val, rhs_val) {
        if let Some(res) = compute_const(*op, *lhs_imm, *rhs_imm) {
            log::trace!(">> Folding {dst} = {lhs} {op} {rhs} into {dst} = {res}");
            values[*dst] = Value::Imm(res);
            ins.kind = InsKind::Hole;
            return true;
        }
    };

    if let (BinOp::BitOr, Some(res)) = (*op, to_0xff_imm(lhs_val).or(to_0xff_imm(rhs_val))) {
        log::info!(">> Folding {dst} = {lhs} {op} {rhs} into {dst} = {res}");
        values[*dst] = Value::Imm(res);
        ins.kind = InsKind::Hole;
        return true;
    }

    false
}

fn to_0xff_imm(imm: &Value) -> Option<Imm> {
    if let Value::Imm(imm) = imm {
        to_0xff(*imm)
    } else {
        None
    }
}

fn to_0xff(imm: Imm) -> Option<Imm> {
    match imm {
        Imm::U8(x) => (x == 0xff).then_some(Imm::U8(0xff)),
        Imm::U16(x) => (x == 0xffff).then_some(Imm::U16(0xffff)),
        Imm::U32(x) => (x == 0xffffffff).then_some(Imm::U32(0xffffffff)),
    }
}

macro_rules! match_bin_imm {
    (
        ($lhs:expr, $rhs:expr),
        ($x:ident, $y:ident) => $body:expr
    ) => {{
        match ($lhs, $rhs) {
            (Imm::U8($x), Imm::U8($y)) => Imm::U8($body),
            (Imm::U16($x), Imm::U16($y)) => Imm::U16($body),
            (Imm::U32($x), Imm::U32($y)) => Imm::U32($body),
            _ => panic!("mismatched immediate sizes"),
        }
    }};
}

pub(crate) use match_bin_imm;

fn compute_const(op: BinOp, lhs: Imm, rhs: Imm) -> Option<Imm> {
    Some(match_bin_imm!((lhs, rhs), (x, y) => match op {
        BinOp::Add => x.wrapping_add(y),
        BinOp::Sub => x.wrapping_sub(y),
        BinOp::Mulu => return None,
        BinOp::Muls => return None,
        BinOp::Div => x / y,
        BinOp::Xor => x ^ y,
        BinOp::BitAnd => x & y,
        BinOp::BitOr => x | y,
        BinOp::ShiftLeft => x << y,
        BinOp::ShifRight => x >> y,
        BinOp::ShifArithRight => return None,
        BinOp::Condition(_condition) => return None,
    }))
}
