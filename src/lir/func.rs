use crate::lir::{
    block::{BlockId, Blocks},
    ins::Instrs,
    ins_builder::InsBuilder,
    value::{Value, ValueId, Values},
};

#[derive(Debug)]
pub struct SsaFunction {
    pub ins: Instrs,
    pub blocks: Blocks,
    pub values: Values,
}

impl SsaFunction {
    pub fn new() -> Self {
        Self {
            ins: Instrs::new(),
            blocks: Blocks::new(),
            values: Values::new(),
        }
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

    pub fn patch_remove_block_param(&mut self, block_id: BlockId, value: ValueId) {
        let block = &mut self.blocks[block_id];
        let idx = block.params.iter().position(|x| *x == value).unwrap();

        block.params.remove(idx);

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
        let from = &mut self.blocks[from];
        from.terminator_mut().visit_jumps_mut(|target, args| {
            if target == to {
                args.push(val);
            }
        });
    }
}
