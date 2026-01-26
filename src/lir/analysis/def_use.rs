use std::collections::HashMap;

use crate::lir::{
    block::BlockId,
    func::SsaFunction,
    ins::InsId,
    value::{Imm, Value, ValueId},
};

#[derive(Debug, Clone, Copy)]
pub enum ValueSource {
    Ins(InsId),
    Param { block: BlockId, idx: usize },
    Imm(Imm),
}

pub fn compute(func: &SsaFunction) -> HashMap<ValueId, ValueSource> {
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

    for (val_id, val) in func.values.iter() {
        let Value::Imm(imm) = val else {
            continue;
        };
        let old = res.insert(val_id, ValueSource::Imm(*imm));
        if let Some(old) = old {
            panic!("value {val_id} is defined twice, first time: {old:?}, second time: {val:?}")
        }
    }

    res
}
