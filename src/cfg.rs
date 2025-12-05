use crate::{
    Binary,
    addr::Addr,
    block_set::BlockSet,
    ins::{self, BinaryInstruction, Instruction, Mem, Op},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Function,
    Entry,
    Jump,
    Filler,
    JumpTable,
}

#[derive(Debug, Clone, Copy)]
enum ToProcessType {
    Call,
    Jump,
    Indirect,
    JumpTable,
    Entry,
}

#[derive(Debug, Clone, Copy)]
pub struct Block {
    addr: Addr,
    size: usize,
    typ: BlockType,
}

pub struct JumpTableEntry {
    pub addr: Addr,
    pub target: Addr,
}

impl Block {
    pub fn new(addr: Addr, size: usize, typ: BlockType) -> Self {
        Self { addr, size, typ }
    }

    pub fn addr(&self) -> Addr {
        self.addr
    }

    pub fn end_addr(&self) -> Addr {
        self.addr + u32::try_from(self.size).unwrap()
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn typ(&self) -> BlockType {
        self.typ
    }

    pub fn set_typ(&mut self, typ: BlockType) {
        self.typ = typ;
    }

    pub fn split(mut self, addr: Addr, new_type: BlockType) -> Result<(Self, Self), Self> {
        let left_size = addr.0.checked_sub(self.addr.0).unwrap();
        let left_size = usize::try_from(left_size).unwrap();
        let right_size = self.size - left_size;
        self.size = left_size;
        Ok((
            self,
            Block {
                addr,
                size: right_size,
                typ: new_type,
            },
        ))
    }

    pub fn contains(&self, addr: Addr) -> bool {
        let end = self.addr + u32::try_from(self.size).unwrap();
        (self.addr..end).contains(&addr)
    }

    pub fn jump_targets(
        &self,
        binary: &Binary<'_>,
    ) -> Option<impl Iterator<Item = JumpTableEntry>> {
        if self.typ != BlockType::JumpTable {
            return None;
        }

        let slice = binary.sections.text.slice(self.addr, self.size);
        let (chunks, _) = slice.as_chunks::<4>();

        Some(chunks.iter().enumerate().map(|(i, chunk)| {
            let u32 = u32::from_le_bytes(*chunk);
            let target = Addr(u32);
            let addr = self.addr + u32::try_from(i).unwrap() * 4;
            JumpTableEntry { addr, target }
        }))
    }
}

pub fn cut_blocks_as_sausage(binary: &Binary<'_>) -> Vec<Block> {
    let decoder = ins::Decoder::new(binary.sections.text.data, binary.sections.text.address);
    let mut instructions = Vec::with_capacity(2usize.pow(16));
    instructions.extend(decoder);

    let (blocks, mut to_process) = cut_slice_of_instructions(binary, &instructions);

    let jump_tables = process_jump_tables(binary, &mut to_process);
    to_process.push((binary.entry_point, ToProcessType::Entry));

    let blocks = process_blocks(binary, blocks, jump_tables, to_process);
    blocks
}

fn process_jump_tables(
    binary: &Binary<'_>,
    to_process: &mut Vec<(Addr, ToProcessType)>,
) -> Vec<Block> {
    let mut res = vec![];
    for (addr, _) in to_process.extract_if(.., |(_, typ)| matches!(typ, ToProcessType::JumpTable)) {
        let Some(jump_table) = process_jump_table(addr, binary) else {
            continue;
        };
        res.push(jump_table);
    }
    res
}

fn process_blocks(
    binary: &Binary<'_>,
    blocks: Vec<Block>,
    jump_tables: Vec<Block>,
    mut to_process: Vec<(Addr, ToProcessType)>,
) -> Vec<Block> {
    let mut blockset = BlockSet::new();
    for block in blocks {
        blockset.insert_non_overlaping_unchecked(block);
    }

    for jump_table in jump_tables {
        to_process.extend(
            jump_table
                .jump_targets(binary)
                .unwrap()
                .map(|jump| (jump.target, ToProcessType::Jump)),
        );

        blockset.split_range(jump_table.addr(), jump_table.size);
        let table = blockset.split_at(jump_table.addr()).unwrap();
        table.typ = BlockType::JumpTable;
    }

    while let Some((addr, to_process_type)) = to_process.pop() {
        match to_process_type {
            ToProcessType::Call
            | ToProcessType::Jump
            | ToProcessType::Indirect
            | ToProcessType::Entry => {
                let Some(block) = blockset.split_at(addr) else {
                    continue;
                };

                block.typ = BlockType::Filler;
                match to_process_type {
                    ToProcessType::Call | ToProcessType::Indirect => {
                        block.typ = BlockType::Function
                    }
                    ToProcessType::Jump => block.typ = BlockType::Jump,
                    ToProcessType::Entry => block.typ = BlockType::Entry,
                    ToProcessType::JumpTable => unreachable!(),
                }
            }
            ToProcessType::JumpTable => {
                unreachable!()
            }
        }
    }

    blockset.into_iter().collect()
}

fn cut_slice_of_instructions(
    binary: &Binary<'_>,
    mut instructions: &[BinaryInstruction],
) -> (Vec<Block>, Vec<(Addr, ToProcessType)>) {
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
) -> Option<(Block, usize)> {
    let first_instr = instructions.first().copied()?;

    if let Instruction::Int3 = first_instr.instr {
        return Some(cut_filler(instructions, first_instr));
    }

    let index = process_function_till_the_end_of_block(binary, instructions, to_process);
    let block = Block {
        addr: first_instr.addr,
        size: block_size(&instructions[..index]),
        typ: BlockType::Jump,
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
    if instructions[0].addr == Addr(0x404360) {
        eprintln!("processing 0x404360");
    }

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
                break;
            }
            ins::Instruction::IcedX86 => {}
        }
    }

    len
}

fn cut_filler(
    instructions: &[BinaryInstruction],
    first_instr: BinaryInstruction,
) -> (Block, usize) {
    let idx = instructions
        .iter()
        .position(|ins| !matches!(ins.instr, Instruction::Int3))
        .unwrap_or(instructions.len());

    let block = Block {
        addr: first_instr.addr,
        size: block_size(&instructions[..idx]),
        typ: BlockType::Filler,
    };
    (block, idx)
}

fn process_jump_table(addr: Addr, binary: &Binary<'_>) -> Option<Block> {
    let mut i = 0;
    loop {
        let addr_counter = addr + i * 4;
        let target = Addr(binary.sections.text.read_u32_le(addr_counter));
        if !binary.sections.text.contains(target) {
            break;
        }
        i += 1;
    }

    if i >= 3 {
        Some(Block {
            addr,
            size: usize::try_from(i).unwrap() * 4,
            typ: BlockType::JumpTable,
        })
    } else {
        None
    }
}

pub fn looks_like_jump_table(mem: &Mem) -> bool {
    !mem.is_base && mem.is_index && mem.scale == 4
}
