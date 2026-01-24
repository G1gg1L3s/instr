use std::collections::HashMap;

use crate::lir::{block::BlockId, func::SsaFunction, ins::InsId, value::ValueId};

#[derive(Debug, Clone, Copy)]
pub enum ValueSource {
    Ins(InsId),
    Param { block: BlockId, idx: usize },
}

pub fn compute_value_dest(func: &SsaFunction) -> HashMap<ValueId, ValueSource> {
    let mut res = HashMap::new();
    for block in func.blocks.values() {
        for (idx, value) in block.params.iter().enumerate() {
            let current = ValueSource::Param {
                block: block.id,
                idx,
            };
            let old = res.insert(*value, current);
            if let Some(old) = old {
                panic!(
                    "value {value} is defined twice, first time: {old:?}, second time: {current:?}"
                )
            }
        }

        for ins in block.ins.iter().copied() {
            let values = func.instr_result(ins);
            for value in values {
                let old = res.insert(value, ValueSource::Ins(ins));
                if let Some(old) = old {
                    panic!(
                        "value {value} is defined twice, first time: {old:?}, second time: {ins}"
                    )
                }
            }
        }
    }
    res
}
