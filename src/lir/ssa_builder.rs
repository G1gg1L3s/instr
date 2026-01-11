use std::collections::HashMap;

use crate::lir::{
    block::BlockId,
    func::SsaFunction,
    ins_builder::InsBuilder,
    value::{Value, ValueId},
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarId(u16);

#[derive(Default, Debug)]
struct BuilderBlock {
    sealed: bool,
}

pub struct SsaBuilder {
    pub func: SsaFunction,

    next_var: u16,

    current_block: Option<BlockId>,

    blocks: HashMap<BlockId, BuilderBlock>,

    variables: HashMap<VarId, HashMap<BlockId, ValueId>>,
    incomplete_phis: HashMap<BlockId, Vec<(VarId, ValueId)>>,
}

impl SsaBuilder {
    pub fn new() -> Self {
        Self {
            func: SsaFunction::new(),
            next_var: 0,
            current_block: None,
            blocks: HashMap::new(),
            variables: HashMap::new(),
            incomplete_phis: HashMap::new(),
        }
    }

    fn new_param(&mut self) -> ValueId {
        // TODO: type
        self.func.values.add(Value::Invalid)
    }

    pub fn new_value(&mut self) -> ValueId {
        // TODO: type
        self.func.values.add(Value::Invalid)
    }

    fn add_block_param(&mut self, block: BlockId, param: ValueId) {
        self.func.blocks[block].params.push(param);
    }

    pub fn ins(&mut self) -> InsBuilder<'_> {
        self.func.ins(self.current_block.unwrap())
    }

    // --- Variable handling --------------------

    pub fn declare_var(&mut self) -> VarId {
        let v = VarId(self.next_var);
        self.next_var += 1;
        v
    }

    pub fn switch(&mut self, block: BlockId) {
        self.current_block = Some(block);
    }

    pub fn write_var(&mut self, block: BlockId, var: VarId, val: ValueId) {
        self.variables.entry(var).or_default().insert(block, val);
    }

    pub fn read_var(&mut self, block: BlockId, var: VarId) -> ValueId {
        if let Some(&v) = self.variables.entry(var).or_default().get(&block) {
            return v;
        }

        let sealed = self.blocks.get(&block).map_or(false, |b| b.sealed);

        if !sealed {
            let phi = self.new_param();
            self.add_block_param(block, phi);
            self.incomplete_phis
                .entry(block)
                .or_default()
                .push((var, phi));
            self.write_var(block, var, phi);
            return phi;
        }

        match &self.func.blocks[block].predecessors.as_slice() {
            [] => {
                let val = self.ins().prepend_uninit_read();
                self.write_var(block, var, val);
                return val;
            }
            [pred] => {
                let val = self.read_var(*pred, var);
                self.write_var(block, var, val);
                return val;
            }
            _ => {}
        }

        let phi = self.new_value();
        self.add_block_param(block, phi);
        self.write_var(block, var, phi);

        let preds = self.func.blocks[block].predecessors.clone();
        let mut values = Vec::with_capacity(preds.len());

        for b in &preds {
            let val = self.read_var(*b, var);
            let val = self.func.resolve_alias(val);
            values.push(val);
        }

        let trivial = extract_trivial_phi(phi, &values);
        if let Some(trivial) = trivial {
            self.func.set_alias(phi, trivial);
            self.func.patch_remove_block_param(block, phi);
            return trivial;
        }

        for (pred, val) in preds.iter().zip(values) {
            self.func.patch_add_phi_arg(*pred, block, val);
        }

        phi
    }

    pub fn seal(&mut self, block: BlockId) {
        self.blocks.entry(block).or_default().sealed = true;

        let entries: Vec<(VarId, ValueId)> =
            self.incomplete_phis.remove(&block).unwrap_or_default();

        let preds = self.func.blocks[block].predecessors.clone();
        for (var, phi) in entries {
            let mut values = Vec::with_capacity(preds.len());

            for p in &preds {
                let val = self.read_var(*p, var);
                let val = self.func.resolve_alias(val);
                values.push(val);
            }

            if let Some(trivial) = extract_trivial_phi(phi, &values) {
                self.func.set_alias(phi, trivial);
                self.func.patch_remove_block_param(block, phi);
                continue;
            }

            for (pred, val) in preds.iter().zip(values) {
                self.func.patch_add_phi_arg(*pred, block, val);
            }
        }
    }

    pub fn seal_all_blocks(&mut self) {
        for block in topo_sort_blocks(&self.func.blocks) {
            self.seal(block);
        }
    }
}

fn topo_sort_blocks(blocks: &super::block::Blocks) -> Vec<BlockId> {
    let mut indegree: HashMap<BlockId, usize> = HashMap::new();

    for (block_id, block) in blocks.iter() {
        indegree.entry(block_id).or_insert(0);
        for &pred in &block.predecessors {
            *indegree.entry(block_id).or_insert(0) += 1;
            indegree.entry(pred).or_insert(0);
        }
    }

    let mut queue = std::collections::VecDeque::new();
    for (&b, &deg) in &indegree {
        if deg == 0 {
            queue.push_back(b);
        }
    }

    let mut result = Vec::new();
    let mut indegree = indegree;

    while let Some(b) = queue.pop_front() {
        result.push(b);

        for (succ, bb) in blocks.iter() {
            if bb.predecessors.contains(&b) {
                let deg = indegree.get_mut(&succ).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(succ);
                }
            }
        }
    }

    if result.len() < indegree.len() {
        let remaining: Vec<_> = indegree
            .keys()
            .filter(|b| !result.contains(b))
            .copied()
            .collect();

        result.extend(remaining);
    }

    result
}

fn extract_trivial_phi(phi: ValueId, values: &[ValueId]) -> Option<ValueId> {
    let mut candidate = None;

    for &v in values {
        if v == phi {
            continue;
        }

        match candidate {
            None => candidate = Some(v),
            Some(x) if x == v => {}
            Some(_) => return None, // more than one non-phi value
        }
    }

    candidate
}
