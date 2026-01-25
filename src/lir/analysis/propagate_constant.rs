use crate::lir::{
    analysis::inverse_map,
    func::SsaFunction,
    ins::{BinOp, Ins},
    value::Value,
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
                _ => {}
            }

            ins_idx += 1;
        }
    }
}
