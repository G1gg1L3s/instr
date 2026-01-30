use crate::lir::{
    func::SsaFunction,
    ins::{BinOp, Ins, InsKind},
    ty::Ty,
    value::{Value, ValueId, Values},
};
use std::collections::HashMap;

#[derive(Hash, PartialEq, Eq)]
enum ExprKey {
    BinOp {
        op: BinOp,
        lhs: ValueId,
        rhs: ValueId,
    },
    Extract {
        src: ValueId,
        offset: u8,
        ty: Ty,
    },
}

fn canonicalize_binop(op: BinOp, lhs: ValueId, rhs: ValueId) -> (ValueId, ValueId) {
    if is_commutative(op) && lhs.id() > rhs.id() {
        (rhs, lhs)
    } else {
        (lhs, rhs)
    }
}

fn is_commutative(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Add | BinOp::Xor | BinOp::BitAnd | BinOp::BitOr | BinOp::Mulu | BinOp::Muls
    )
}

pub fn run(func: &mut SsaFunction) -> bool {
    let mut expr_table: HashMap<ExprKey, ValueId> = HashMap::new();
    let mut changed = false;

    for ins in func.ins.values_mut() {
        if let Some(key_dst) = make_expr_key(ins, &func.values) {
            let (key, dst) = key_dst;

            if let Some(&prev) = expr_table.get(&key) {
                log::trace!("> [func {}] Replacing {dst} with {prev}", func.addr);

                func.values[dst] = Value::Alias { to: prev };
                ins.kind = InsKind::Hole;
                changed = true;
            } else {
                expr_table.insert(key, dst);
            }
        }
    }

    changed
}

fn make_expr_key(ins: &Ins, values: &Values) -> Option<(ExprKey, ValueId)> {
    match &ins.kind {
        InsKind::BinOp {
            op,
            dst,
            lhs,
            rhs,
            flags: None,
        } => {
            let lhs = values.resolve_alias(*lhs);
            let rhs = values.resolve_alias(*rhs);

            let (lhs, rhs) = canonicalize_binop(*op, lhs, rhs);

            let key = ExprKey::BinOp { op: *op, lhs, rhs };

            Some((key, *dst))
        }

        InsKind::Extract { dst, src, offset } => {
            let src = values.resolve_alias(*src);
            let ty = values.val_ty(*dst)?;
            let key = ExprKey::Extract {
                src,
                offset: *offset,
                ty,
            };
            Some((key, *dst))
        }

        _ => None,
    }
}
