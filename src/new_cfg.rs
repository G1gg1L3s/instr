use std::collections::BTreeMap;

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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
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
