use std::collections::HashMap;

use crate::lir::{func::SsaFunction, ins::InsId, value::ValueId};

pub fn compute_value_dest(func: &SsaFunction) -> HashMap<ValueId, InsId> {
    let mut res = HashMap::new();
    for block in func.blocks.values() {
        for ins in block.ins.iter().copied() {
            let values = func.instr_result(ins);
            for value in values {
                let old = res.insert(value, ins);
                if let Some(old) = old {
                    panic!("value {old} is defined twice")
                }
            }
        }
    }
    res
}
