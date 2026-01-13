use std::collections::HashMap;

use crate::{
    addr::Addr,
    lir::{
        block::{BlockId, Blocks},
        fmt::FuncFmt,
        ins::{Ins, InsId, Instrs},
        ins_builder::InsBuilder,
        ty::Ty,
        value::{Value, ValueId, Values},
    },
};

#[derive(Debug)]
pub struct SsaFunction {
    pub addr: Addr,
    pub ins: Instrs,
    pub blocks: Blocks,
    pub values: Values,
}

impl SsaFunction {
    pub fn new(addr: Addr) -> Self {
        Self {
            addr,
            ins: Instrs::new(),
            blocks: Blocks::new(),
            values: Values::new(),
        }
    }

    pub fn fmt(&self) -> FuncFmt<'_> {
        FuncFmt::new(self)
    }

    pub fn record_predecessor(&mut self, from: BlockId, to: BlockId) {
        self.blocks[to].predecessors.push(from);
    }

    pub fn ins(&mut self, block: BlockId) -> InsBuilder<'_> {
        InsBuilder { func: self, block }
    }

    pub fn set_alias(&mut self, from: ValueId, to: ValueId) {
        self.values[from] = Value::Alias { to };
    }

    pub fn resolve_alias(&self, mut val: ValueId) -> ValueId {
        for _ in self.values.keys() {
            match self.values[val] {
                Value::Alias { to } => val = to,
                _ => break,
            }
        }

        val
    }

    pub fn patch_remove_block_param(&mut self, block_id: BlockId, value: ValueId) -> usize {
        let block: &mut super::block::Block = &mut self.blocks[block_id];
        let idx = block.params.iter().position(|x| *x == value).unwrap();

        block.params.remove(idx);
        idx
    }

    pub fn patch_remove_block_param_and_calls(&mut self, block_id: BlockId, value: ValueId) {
        let idx = self.patch_remove_block_param(block_id, value);
        let block = &mut self.blocks[block_id];

        for pred in block.predecessors.clone() {
            let pred = &mut self.blocks[pred];
            pred.terminator_mut().visit_jumps_mut(|target, args| {
                if target == block_id {
                    args.remove(idx);
                }
            });
        }
    }

    pub fn patch_add_phi_arg(&mut self, from: BlockId, to: BlockId, val: ValueId) {
        log::trace!("+ Add phi args: {from} -> {to}: {val}");

        let from = &mut self.blocks[from];
        from.terminator_mut().visit_jumps_mut(|target, args| {
            if target == to {
                args.push(val);
            }
        });
    }

    pub(crate) fn inverse_aliases(&self) -> HashMap<ValueId, Vec<ValueId>> {
        let mut res = std::collections::HashMap::<ValueId, Vec<_>>::new();
        for (val_id, val) in self.values.iter() {
            if let Value::Alias { to } = val {
                res.entry(*to).or_default().push(val_id);
            }
        }
        res
    }

    pub fn instr_result(&self, ins: InsId) -> Option<ValueId> {
        let ins = &self.ins[ins];
        match ins {
            Ins::Uninit { dst } => Some(*dst),
            Ins::BinOp { dst, .. } => Some(*dst),
            Ins::Const { dst, .. } => Some(*dst),
            Ins::Unimpl { dst } => Some(*dst),
            Ins::Load { dst, .. } => Some(*dst),
        }
    }

    pub fn val_ty(&self, val: ValueId) -> Option<Ty> {
        let value = &self.values[val];
        match value {
            Value::Invalid => None,
            Value::Temp { ty } => Some(*ty),
            Value::Alias { .. } => self.val_ty(self.resolve_alias(val)),
            Value::Flags(_) => Some(Ty::Flags),
            Value::Mem => None,
        }
    }
}
