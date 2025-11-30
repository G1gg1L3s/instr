use std::{collections::BTreeMap, ops::Range};

use crate::{
    Binary, DataObject, DataObjectKind,
    addr::Addr,
    ins::{self, Mem, Op},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Entry,
    Function,
    Jump,
    Indirect,
    JumpTable,
}

enum ToProcessType {
    Code(BlockType),
    JumpTable,
}

#[derive(Debug, Clone, Copy)]
pub struct Block {
    pub typ: BlockType,
    pub size: Option<usize>,
}

impl Block {
    pub fn size_u32_assert(&self) -> u32 {
        self.size.unwrap_or(0).try_into().unwrap()
    }
}

pub fn derive_blocks(binary: &Binary<'_>) -> BTreeMap<Addr, Block> {
    let mut to_process = Vec::with_capacity(64_000);
    derive_functions_from_data_objects(&binary.rdata_objects, &mut to_process);
    derive_functions_from_data_objects(&binary.data_objects, &mut to_process);
    to_process.push((binary.entry_point, ToProcessType::Code(BlockType::Entry)));

    let mut processed = BTreeMap::<Addr, Block>::new();

    while let Some((addr, block_type)) = to_process.pop() {
        if processed.contains_key(&addr) {
            continue;
        }

        match block_type {
            ToProcessType::Code(block_type) => {
                let size = if binary.sections.text.contains(addr) {
                    let code = binary.sections.text.slice_to_end(addr);
                    let size = derive_blocks_from_function(code, addr, binary, &mut to_process);
                    Some(size)
                } else {
                    None
                };

                processed.insert(
                    addr,
                    Block {
                        typ: block_type,
                        size,
                    },
                );
            }
            ToProcessType::JumpTable => {
                let size = process_jump_table(addr, binary, &mut to_process);
                if size != 0 {
                    processed.insert(
                        addr,
                        Block {
                            typ: BlockType::JumpTable,
                            size: Some(size),
                        },
                    );
                }
            }
        }
    }

    normalise(processed)
}

struct NormaliseOverlap {
    blocks: [Range<Addr>; 3],
}

fn normalise_overlap(block1: Range<Addr>, block2: Range<Addr>) -> Option<NormaliseOverlap> {
    if are_overlapping(&block1, &block2) {
        let mut values = [block1.start, block1.end, block2.start, block2.end];
        values.sort();

        Some(NormaliseOverlap {
            blocks: [
                values[0]..values[1],
                values[1]..values[2],
                values[2]..values[3],
            ],
        })
    } else {
        None
    }
}

fn are_overlapping(block1: &Range<Addr>, block2: &Range<Addr>) -> bool {
    block1.start < block2.end && block2.start < block1.end
}

fn normalise(mut blocks: BTreeMap<Addr, Block>) -> BTreeMap<Addr, Block> {
    let mut new = BTreeMap::new();

    let Some((mut prev_addr, mut prev_block)) = blocks.pop_first() else {
        return new;
    };

    for (next_addr, next_block) in blocks {
        let prev_range = prev_addr..(prev_addr + prev_block.size_u32_assert());
        let next_range = next_addr..(next_addr + next_block.size_u32_assert());

        if let Some(normalised) = normalise_overlap(prev_range, next_range) {
            for (i, normalised_block) in normalised
                .blocks
                .into_iter()
                .filter(|block| !block.is_empty())
                .enumerate()
            {
                let size = normalised_block.end.0 - normalised_block.start.0;
                let block = Block {
                    typ: if i == 0 {
                        prev_block.typ
                    } else {
                        BlockType::Jump
                    },
                    size: Some(usize::try_from(size).unwrap()),
                };
                new.insert(normalised_block.start, block);

                prev_addr = normalised_block.start;
                prev_block = block;
            }
        } else {
            new.insert(prev_addr, prev_block);
            prev_addr = next_addr;
            prev_block = next_block;
        }
    }

    new
}

fn derive_blocks_from_function(
    code: &[u8],
    addr: Addr,
    binary: &Binary<'_>,
    to_process: &mut Vec<(Addr, ToProcessType)>,
) -> usize {
    let mut decoder = ins::Decoder::new(code, addr);

    for ins in &mut decoder {
        match ins.instr {
            ins::Instruction::CallNear(addr) => {
                to_process.push((addr, ToProcessType::Code(BlockType::Function)));
            }
            ins::Instruction::CallMem(addr) => {
                to_process.push((addr, ToProcessType::Code(BlockType::Function)));
            }
            // End of block
            ins::Instruction::Return(_) => return decoder.position(),
            ins::Instruction::Jump(Op::Addr(addr)) => {
                to_process.push((addr, ToProcessType::Code(BlockType::Jump)));
                return decoder.position();
            }
            ins::Instruction::Jump(Op::Mem(mem)) => {
                if looks_like_jump_table(&mem) && binary.sections.text.contains(Addr(mem.disp)) {
                    to_process.push((Addr(mem.disp), ToProcessType::JumpTable));
                }

                return decoder.position();
            }
            ins::Instruction::JumpConditional(Op::Addr(addr), _) => {
                to_process.push((addr, ToProcessType::Code(BlockType::Jump)));
                to_process.push((decoder.address(), ToProcessType::Code(BlockType::Jump)));
                return decoder.position();
            }
            ins::Instruction::JumpConditional(Op::Mem(mem), _) => {
                if looks_like_jump_table(&mem) && binary.sections.text.contains(Addr(mem.disp)) {
                    to_process.push((Addr(mem.disp), ToProcessType::JumpTable));
                }
                return decoder.position();
            }
            ins::Instruction::Loop(addr) => {
                to_process.push((addr, ToProcessType::Code(BlockType::Jump)));
                to_process.push((decoder.address(), ToProcessType::Code(BlockType::Jump)));
                return decoder.position();
            }
            ins::Instruction::PushImm(imm) => {
                if binary.sections.text.contains(Addr(imm)) {
                    to_process.push((Addr(imm), ToProcessType::Code(BlockType::Indirect)));
                }
            }
            ins::Instruction::MovImm(imm) => {
                if let Ok(imm) = imm.try_into() {
                    let addr = Addr(imm);
                    if binary.sections.text.contains(addr) {
                        to_process.push((addr, ToProcessType::Code(BlockType::Indirect)));
                    }
                }
            }
        }
    }
    decoder.position()
}

fn process_jump_table(
    mut addr: Addr,
    binary: &Binary<'_>,
    to_process: &mut Vec<(Addr, ToProcessType)>,
) -> usize {
    let start_addr = addr;
    loop {
        let target = Addr(binary.sections.text.read_u32_le(addr));
        if !binary.sections.text.contains(target) {
            return usize::try_from(addr.0 - start_addr.0).unwrap();
        }
        to_process.push((target, ToProcessType::Code(BlockType::Jump)));
        addr += 4;
    }
}

fn derive_functions_from_data_objects(
    robjects: &[DataObject],
    result: &mut Vec<(Addr, ToProcessType)>,
) {
    for obj in robjects {
        if let DataObjectKind::FunctionsRef(addr) = &obj.kind {
            result.push((*addr, ToProcessType::Code(BlockType::Function)));
        }
    }
}

pub fn looks_like_jump_table(mem: &Mem) -> bool {
    !mem.is_base && mem.is_index && mem.scale == 4
}
