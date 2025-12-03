use crate::{
    Binary,
    addr::Addr,
    block_set::{BlockSet, SplitError},
    ins::{self, BinaryInstruction, Instruction, Mem, Op},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeBlockType {
    Function,
    Entry,
    Jump,
    Filler,
}

#[derive(Debug, Clone, Copy)]
enum ToProcessType {
    Call,
    Jump,
    Indirect,
    JumpTable,
}

#[derive(Debug, Clone)]
pub struct CodeBlock {
    pub addr: Addr,
    pub size: usize,
    pub typ: CodeBlockType,
}

#[derive(Debug, Clone)]
pub struct JumpTable {
    pub addr: Addr,
    pub jumps: Vec<Addr>,
}

impl JumpTable {
    pub fn size(&self) -> usize {
        self.jumps.len() * 4
    }

    pub fn split(mut self, addr: Addr) -> Result<(Self, Self), Self> {
        let idx = addr.0.checked_sub(self.addr.0).unwrap();
        let idx = usize::try_from(idx).unwrap() / 4;

        if self.jumps.len() <= idx {
            return Err(self);
        }
        let right = self.jumps.split_off(idx);
        Ok((
            self,
            Self {
                addr: addr,
                jumps: right,
            },
        ))
    }

    fn contains(&self, addr: Addr) -> bool {
        let end = self.addr + u32::try_from(self.size()).unwrap();
        (self.addr..end).contains(&addr)
    }

    fn truncate(&mut self, size: usize) {
        let elements = size / 4;
        self.jumps.truncate(elements);
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
            Block::Code(code_block) => code_block.addr,
            Block::JumpTable(jump_table) => jump_table.addr,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Block::Code(code_block) => code_block.size,
            Block::JumpTable(jump_table) => jump_table.size(),
        }
    }

    pub fn split(self, addr: Addr, new_type: CodeBlockType) -> Result<(Self, Self), Self> {
        match self {
            Block::Code(code_block) => match code_block.split(addr, new_type) {
                Ok((left, right)) => Ok((Block::Code(left), Block::Code(right))),
                Err(block) => Err(Block::Code(block)),
            },
            Block::JumpTable(jump_table) => match jump_table.split(addr) {
                Ok((left, right)) => Ok((Block::JumpTable(left), Block::JumpTable(right))),
                Err(table) => Err(Block::JumpTable(table)),
            },
        }
    }

    pub fn contains(&self, addr: Addr) -> bool {
        match self {
            Block::Code(code_block) => code_block.contains(addr),
            Block::JumpTable(jump_table) => jump_table.contains(addr),
        }
    }
}

impl CodeBlock {
    pub fn split(mut self, addr: Addr, new_type: CodeBlockType) -> Result<(Self, Self), Self> {
        let left_size = addr.0.checked_sub(self.addr.0).unwrap();
        let left_size = usize::try_from(left_size).unwrap();
        let right_size = self.size - left_size;
        self.size = left_size;
        Ok((
            self,
            CodeBlock {
                addr,
                size: right_size,
                typ: new_type,
            },
        ))
    }

    fn contains(&self, addr: Addr) -> bool {
        let end = self.addr + u32::try_from(self.size).unwrap();
        (self.addr..end).contains(&addr)
    }
}

pub fn cut_blocks_as_sausage(binary: &Binary<'_>) -> Vec<Block> {
    let decoder = ins::Decoder::new(binary.sections.text.data, binary.sections.text.address);
    let mut instructions = Vec::with_capacity(2usize.pow(16));
    instructions.extend(decoder);

    let (blocks, to_process) = cut_slice_of_instructions(binary, &instructions);

    eprintln!(
        ">> debug block: {:?}",
        to_process.iter().find(|(addr, _)| *addr == Addr(0x422a45))
    );

    let blocks = process_blocks(binary, blocks, to_process);
    blocks
}

fn process_blocks(
    binary: &Binary<'_>,
    blocks: Vec<CodeBlock>,
    mut to_process: Vec<(Addr, ToProcessType)>,
) -> Vec<Block> {
    let mut blockset = BlockSet::new();
    for block in blocks {
        blockset.insert_non_overlaping_unchecked(Block::Code(block));
    }

    while let Some((addr, to_process_type)) = to_process.pop() {
        match to_process_type {
            ToProcessType::Call | ToProcessType::Jump | ToProcessType::Indirect => {
                let Ok(block) = blockset.get_split(addr, CodeBlockType::Filler) else {
                    continue;
                };

                if let Block::Code(block) = block {
                    match to_process_type {
                        ToProcessType::Call => block.typ = CodeBlockType::Function,
                        ToProcessType::Jump | ToProcessType::Indirect => {
                            block.typ = CodeBlockType::Jump
                        }
                        ToProcessType::JumpTable => unreachable!(),
                    }
                }
            }
            ToProcessType::JumpTable => {
                let mut jump_table = process_jump_table(addr, binary);
                to_process.extend(
                    jump_table
                        .jumps
                        .iter()
                        .map(|jump| (*jump, ToProcessType::Jump)),
                );

                match blockset.remove_split(jump_table.addr, jump_table.size()) {
                    Ok(block) => jump_table.truncate(block.size()),
                    Err(SplitError::OutOfRange) => {}
                    Err(SplitError::Unalighed) => panic!("unalighed jump table"),
                }

                blockset.insert_non_overlaping_unchecked(Block::JumpTable(jump_table));
            }
        }
    }

    blockset.into_iter().collect()
}

