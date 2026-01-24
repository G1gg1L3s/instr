use crate::lir::{
    analysis::inverse_map,
    func::SsaFunction,
    ins::{BinOp, Ins},
    value::ValueId,
};

pub fn exec(func: &mut SsaFunction) {
    let val_to_ins = inverse_map::compute_value_dest(func);

    let mut worklist = vec![];

    enum Arg {
        Val(ValueId),
    }

    for (_, block) in func.blocks.iter_mut() {
        for (_, &ins_id) in block.ins.iter().enumerate() {
            let ins = &func.ins[ins_id];
            let &Ins::Cond { dst, flags, cond } = ins else {
                continue;
            };

            let Some(flags_source_ins) = val_to_ins.get(&flags).copied() else {
                log::error!("No entry found for flags {flags} in {}", block.addr);
                continue;
            };
            let &Ins::BinOp {
                op,
                dst: _,
                lhs,
                rhs,
                flags: flags_dst,
            } = &func.ins[flags_source_ins]
            else {
                continue;
            };

            assert_eq!(flags_dst.as_ref(), Some(&flags));

            match (cond, op) {
                (c, BinOp::Sub) => {
                    worklist.push((ins_id, c, Arg::Val(lhs), Arg::Val(rhs), dst));
                }
                _ => {}
            }
        }
    }

    while let Some((ins_id, cond, lhs, rhs, dst)) = worklist.pop() {
        let lhs = match lhs {
            Arg::Val(v) => v,
        };
        let rhs = match rhs {
            Arg::Val(v) => v,
        };

        func.ins[ins_id] = Ins::BinOp {
            op: BinOp::Condition(cond),
            dst,
            lhs,
            rhs,
            flags: None,
        };
    }
}
