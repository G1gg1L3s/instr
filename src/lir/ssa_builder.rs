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
        io::Io,
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

    variables: HashMap<VarId, HashMap<BlockId, Vec<ValueId>>>,
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

    pub fn add_entry_param(&mut self, block: BlockId, arg: Io, param: ValueId) {
        self.func.blocks[block].params.push(param);
        self.func.add_input_arg(arg, param);
    }

    pub fn ins_addr(&mut self, addr: Addr) -> InsBuilder<'_> {
        self.func.ins_addr(self.current_block.unwrap(), addr)
    }

    pub fn ins(&mut self) -> InsBuilder<'_> {
        self.func.ins(self.current_block.unwrap())
    }

    pub fn set_var_ty(&mut self, var: VarId, ty: Ty) {
        self.vars[var].ty = ty;
    }

    // --- Variable handling --------------------

    pub fn declare_var(&mut self, ty: Ty) -> VarId {
        self.vars.add(Var { ty })
    }

    pub fn switch(&mut self, block: BlockId) {
        self.current_block = Some(block);
    }

    pub fn write_var_in_block(&mut self, block: BlockId, var: VarId, val: ValueId) {
        let list = self
            .variables
            .entry(var)
            .or_default()
            .entry(block)
            .or_default();
        if let Ty::Flags(_) = self.vars[var].ty {
            list.push(val);
        } else {
            list.clear();
            list.push(val);
        }
    }

    pub fn get_var_in_block(&self, block: BlockId, var: VarId) -> Option<ValueId> {
        // This is cursed, but I want an easy way to support flags as values,
        // without adding separate Value for a flag
        let list = self.variables.get(&var)?.get(&block)?;
        if let Ty::Flags(flags) = self.vars[var].ty {
            for candidate in list.iter().rev() {
                let Value::Temp {
                    ty: Ty::Flags(candidate_flags),
                } = &self.func.values[*candidate]
                else {
                    panic!("value is no types")
                };

                if candidate_flags.contains(flags) {
                    return Some(*candidate);
                }
                if candidate_flags.intersects(flags) {
                    log::error!("partial flag intersection")
                }
            }
            None
        } else {
            list.first().copied()
        }
    }

    pub fn read_var_in_block(&mut self, block: BlockId, var: VarId) -> ValueId {
        if let Some(v) = self.get_var_in_block(block, var) {
            return v;
        };

        let sealed = self.blocks.get(&block).is_some_and(|b| b.sealed);

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
                let val = self.func.ins(block).prepend_uninit_read(self.vars[var].ty);
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

            if values.is_empty() {
                log::trace!(
                    "        >> Predecessors are empty, creating uninit read {var} -> {phi}"
                );
                let uninit = self.func.ins(block).prepend_uninit_read(self.vars[var].ty);
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
        let mut remaining: Vec<_> = indegree
            .keys()
            .filter(|b| !result.contains(b))
            .copied()
            .collect();

        remaining.sort_by_key(|b| b.to_idx());

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
