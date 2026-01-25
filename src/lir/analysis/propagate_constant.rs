use crate::lir::{
    analysis::inverse_map,
    func::SsaFunction,
    ins::{BinOp, Ins},
    value::{Imm, Value},
};

pub fn exec(func: &mut SsaFunction) {
    let mut val_to_ins = inverse_map::compute_value_dest(func);

    for block in func.blocks.values_mut() {
        let mut ins_idx = 0;
        while ins_idx < block.ins.len() {
            let ins_id = block.ins[ins_idx];
            let ins = &func.ins[ins_id];

            match ins {
                &Ins::BinOp {
                    op,
                    dst,
                    lhs,
                    rhs,
                    flags,
                } if op == BinOp::Xor && lhs == rhs => {
                    log::trace!("> Optimizing {lhs} xor {rhs} at {}", block.addr);
                    block.ins.remove(ins_idx);
                    let Some(ty) = func.values.val_ty(lhs) else {
                        log::warn!(
                            "Cannot optimise {lhs} xor {rhs} because cannot compute type of {:?}",
                            &func.values[lhs]
                        );
                        // TODO: this is so ugly
                        ins_idx += 1;
                        continue;
                    };
                    func.values[dst] = Value::Imm(ty.imm(0));
                    func.ins[ins_id] = Ins::Hole;
                    val_to_ins.insert(dst, inverse_map::ValueSource::Imm(ty.imm(0)));

                    if let Some(flags) = flags {
                        func.values[flags] = Value::Todo;
                        val_to_ins.remove(&flags);
                    }

                    continue;
                }

                &Ins::BinOp {
                    op,
                    dst,
                    lhs: lhs_val_id,
                    rhs: rhs_val_id,
                    flags,
                } => {
                    let lhs = &func.values[lhs_val_id];
                    let rhs = &func.values[rhs_val_id];
                    let (Value::Imm(lhs), Value::Imm(rhs)) = (lhs, rhs) else {
                        ins_idx += 1;
                        continue;
                    };

                    if let Some(result) = compute_const(op, *lhs, *rhs) {
                        log::trace!("> Optimizing {lhs} {op} {rhs} at {}", block.addr);
                        block.ins.remove(ins_idx);
                        func.values[dst] = Value::Imm(result);
                        func.ins[ins_id] = Ins::Hole;
                        val_to_ins.insert(dst, inverse_map::ValueSource::Imm(result));

                        if let Some(flags) = flags {
                            func.values[flags] = Value::Todo;
                            val_to_ins.remove(&flags);
                        }
                    }
                }
                _ => {}
            }

            ins_idx += 1;
        }
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
