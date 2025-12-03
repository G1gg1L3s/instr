use crate::{
    Binary,
    addr::Addr,
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

enum ToProcessType {
    Call,
    Jump,
    Indirect,
    JumpTable,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub addr: Addr,
    pub size: usize,
    pub typ: BlockType,
    pub ins: Vec<BinaryInstruction>,
}

pub fn cut_blocks_as_sausage(binary: &Binary<'_>) -> Vec<Block> {
    let decoder = ins::Decoder::new(binary.sections.text.data, binary.sections.text.address);
    let mut instructions = Vec::with_capacity(2usize.pow(16));
    instructions.extend(decoder);

    let (blocks, to_process) = cut_slice_of_instructions(binary, &instructions);
    blocks
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
    let instructions = instructions[..index].to_vec();
    let block = Block {
        addr: first_instr.addr,
        size: instructions.iter().map(|i| i.len).sum(),
        typ: BlockType::Jump,
        ins: instructions,
    };

    Some((block, index))
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
) -> (Block, usize) {
    let idx = instructions
        .iter()
        .position(|ins| !matches!(ins.instr, Instruction::Int3))
        .unwrap_or(instructions.len());

    let instructions = instructions[..idx].to_vec();

    let block = Block {
        addr: first_instr.addr,
        size: instructions.iter().map(|i| i.len).sum(),
        typ: BlockType::Filler,
        ins: instructions,
    };
    (block, idx)
}

// fn process_jump_table(
//     mut addr: Addr,
//     binary: &Binary<'_>,
//     to_process: &mut Vec<(Addr, ToProcessType)>,
// ) -> usize {
//     let start_addr = addr;
//     loop {
//         let target = Addr(binary.sections.text.read_u32_le(addr));
//         if !binary.sections.text.contains(target) {
//             return usize::try_from(addr.0 - start_addr.0).unwrap();
//         }
//         to_process.push((target, ToProcessType::Known(BlockType::Jump)));
//         addr += 4;
//     }
// }

// fn derive_functions_from_data_objects(
//     robjects: &[DataObject],
//     result: &mut Vec<(Addr, ToProcessType)>,
// ) {
//     for obj in robjects {
//         if let DataObjectKind::FunctionsRef(addr) = &obj.kind {
//             result.push((*addr, ToProcessType::Known(BlockType::Function)));
//         }
//     }
// }

pub fn looks_like_jump_table(mem: &Mem) -> bool {
    !mem.is_base && mem.is_index && mem.scale == 4
}
