use std::collections::{BTreeMap, HashMap, HashSet, hash_map::Entry};

use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
};

use crate::{
    SectionData,
    addr::Addr,
    cfg::looks_like_jump_table,
    ins::{BinaryInstruction, Decoder, Instruction, Op},
    obj::{ObjDatabase, ObjectTyp},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeBlockTyp {
    Plain,
    Entry,
}

#[derive(Debug, Clone)]
pub struct CodeBlock {
    code: Vec<BinaryInstruction>,
    len: usize,
    typ: CodeBlockTyp,
}

impl CodeBlock {
    pub fn new(code: Vec<BinaryInstruction>, typ: CodeBlockTyp) -> Self {
        Self {
            len: code.iter().map(|i| i.len).sum(),
            code,
            typ,
        }
    }

    pub fn addr(&self) -> Addr {
        self.code[0].addr
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn end(&self) -> Addr {
        self.addr() + self.len()
    }

    pub fn to_range(&self) -> std::ops::Range<Addr> {
        self.addr()..self.end()
    }

    pub fn split(mut self, addr: Addr) -> (Self, Vec<BinaryInstruction>) {
        let idx = self
            .code
            .binary_search_by(|i| i.addr.cmp(&addr))
            .expect("cannot split code block");

        let tail = self.code.split_off(idx);

        (Self::new(self.code, self.typ), tail)
    }

    pub fn typ(&self) -> CodeBlockTyp {
        self.typ
    }
}

#[derive(Debug, Clone)]
pub struct JumpTable {
    pub entries: Vec<JumpTableEntry>,
}

impl JumpTable {
    pub fn addr(&self) -> Addr {
        self.entries[0].addr
    }

    pub fn len(&self) -> usize {
        self.entries.len() * 4
    }
}

#[derive(Debug, Clone)]
pub enum Block {
    Code(CodeBlock),
    JumpTable(JumpTable),
}

impl Block {
    pub fn addr(&self) -> Addr {
        match self {
            Block::Code(code_block) => code_block.addr(),
            Block::JumpTable(jump_table) => jump_table.addr(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Block::Code(code_block) => code_block.len(),
            Block::JumpTable(jump_table) => jump_table.len(),
        }
    }

    pub fn end(&self) -> Addr {
        self.addr() + self.len()
    }

    pub fn to_range(&self) -> std::ops::Range<Addr> {
        self.addr()..self.end()
    }
}

#[derive(Default)]
pub struct BlockTree {
    tree: BTreeMap<Addr, Block>,
}

impl BlockTree {
    pub fn insert(&mut self, block: Block) {
        self.tree.insert(block.addr(), block);
    }

    pub fn get(&self, addr: Addr) -> Option<&Block> {
        self.tree.get(&addr)
    }

    pub fn get_mut(&mut self, addr: Addr) -> Option<&mut Block> {
        self.tree.get_mut(&addr)
    }

    pub fn get_with_addr(&mut self, addr: Addr) -> Option<&Block> {
        let (_, block) = self.tree.range(..=addr).next_back()?;

        if block.to_range().contains(&addr) {
            Some(block)
        } else {
            None
        }
    }

    pub fn remove_with_addr(&mut self, addr: Addr) -> Option<Block> {
        let block = self.get_with_addr(addr)?;
        let addr = block.addr();
        self.tree.remove(&addr)
    }

    pub fn next_after(&self, addr: Addr) -> Option<&Block> {
        self.tree.range(addr..).next().map(|(_, v)| v)
    }
}

pub fn walk_code_blocks(
    db: &ObjDatabase,
    text: SectionData<'_>,
    start: Addr,
) -> BTreeMap<Addr, Block> {
    let mut queue = vec![(start, CodeBlockTyp::Entry)];
    let mut res = BlockTree::default();

    while let Some((addr, typ)) = queue.pop() {
        if let Some(block) = res.get_mut(addr) {
            if let CodeBlockTyp::Entry = typ
                && let Block::Code(code) = block
            {
                code.typ = typ;
            }
            continue;
        }

        if let Some(block) = res.remove_with_addr(addr) {
            let Block::Code(block) = block else {
                panic!("trying to split non-code block at {}", addr);
            };

            let (left, right_code) = block.split(addr);
            let right = CodeBlock::new(right_code, typ);

            res.insert(Block::Code(left));
            res.insert(Block::Code(right));
            continue;
        }

        let limit = res
            .next_after(addr)
            .map(|next_after| usize::try_from(next_after.addr().0 - addr.0).unwrap());

        let WalkedCodeBlock { code, successors } = walk_block(db, text, addr, limit);
        let block = CodeBlock::new(code, typ);

        for succ in successors {
            match succ {
                CodeBlockSucc::Jump(addr) => queue.push((addr, CodeBlockTyp::Plain)),
                CodeBlockSucc::JumpCond(addr) => queue.push((addr, CodeBlockTyp::Plain)),
                CodeBlockSucc::Call(addr) => queue.push((addr, CodeBlockTyp::Entry)),
                CodeBlockSucc::JumpTable(addr) => {
                    eprintln!(">> Jump table: {addr}");
                    let Some(entries) = walk_jump_table(text, addr) else {
                        continue;
                    };

                    queue.extend(
                        entries
                            .iter()
                            .map(|entry| (entry.target, CodeBlockTyp::Plain)),
                    );
                    res.insert(Block::JumpTable(JumpTable { entries }));
                }
                CodeBlockSucc::Fallthrough => queue.push((block.end(), CodeBlockTyp::Plain)),
            }
        }

        res.insert(Block::Code(block));
    }

    res.tree
}

#[derive(Debug, Clone)]
pub struct JumpTableEntry {
    pub addr: Addr,
    pub target: Addr,
}

fn walk_jump_table(text: SectionData<'_>, mut addr: Addr) -> Option<Vec<JumpTableEntry>> {
    let mut res = vec![];

    loop {
        let target = Addr(text.read_u32_le(addr));
        if !text.contains(target) {
            break;
        }
        res.push(JumpTableEntry { addr, target });
        addr += 4;
    }

    if res.len() >= 3 { Some(res) } else { None }
}

enum CodeBlockSucc {
    Jump(Addr),
    JumpCond(Addr),
    Call(Addr),
    JumpTable(Addr),
    Fallthrough,
}

struct WalkedCodeBlock {
    code: Vec<BinaryInstruction>,
    successors: Vec<CodeBlockSucc>,
}

fn walk_block(
    db: &ObjDatabase,
    text: SectionData<'_>,
    addr: Addr,
    limit: Option<usize>,
) -> WalkedCodeBlock {
    let mut successors = vec![];
    let mut code = vec![];

    let slice = if let Some(limit) = limit {
        text.slice(addr, limit)
    } else {
        text.slice_to_end(addr)
    };
    let mut decoder = Decoder::new(slice, addr);

    for ins in &mut decoder {
        code.push(ins);

        match ins.instr {
            Instruction::Call(Op::Mem(mem)) => {
                if let Some(obj) = mem.to_absolute().map(Addr).and_then(|addr| db.get(addr))
                    && let ObjectTyp::ImportThunk(import) = obj.typ()
                    && import.is_terminating
                {
                    break;
                }
            }
            Instruction::Call(Op::Addr(addr)) => successors.push(CodeBlockSucc::Call(addr)),
            Instruction::Return(_) => break,
            Instruction::Jump(Op::Addr(addr)) => {
                successors.push(CodeBlockSucc::Jump(addr));
                break;
            }
            Instruction::Jump(Op::Mem(mem)) => {
                if looks_like_jump_table(&mem) && text.contains(Addr(mem.disp)) {
                    successors.push(CodeBlockSucc::JumpTable(Addr(mem.disp)))
                }

                break;
            }
            Instruction::JumpConditional(Op::Addr(addr), _) => {
                successors.push(CodeBlockSucc::JumpCond(addr));
                successors.push(CodeBlockSucc::Fallthrough);
                break;
            }
            Instruction::JumpConditional(Op::Mem(mem), _) => {
                if looks_like_jump_table(&mem) && text.contains(Addr(mem.disp)) {
                    successors.push(CodeBlockSucc::JumpTable(Addr(mem.disp)))
                }
                successors.push(CodeBlockSucc::Fallthrough);
                break;
            }
            Instruction::Loop(addr) => {
                successors.push(CodeBlockSucc::JumpCond(addr));
                successors.push(CodeBlockSucc::Fallthrough);
                break;
            }
            Instruction::Int3 | Instruction::Invalid => {
                panic!(">> {ins:?}");
            }
            Instruction::PushImm(_) | Instruction::MovImm(_) => {}
            Instruction::IcedX86 => {}
        }
    }

    WalkedCodeBlock { code, successors }
}

#[derive(Debug, Default)]
pub struct BlockGraph {
    graph: DiGraph<Addr, ()>,
    addr_to_node: HashMap<Addr, NodeIndex>,
}

impl BlockGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_node(&mut self, addr: Addr) {
        match self.addr_to_node.entry(addr) {
            Entry::Occupied(_) => {}
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(self.graph.add_node(addr));
            }
        }
    }

    pub fn insert_edge(&mut self, from: Addr, to: Addr) {
        let from = self
            .addr_to_node
            .entry(from)
            .or_insert_with(|| self.graph.add_node(from));
        let from = *from;
        let to = self
            .addr_to_node
            .entry(to)
            .or_insert_with(|| self.graph.add_node(to));
        let to = *to;
        self.graph.add_edge(from, to, ());
    }

    pub fn successors(&self, addr: Addr) -> impl Iterator<Item = Addr> {
        let idx = self.addr_to_node[&addr];
        self.graph
            .edges_directed(idx, petgraph::Direction::Outgoing)
            .map(|edge| {
                let target = edge.target();
                self.graph.node_weight(target).copied().unwrap()
            })
    }

    pub fn externals(&self) -> impl Iterator<Item = Addr> {
        self.graph
            .externals(petgraph::Direction::Incoming)
            .map(|node| self.graph.node_weight(node).copied().unwrap())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FuncAddr(Addr);

pub fn build_jump_graph(db: &ObjDatabase, blocks: &[Block]) -> BlockGraph {
    let blocks = BTreeMap::from_iter(blocks.iter().map(|b| (b.addr(), b)));
    let mut jump_graph = BlockGraph::new();

    for block in blocks.values() {
        if let Block::Code(block) = block {
            jump_graph.insert_node(block.addr());
            let next_addrs = next_addrs(db, &blocks, block);

            for next in &next_addrs {
                jump_graph.insert_edge(block.addr(), *next);
            }
        }
    }

    let dot = petgraph::dot::Dot::with_config(
        &jump_graph.graph,
        &[
            petgraph::dot::Config::EdgeNoLabel,
            petgraph::dot::Config::RankDir(petgraph::dot::RankDir::TB),
        ],
    );
    std::fs::write("block.dot", format!("{:?}", dot)).unwrap();

    jump_graph
}

fn next_addrs(db: &ObjDatabase, blocks: &BTreeMap<Addr, &Block>, block: &CodeBlock) -> Vec<Addr> {
    let last = block.code.last().unwrap();
    let mut successors = vec![];

    match last.instr {
        Instruction::Call(Op::Mem(mem)) => {
            let is_terminating = mem
                .to_absolute()
                .map(Addr)
                .and_then(|addr| db.get(addr))
                .and_then(|obj| match obj.typ() {
                    ObjectTyp::ImportThunk(import) => Some(import),
                    _ => None,
                })
                .map(|import| import.is_terminating)
                .unwrap_or(false);

            if !is_terminating {
                successors.push(block.end());
            }
        }
        Instruction::Return(_) => {}
        Instruction::Jump(Op::Addr(addr)) => {
            successors.push(addr);
        }
        Instruction::Jump(Op::Mem(mem)) => {
            if looks_like_jump_table(&mem)
                && let Some(Block::JumpTable(jt)) = blocks.get(&Addr(mem.disp))
            {
                successors.extend(jt.entries.iter().map(|entry| entry.target));
            }
        }
        Instruction::JumpConditional(Op::Addr(addr), _) => {
            successors.push(addr);
            successors.push(block.end());
        }
        Instruction::JumpConditional(Op::Mem(mem), _) => {
            if looks_like_jump_table(&mem)
                && let Some(Block::JumpTable(jt)) = blocks.get(&Addr(mem.disp))
            {
                successors.extend(jt.entries.iter().map(|entry| entry.target));
            }
            successors.push(block.end());
        }
        Instruction::Loop(addr) => {
            successors.push(addr);
            successors.push(block.end());
        }
        Instruction::Int3 | Instruction::Invalid => {
            panic!(">> {last:?}");
        }
        _ => {
            successors.push(block.end());
        }
    }

    successors
}

pub fn derive_functions(db: &ObjDatabase, blocks: &[Block]) {
    let jump_graph = build_jump_graph(db, blocks);

    let mut blocks = BTreeMap::from_iter(blocks.iter().map(|b| (b.addr(), b.clone())));

    let (block_to_func, tail_call_funcs) = assign_block_to_func(&jump_graph, &blocks);

    let block_to_func = if !tail_call_funcs.is_empty() {
        for FuncAddr(addr) in tail_call_funcs {
            let Some(Block::Code(code)) = blocks.get_mut(&addr) else {
                continue;
            };

            if code.typ != CodeBlockTyp::Entry {
                eprintln!("Promoting {addr} to function");
                code.typ = CodeBlockTyp::Entry;
            }
        }

        build_jump_graph(db, &blocks.values().cloned().collect::<Vec<_>>());

        let (block_to_func, tail_call_funcs) = assign_block_to_func(&jump_graph, &blocks);

        if !tail_call_funcs.is_empty() {
            panic!(">> tail calls again: {:?}", tail_call_funcs);
        }
        block_to_func
    } else {
        block_to_func
    };

    println!("block to func:");
    for (block_addr, func) in BTreeMap::from_iter(block_to_func) {
        println!("    {block_addr} -> {func:?}");
    }
}

fn assign_block_to_func(
    jump_graph: &BlockGraph,
    blocks: &BTreeMap<Addr, Block>,
) -> (HashMap<Addr, FuncAddr>, HashSet<FuncAddr>) {
    let mut block_to_func: HashMap<Addr, FuncAddr> = HashMap::new();
    let mut identified_funcs = HashSet::new();

    let mut to_visit = vec![];
    for func_addr in jump_graph.externals() {
        if func_addr == Addr(0x6baff0) {
            eprintln!(">> Processing 0x6baff0");
        }

        if let Some(Block::Code(code)) = blocks.get(&func_addr)
            && code.typ != CodeBlockTyp::Entry
        {
            identified_funcs.insert(FuncAddr(func_addr));
        }

        to_visit.clear();
        to_visit.push(func_addr);

        while let Some(to_visit_addr) = to_visit.pop() {
            match block_to_func.entry(to_visit_addr) {
                Entry::Occupied(entry) => {
                    if entry.get() != &FuncAddr(func_addr) {
                        identified_funcs.insert(FuncAddr(to_visit_addr));
                    }
                    continue;
                }
                Entry::Vacant(vacant_entry) => {
                    vacant_entry.insert(FuncAddr(func_addr));
                }
            }

            block_to_func
                .entry(to_visit_addr)
                .or_insert_with(|| FuncAddr(func_addr));

            for succ in jump_graph.successors(to_visit_addr) {
                let Block::Code(block) = &blocks[&succ] else {
                    continue;
                };
                if block.typ() == CodeBlockTyp::Entry {
                    continue;
                }
                to_visit.push(succ);
            }
        }
    }

    (block_to_func, identified_funcs)
}
