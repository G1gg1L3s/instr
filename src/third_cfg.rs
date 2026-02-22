use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::{
    SectionData,
    addr::Addr,
    flat_ir::{self, Block, Imm, Value},
};

pub struct CfgDb {
    entry: Addr,
    block_starts: BTreeSet<Addr>,
    function_starts: BTreeSet<Addr>,
    blocks: BTreeMap<Addr, Block>,
    functions: Vec<Function>,
    jump_tables: HashMap<Addr, JumpTable>,
}

impl CfgDb {
    pub fn new(entry: Addr) -> Self {
        Self {
            entry,
            block_starts: BTreeSet::from_iter([entry]),
            function_starts: BTreeSet::from_iter([entry]),
            blocks: Default::default(),
            functions: Default::default(),
            jump_tables: Default::default(),
        }
    }

    pub fn set_blocks(&mut self, blocks: BTreeMap<Addr, Block>) {
        self.blocks = blocks;
    }

    pub fn insert_func_addr(&mut self, addr: Addr) {
        self.block_starts.insert(addr);
        self.function_starts.insert(addr);
    }

    pub fn blocks(&self) -> &BTreeMap<Addr, Block> {
        &self.blocks
    }

    pub fn set_functions(&mut self, functions: Vec<Function>) {
        self.functions = functions;
    }

    pub fn functions(&self) -> &[Function] {
        &self.functions
    }

    pub fn set_jump_tables(&mut self, jump_tables: impl IntoIterator<Item = JumpTable>) {
        self.jump_tables = jump_tables.into_iter().map(|t| (t.ins_addr, t)).collect();
    }

    pub fn get_jump_table(&self, instruction_addr: Addr) -> Option<&JumpTable> {
        self.jump_tables.get(&instruction_addr)
    }

    pub fn entry(&self) -> Addr {
        self.entry
    }
}

pub fn walk_code_blocks(db: &CfgDb, text: SectionData<'_>) -> BTreeMap<Addr, Block> {
    let mut to_visit = db.block_starts.iter().copied().collect::<Vec<_>>();
    let mut block_starts = db.block_starts.clone();

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

        let block = flat_ir::lower_block(db, code, addr);

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
            flat_ir::Terminator::JumpTable {
                addr: _,
                jump_addr: _,
                entries,
            } => {
                to_visit.extend(entries);
                block_starts.extend(entries);
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

pub fn walk_code_blocks_on_section(db: &CfgDb, text: SectionData<'_>) -> BTreeMap<Addr, Block> {
    let mut to_visit = vec![text.address, db.entry];
    let mut block_starts = BTreeSet::from_iter([text.address, db.entry]);

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

        let block = match flat_ir::lower_maybe_block(db, code, addr) {
            flat_ir::BlockOrPadding::Block(block) => block,
            flat_ir::BlockOrPadding::Padding { next } => {
                to_visit.push(next);
                block_starts.insert(next);
                continue;
            }
        };

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
            flat_ir::Terminator::Ret { .. } => {
                let next_block = block.addr() + block.len_u32();
                to_visit.push(next_block);
                block_starts.insert(next_block);
            }
            flat_ir::Terminator::Fallthrough { next } => {
                to_visit.push(*next);
                block_starts.insert(*next);
            }
            flat_ir::Terminator::JumpTable {
                addr: _,
                jump_addr: _,
                entries,
            } => {
                to_visit.extend(entries);
                block_starts.extend(entries);
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

        println!("=== block {}", block.addr());
        println!("{}", block.asm_fmt(code));

        blocks.insert(block.addr(), block);
    }
    blocks
}

pub struct Function {
    addr: Addr,
    blocks: Vec<Addr>,
}

impl Function {
    pub fn addr(&self) -> Addr {
        self.addr
    }

    pub fn blocks(&self) -> &[Addr] {
        &self.blocks
    }
}

pub fn derive_functions(db: &CfgDb) -> Vec<Function> {
    let mut entries = collect_entries(db.blocks.values());
    entries.extend(&db.function_starts);

    let mut funcs = Vec::with_capacity(entries.len());
    for addr in &entries {
        let func = derive_func(&db.blocks, &entries, *addr);
        funcs.push(func);
    }

    funcs.sort_unstable_by(|a, b| a.addr.cmp(&b.addr));

    funcs
}

fn derive_func(blocks: &BTreeMap<Addr, Block>, entries: &HashSet<Addr>, start: Addr) -> Function {
    let mut func_blocks = BTreeSet::new();

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
            flat_ir::Terminator::Ret { .. } => {}
            flat_ir::Terminator::Fallthrough { next } => {
                worklist.push(*next);
            }
            flat_ir::Terminator::JumpTable {
                addr: _,
                jump_addr: _,
                entries,
            } => {
                worklist.extend(entries);
            }
        }
    }

    Function {
        addr: start,
        blocks: func_blocks.into_iter().collect(),
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

#[derive(Debug, Clone)]
pub struct JumpTable {
    pub ins_addr: Addr,
    pub entries: Vec<Addr>,
    pub size: Option<u32>,
}

impl JumpTable {
    pub fn addr(&self) -> Addr {
        *self.entries.first().unwrap()
    }
}
