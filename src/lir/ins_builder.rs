use crate::lir::{block::BlockId, func::SsaFunction, ins::Ins, value::ValueId};

use super::value::Value;

pub struct InsBuilder<'a> {
    pub func: &'a mut SsaFunction,
    pub block: BlockId,
}

impl<'a> InsBuilder<'a> {
    pub fn prepend_uninit_read(&mut self) -> ValueId {
        let dst = self.func.values.add(Value::Invalid);
        let ins = self.func.ins.add(Ins::Uninit { dst });
        self.func.blocks[self.block].ins.insert(0, ins);
        dst
    }
}
