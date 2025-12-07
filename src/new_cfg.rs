use std::collections::BTreeMap;

use crate::{
    SectionData,
    addr::Addr,
    cfg::looks_like_jump_table,
    ins::{BinaryInstruction, Decoder, Instruction, Op},
    obj::{ObjDatabase, ObjectTyp},
};

#[derive(Debug)]
pub struct CodeBlock {
    code: Vec<BinaryInstruction>,
    len: usize,
}

impl CodeBlock {
    pub fn new(code: Vec<BinaryInstruction>) -> Self {
        Self {
            len: code.iter().map(|i| i.len).sum(),
            code,
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

        (Self::new(self.code), tail)
    }
}

#[derive(Default)]
pub struct BlockTree {
    tree: BTreeMap<Addr, CodeBlock>,
}

impl BlockTree {
    pub fn insert(&mut self, block: CodeBlock) {
        self.tree.insert(block.addr(), block);
    }

    pub fn get(&mut self, addr: Addr) -> Option<&CodeBlock> {
        self.tree.get(&addr)
    }

    pub fn get_with_addr(&mut self, addr: Addr) -> Option<&CodeBlock> {
        let (_, block) = self.tree.range(..=addr).next_back()?;

        if block.to_range().contains(&addr) {
            Some(block)
        } else {
            None
        }
    }

    pub fn remove_with_addr(&mut self, addr: Addr) -> Option<CodeBlock> {
        let block = self.get_with_addr(addr)?;
        let addr = block.addr();
        self.tree.remove(&addr)
    }

    pub fn next_after(&self, addr: Addr) -> Option<&CodeBlock> {
        self.tree.range(addr..).next().map(|(_, v)| v)
    }
}

pub fn walk_code_blocks(
    db: &ObjDatabase,
    text: SectionData<'_>,
    start: Addr,
) -> BTreeMap<Addr, CodeBlock> {
    let mut queue = vec![start];
    let mut res = BlockTree::default();

    while let Some(addr) = queue.pop() {
        if res.get(addr).is_some() {
            continue;
        }

        if let Some(block) = res.remove_with_addr(addr) {
            let (left, right_code) = block.split(addr);
            let right = CodeBlock::new(right_code);

            res.insert(left);
            res.insert(right);
            continue;
        }

        let limit = if let Some(next_after) = res.next_after(addr) {
            Some(usize::try_from(next_after.addr().0 - addr.0).unwrap())
        } else {
            None
        };

        let WalkedCodeBlock { code, successors } = walk_block(db, text, addr, limit);
        let block = CodeBlock::new(code);

        for succ in successors {
            match succ {
                CodeBlockSucc::Jump(addr) => queue.push(addr),
                CodeBlockSucc::JumpCond(addr) => queue.push(addr),
                CodeBlockSucc::Call(addr) => queue.push(addr),
                CodeBlockSucc::JumpTable(addr) => {}
                CodeBlockSucc::Fallthrough => queue.push(block.end()),
            }
        }

        res.insert(block);
    }

    res.tree
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
                if let Some(obj) = mem.to_absolute().map(Addr).and_then(|addr| db.get(addr)) {
                    if let ObjectTyp::ImportThunk(import) = obj.typ() {
                        if import.is_terminating {
                            break;
                        }
                    }
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
