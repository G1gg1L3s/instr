use crate::lir::{
    analysis::def_use::{self, ValueSource},
    func::SsaFunction,
    ins::{BinOp, Condition, InsKind},
    value::{Value, ValueId},
};

pub fn exec(func: &mut SsaFunction) {
    let val_to_ins = def_use::compute(func);

    let mut worklist = vec![];

    enum Arg {
        Val(ValueId),
        Imm(u8),
    }

    for (_, block) in func.blocks.iter_mut() {
        for &ins_id in block.ins.iter() {
            let ins = &func.ins[ins_id];
            let &InsKind::Cond { dst, flags, cond } = &ins.kind else {
                continue;
            };

            let Some(ValueSource::Ins(flags_source_ins)) = val_to_ins.get(&flags).copied() else {
                log::error!(
                    "No instruction entry found for flags {flags} in {}",
                    block.addr
                );
                continue;
            };

            let &InsKind::BinOp {
                op,
                dst: bin_res,
                lhs,
                rhs,
                flags: flags_dst,
            } = &func.ins[flags_source_ins].kind
            else {
                continue;
            };

            assert_eq!(flags_dst.as_ref(), Some(&flags));

            match (cond, op) {
                (c, BinOp::Sub) => {
                    worklist.push((ins_id, c, lhs, Arg::Val(rhs), dst));
                }
                (
                    Condition::Equal
                    | Condition::NotEqual
                    | Condition::SignLess
                    | Condition::SignedLessEqual
                    | Condition::SignedGreater
                    | Condition::SignedGreaterEqual,
                    BinOp::BitAnd,
                ) if lhs == rhs => {
                    worklist.push((ins_id, cond, lhs, Arg::Imm(0), dst));
                }

                (Condition::Equal, BinOp::Add) => {
                    worklist.push((ins_id, cond, bin_res, Arg::Imm(0), dst));
                }
                _ => {}
            }
        }
    }

    while let Some((ins_id, cond, lhs, rhs, dst)) = worklist.pop() {
        let rhs = match rhs {
            Arg::Val(v) => v,
            Arg::Imm(x) => {
                let Some(ty) = func.val_ty(lhs) else {
                    log::warn!(">> Cannot derive type of {lhs}, skipping");
                    continue;
                };
                let imm = ty.imm(x);
                func.values.add(Value::Imm(imm))
            }
        };

        func.ins[ins_id].kind = InsKind::BinOp {
            op: BinOp::Condition(cond),
            dst,
            lhs,
            rhs,
            flags: None,
        };
    }
}
