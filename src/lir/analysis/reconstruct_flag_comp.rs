use crate::lir::{
    analysis::def_use::{self, ValueSource},
    flags::Flag,
    func::SsaFunction,
    ins::{BinOp, Condition, InsKind},
    value::{Imm, Value},
};

pub fn exec(func: &mut SsaFunction) -> bool {
    let mut changed = false;
    let val_to_ins = def_use::compute(func);

    for ins_id in func.ins.keys() {
        let ins = &func.ins[ins_id];
        let &InsKind::ExtractFlag { dst, src, flag } = &ins.kind else {
            continue;
        };

        let Some(ValueSource::Ins(flags_source_ins)) = val_to_ins.get(&src).copied() else {
            log::error!(
                "No instruction entry found for flags {src} in {} ({})",
                func.addr,
                ins_id
            );
            continue;
        };

        let &InsKind::BinOp {
            op,
            dst: _,
            lhs,
            rhs,
            flags: flags_dst,
        } = &func.ins[flags_source_ins].kind
        else {
            continue;
        };

        assert_eq!(flags_dst.as_ref(), Some(&src));

        match (op, flag) {
            (BinOp::Sub, Flag::Zero) => {
                log::trace!("> Replacing {ins} with equality binary operation");
                func.ins[ins_id].kind = InsKind::BinOp {
                    op: BinOp::Condition(Condition::Equal),
                    dst,
                    lhs,
                    rhs,
                    flags: None,
                };
                changed = true;
            }
            (BinOp::BitAnd, Flag::Zero) if lhs == rhs => {
                log::trace!("> Replacing {ins} with zero");
                func.ins[ins_id].kind = InsKind::Hole;
                func.values[dst] = Value::Imm(Imm::Bool(false));
                changed = true;
            }
            _ => {}
        }
    }

    changed
}