fn cut_slice_of_instructions(
    binary: &Binary<'_>,
    mut instructions: &[BinaryInstruction],
) -> (Vec<CodeBlock>, Vec<(Addr, ToProcessType)>) {
    let mut res = vec![];
    let mut to_process = vec![];

    while let Some((block, index)) = cut_block(binary, instructions, &mut to_process) {
        instructions = &instructions[index..];
        res.push(block);
    }

    (res, to_process)
}

fn cut_block(
    binary: &Binary<'_>,
    instructions: &[BinaryInstruction],
    to_process: &mut Vec<(Addr, ToProcessType)>,
) -> Option<(CodeBlock, usize)> {
    let first_instr = instructions.first().copied()?;

    if let Instruction::Int3 = first_instr.instr {
        return Some(cut_filler(instructions, first_instr));
    }

    let index = process_function_till_the_end_of_block(binary, instructions, to_process);
    let block = CodeBlock {
        addr: first_instr.addr,
        size: block_size(&instructions[..index]),
        typ: CodeBlockType::Jump,
    };

    Some((block, index))
}

fn block_size(instructions: &[BinaryInstruction]) -> usize {
    instructions.iter().map(|i| i.len).sum()
}

fn process_function_till_the_end_of_block(
    binary: &Binary<'_>,
    instructions: &[BinaryInstruction],
    to_process: &mut Vec<(Addr, ToProcessType)>,
) -> usize {
    let mut len = 0;
    for ins in instructions {
        len += 1;

        match ins.instr {
            ins::Instruction::CallNear(addr) => {
                to_process.push((addr, ToProcessType::Call));
            }
            ins::Instruction::CallMem(_addr) => {
                // TODO
            }
            ins::Instruction::Return(_) => break,
            ins::Instruction::Jump(Op::Addr(addr)) => {
                to_process.push((addr, ToProcessType::Jump));
                break;
            }
            ins::Instruction::Jump(Op::Mem(mem)) => {
                if looks_like_jump_table(&mem) && binary.sections.text.contains(Addr(mem.disp)) {
                    to_process.push((Addr(mem.disp), ToProcessType::JumpTable));
                }

                break;
            }
            ins::Instruction::JumpConditional(Op::Addr(addr), _) => {
                to_process.push((addr, ToProcessType::Jump));
            }
            ins::Instruction::JumpConditional(Op::Mem(mem), _) => {
                if looks_like_jump_table(&mem) && binary.sections.text.contains(Addr(mem.disp)) {
                    to_process.push((Addr(mem.disp), ToProcessType::JumpTable));
                }
            }
            ins::Instruction::Loop(addr) => {
                to_process.push((addr, ToProcessType::Jump));
            }
            ins::Instruction::PushImm(imm) => {
                if binary.sections.text.contains(Addr(imm)) {
                    to_process.push((Addr(imm), ToProcessType::Indirect));
                }
            }
            ins::Instruction::MovImm(imm) => {
                if let Ok(imm) = imm.try_into() {
                    let addr = Addr(imm);
                    if binary.sections.text.contains(addr) {
                        to_process.push((addr, ToProcessType::Indirect));
                    }
                }
            }
            ins::Instruction::Int3 => {
                // Go back 1 instruction
                len -= 1;
            }
            ins::Instruction::IcedX86 => {}
        }
    }

    len
}

fn cut_filler(
    instructions: &[BinaryInstruction],
    first_instr: BinaryInstruction,
) -> (CodeBlock, usize) {
    let idx = instructions
        .iter()
        .position(|ins| !matches!(ins.instr, Instruction::Int3))
        .unwrap_or(instructions.len());

    let block = CodeBlock {
        addr: first_instr.addr,
        size: block_size(&instructions[..idx]),
        typ: CodeBlockType::Filler,
    };
    (block, idx)
}

fn process_jump_table(addr: Addr, binary: &Binary<'_>) -> JumpTable {
    let mut addr_counter = addr;
    let mut jumps = vec![];
    loop {
        let target = Addr(binary.sections.text.read_u32_le(addr_counter));
        if !binary.sections.text.contains(target) {
            break JumpTable { addr, jumps };
        }
        jumps.push(target);
        addr_counter += 4;
    }
}

pub fn looks_like_jump_table(mem: &Mem) -> bool {
    !mem.is_base && mem.is_index && mem.scale == 4
}
