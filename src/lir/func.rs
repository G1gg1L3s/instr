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
        log::trace!("- Patch remove block param: {block_id}({value})");
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

    pub fn instr_result(&self, ins: InsId) -> heapless::Vec<ValueId, 16> {
        let mut res = heapless::Vec::<ValueId, 16>::new();
        let ins = &self.ins[ins];
        match ins {
            Ins::Uninit { dst } => res.push(*dst).unwrap(),
            Ins::BinOp { dst, flags, .. } => {
                res.push(*dst).unwrap();
                if let Some(flags) = flags {
                    res.push(*flags).unwrap()
                }
            }
            Ins::Const { dst, .. } => res.push(*dst).unwrap(),
            Ins::Unimpl { dst } => res.push(*dst).unwrap(),
            Ins::Load { dst, .. } => res.push(*dst).unwrap(),
            Ins::Cond { dst, .. } => res.push(*dst).unwrap(),
            Ins::Store { dst_mem, .. } => res.push(*dst_mem).unwrap(),
            Ins::Call { result, .. } => res.extend_from_slice(result).unwrap(),
        }

        res
    }

    pub fn val_ty(&self, val: ValueId) -> Option<Ty> {
        let value = &self.values[val];
        match value {
            Value::Invalid => None,
            Value::Temp { ty } => Some(*ty),
            Value::Alias { .. } => self.val_ty(self.resolve_alias(val)),
            Value::Mem => None,
        }
    }

    pub fn val(&self, val: ValueId) -> &Value {
        let val = self.resolve_alias(val);
        &self.values[val]
    }

    pub fn patch_resolve_aliases(&mut self) -> bool {
        let mut patched = false;
        for id in self.values.keys() {
            let resolved = self.resolve_alias(id);

            if resolved != id {
                patched = true;
                self.patch_replace_value(id, resolved);
                self.values[id] = Value::Invalid;
            }
        }
        patched
    }

    fn patch_replace_value(&mut self, from: ValueId, to: ValueId) {
        for (_, ins) in self.ins.iter_mut() {
            ins.patch_replace_value(from, to);
        }

        for (_, block) in self.blocks.iter_mut() {
            for arg in &mut block.params {
                if *arg == from {
                    *arg = to;
                }
            }
            block.terminator_mut().visit_values_mut(|val| {
                if *val == from {
                    *val = to;
                }
            });
        }
    }

    pub fn patch_resolve_trivial_phis(&mut self) -> bool {
        let mut to_patch = vec![];
        for block_id in self.blocks.keys() {
            let block = &self.blocks[block_id];
            let preds = block.predecessors.clone();

            let params: Vec<ValueId> = block.params.clone();
            let mut param_values = vec![vec![]; params.len()];

            log::trace!(
                ">> Collecting param values for {block_id}{}",
                crate::lir::fmt::FmtList(&params)
            );

            for pred in preds {
                self.extract_param_values(block_id, &mut param_values, pred);
            }

            for (phi, values) in params.iter().zip(&param_values) {
                if let Some(trivial) = crate::lir::ssa_builder::extract_trivial_phi(*phi, values) {
                    to_patch.push((block_id, *phi, trivial));
                }
            }
        }

        for &(block_id, phi, trivial) in &to_patch {
            log::trace!(">> Patching {block_id}: {phi} -> {trivial}");
            self.patch_remove_block_param_and_calls(block_id, phi);
            self.set_alias(phi, trivial);
        }
        let patched = !to_patch.is_empty();
        patched
    }

    fn extract_param_values(
        &self,
        block_id: BlockId,
        param_values: &mut Vec<Vec<ValueId>>,
        pred: BlockId,
    ) {
        fn collect_param_values(param_values: &mut Vec<Vec<ValueId>>, args: &[ValueId]) {
            log::trace!(">>> - param values: {}", crate::lir::fmt::FmtList(args));
            for (i, arg) in args.iter().copied().enumerate() {
                param_values[i].push(arg);
            }
        }

        self.blocks[pred].terminator().visit_jumps(|target, args| {
            if target == block_id {
                collect_param_values(param_values, args);
            }
        });
    }

    pub fn resolve(&mut self) {
        loop {
            let patched = self.patch_resolve_aliases();
            if !patched {
                break;
            }
            let patched = self.patch_resolve_trivial_phis();
            if !patched {
                break;
            }
        }
    }
}
