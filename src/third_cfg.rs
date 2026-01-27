use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::{
    SectionData,
    addr::Addr,
    flat_ir::{self, Block, Imm, Value},
};

pub fn walk_code_blocks(text: SectionData<'_>, start: Addr) -> BTreeMap<Addr, Block> {
    let mut to_visit = vec![start];
    let mut block_starts = BTreeSet::from([start]);

    let mut blocks = BTreeMap::<Addr, Block>::new();

    while let Some(addr) = to_visit.pop() {
        if blocks.contains_key(&addr) {
            continue;
        }

        if let Some((&overlapping, block)) = blocks.range(..=addr).next_back()
            && block.contains(addr)
        {
            blocks.remove(&overlapping);
            to_visit.push(overlapping);
        }

        let code = if let Some(next) = block_starts.range(addr..).nth(1) {
            let size = next.0 - addr.0;
            text.slice(addr, size as _)
        } else {
            text.slice_to_end(addr)
        };

        let block = flat_ir::lower_block(code, addr);

        match block.terminator() {
            flat_ir::Terminator::Cond {
                then_bb, else_bb, ..
            } => {
                if let Value::Imm(Imm::U32(u32)) = then_bb {
                    to_visit.push(Addr(*u32));
                    block_starts.insert(Addr(*u32));
                }
                if let Value::Imm(Imm::U32(u32)) = else_bb {
                    to_visit.push(Addr(*u32));
                    block_starts.insert(Addr(*u32));
                }
            }
            flat_ir::Terminator::Jump { target, .. } => {
                if let Value::Imm(Imm::U32(u32)) = target {
                    to_visit.push(Addr(*u32));
                    block_starts.insert(Addr(*u32));
                }
            }
            flat_ir::Terminator::Ret { .. } => {}
            flat_ir::Terminator::Fallthrough { next } => {
                to_visit.push(*next);
                block_starts.insert(*next);
            }
        }

        for ins in block.instr() {
            if let flat_ir::Instr::Call {
                target: Value::Imm(Imm::U32(addr)),
            } = &ins.ins
            {
                to_visit.push(Addr(*addr));
                block_starts.insert(Addr(*addr));
            }
        }

        blocks.insert(block.addr(), block);
    }

    blocks
}

pub struct Function {
    addr: Addr,
    blocks: Vec<Addr>,
    #[allow(unused)]
    exits: Vec<Addr>,
}

impl Function {
    pub fn addr(&self) -> Addr {
        self.addr
    }

    pub fn blocks(&self) -> &[Addr] {
        &self.blocks
    }
}

pub fn derive_functions(blocks: &BTreeMap<Addr, Block>, entry: Addr) -> Vec<Function> {
    let mut entries = collect_entries(blocks.values());
    entries.insert(entry);

    let mut funcs = Vec::with_capacity(entries.len());
    for addr in &entries {
        let func = derive_func(blocks, &entries, *addr);
        funcs.push(func);
    }

    funcs.sort_unstable_by(|a, b| a.addr.cmp(&b.addr));

    funcs
}

fn derive_func(blocks: &BTreeMap<Addr, Block>, entries: &HashSet<Addr>, start: Addr) -> Function {
    let mut func_blocks = BTreeSet::new();
    let mut exits = BTreeSet::new();

    let mut worklist = vec![start];
    let mut visited = HashSet::new();

    while let Some(addr) = worklist.pop() {
        let new = visited.insert(addr);

        if !new {
            continue;
        }

        if entries.contains(&addr) && addr != start {
            continue;
        }

        func_blocks.insert(addr);
        let block = &blocks[&addr];

        match block.terminator() {
            flat_ir::Terminator::Cond {
                then_bb, else_bb, ..
            } => {
                if let Value::Imm(Imm::U32(u32)) = then_bb {
                    worklist.push(Addr(*u32));
                }
                if let Value::Imm(Imm::U32(u32)) = else_bb {
                    worklist.push(Addr(*u32));
                }
            }
            flat_ir::Terminator::Jump { target, .. } => {
                if let Value::Imm(Imm::U32(u32)) = target {
                    worklist.push(Addr(*u32));
                }
            }
            flat_ir::Terminator::Ret { .. } => {
                exits.insert(block.addr());
            }
            flat_ir::Terminator::Fallthrough { next } => {
                worklist.push(*next);
            }
        }
    }

    Function {
        addr: start,
        blocks: func_blocks.into_iter().collect(),
        exits: exits.into_iter().collect(),
    }
}

fn collect_entries<'a>(blocks: impl Iterator<Item = &'a Block>) -> HashSet<Addr> {
    let mut set = HashSet::new();

    for block in blocks {
        for ins in block.instr() {
            if let flat_ir::Instr::Call {
                target: Value::Imm(Imm::U32(addr)),
            } = &ins.ins
            {
                set.insert(Addr(*addr));
            }
        }
    }

    set
}
