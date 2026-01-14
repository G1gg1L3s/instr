use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

use crate::{
    addr::Addr,
    lir::{
        block::BlockId,
        fmt::FmtList,
        func::SsaFunction,
        ins_builder::InsBuilder,
        ty::Ty,
        value::{Value, ValueId},
    },
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarId(u16);

impl std::fmt::Debug for VarId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "var{}", self.0)
    }
}

impl std::fmt::Display for VarId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

#[derive(Debug, Clone)]
struct Var {
    ty: Ty,
}

#[derive(Debug, Clone)]
struct Vars(Vec<Var>);

impl Vars {
    fn new() -> Self {
        Self(vec![])
    }

    fn add(&mut self, var: Var) -> VarId {
        let id = self.0.len().try_into().expect("to much values");
        self.0.push(var);
        VarId(id)
    }
}

impl Index<VarId> for Vars {
    type Output = Var;

    fn index(&self, index: VarId) -> &Self::Output {
        self.0.index(usize::from(index.0))
    }
}

impl IndexMut<VarId> for Vars {
    fn index_mut(&mut self, index: VarId) -> &mut Self::Output {
        self.0.index_mut(usize::from(index.0))
    }
}

#[derive(Default, Debug)]
struct BuilderBlock {
    sealed: bool,
}

#[derive(Debug)]
pub struct SsaBuilder {
    pub func: SsaFunction,

    vars: Vars,

    current_block: Option<BlockId>,

    blocks: HashMap<BlockId, BuilderBlock>,

    variables: HashMap<VarId, HashMap<BlockId, ValueId>>,
    incomplete_phis: HashMap<BlockId, Vec<(VarId, ValueId)>>,
}

impl SsaBuilder {
    pub fn new(addr: Addr) -> Self {
        Self {
            func: SsaFunction::new(addr),
            vars: Vars::new(),
            current_block: None,
            blocks: HashMap::new(),
            variables: HashMap::new(),
            incomplete_phis: HashMap::new(),
        }
    }

    pub fn new_block(&mut self, addr: Addr) -> BlockId {
        self.func.blocks.add(addr)
    }

    pub fn new_param(&mut self, ty: Ty) -> ValueId {
        self.func.values.add(Value::Temp { ty })
    }

    pub fn new_value(&mut self, ty: Ty) -> ValueId {
        self.func.values.add(Value::Temp { ty })
    }

    pub fn add_block_param(&mut self, block: BlockId, param: ValueId) {
        self.func.blocks[block].params.push(param);
    }

    pub fn ins(&mut self) -> InsBuilder<'_> {
        self.func.ins(self.current_block.unwrap())
    }

    // --- Variable handling --------------------

    pub fn declare_var(&mut self, ty: Ty) -> VarId {
        self.vars.add(Var { ty })
    }

    pub fn switch(&mut self, block: BlockId) {
        self.current_block = Some(block);
    }

    pub fn write_var_in_block(&mut self, block: BlockId, var: VarId, val: ValueId) {
        self.variables.entry(var).or_default().insert(block, val);
    }

    pub fn read_var_in_block(&mut self, block: BlockId, var: VarId) -> ValueId {
        if let Some(&v) = self.variables.entry(var).or_default().get(&block) {
            return v;
        }

        let sealed = self.blocks.get(&block).map_or(false, |b| b.sealed);

        if !sealed {
            let phi = self.new_param(self.vars[var].ty);
            log::trace!(">> Reading {var} in {block}: block not sealed, creating phi {phi}");
            self.add_block_param(block, phi);
            self.incomplete_phis
                .entry(block)
                .or_default()
                .push((var, phi));
            self.write_var_in_block(block, var, phi);
            return phi;
        }
        log::trace!(">> Reading {var} in {block}: block sealed");

        match &self.func.blocks[block].predecessors.as_slice() {
            [] => {
                let val = self.ins().prepend_uninit_read();
                self.write_var_in_block(block, var, val);
                log::trace!("    >> Predecessors are empty, creating uninit read {var} -> {val}");
                return val;
            }
            [pred] => {
                let val = self.read_var_in_block(*pred, var);
                self.write_var_in_block(block, var, val);
                log::trace!("    >> One predecessor, returning {val}");
                return val;
            }
            _ => {}
        }

        let phi = self.new_param(self.vars[var].ty);
        self.add_block_param(block, phi);
        self.write_var_in_block(block, var, phi);

        log::trace!("    >> Creating phi {phi} for {var}:");

        let preds = self.func.blocks[block].predecessors.clone();
        let mut values = Vec::with_capacity(preds.len());

        for b in &preds {
            let val = self.read_var_in_block(*b, var);
            let val = self.func.resolve_alias(val);
            values.push(val);
        }

        log::trace!(
            "        >> Values for phi {phi} ({var}): {}",
            FmtList(&values)
        );

        let trivial = extract_trivial_phi(phi, &values);
        if let Some(trivial) = trivial {
            self.func.set_alias(phi, trivial);
            self.func.patch_remove_block_param(block, phi);

            log::trace!("    >> Phi {phi} ({var}) is trivial: {trivial}");
            return trivial;
        }

        for (pred, val) in preds.iter().zip(values) {
            self.func.patch_add_phi_arg(*pred, block, val);
        }

        phi
    }

    pub fn read_var(&mut self, var: VarId) -> ValueId {
        self.read_var_in_block(self.current_block.unwrap(), var)
    }

    pub fn write_var(&mut self, var: VarId, val: ValueId) {
        self.write_var_in_block(self.current_block.unwrap(), var, val);
    }

    pub fn seal(&mut self, block: BlockId) {
        log::trace!(">> Sealing {}", block);
        self.blocks.entry(block).or_default().sealed = true;

        let entries: Vec<(VarId, ValueId)> =
            self.incomplete_phis.remove(&block).unwrap_or_default();

        let preds = self.func.blocks[block].predecessors.clone();
        for (var, phi) in entries {
            log::trace!("    - {}, phi: {}", var, phi);

            let mut values = Vec::with_capacity(preds.len());

            for p in &preds {
                let val = self.read_var_in_block(*p, var);
                let val = self.func.resolve_alias(val);
                values.push(val);
            }

            if values.len() == 0 {
                log::trace!(
                    "        >> Predecessors are empty, creating uninit read {var} -> {phi}"
                );
                let uninit = self.func.ins(block).prepend_uninit_read();
                self.func.patch_remove_block_param(block, phi);
                self.func.set_alias(phi, uninit);
                continue;
            }

            log::trace!("    >> values: {}", FmtList(&values));

            if let Some(trivial) = extract_trivial_phi(phi, &values) {
                log::trace!("    >> trivial phi: {}", trivial);
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

    pub fn finalise(mut self) -> SsaFunction {
        self.seal_all_blocks();
        self.func
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

pub fn extract_trivial_phi(phi: ValueId, values: &[ValueId]) -> Option<ValueId> {
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
