use std::collections::BTreeSet;

use iced_x86::Decoder;
use instr::{
    addr::Addr,
    ins::{Instruction, parse_instruction},
    instruction_signature, instruction_signature_full,
    new_cfg::{Block, CodeBlockType},
    parse_binary,
};

struct CountBlocks {
    funcs: usize,
    jumps: usize,
    filler: usize,
    jump_table: usize,
}

fn count_blocks<'a>(iter: impl Iterator<Item = &'a Block>) -> CountBlocks {
    let mut funcs = 0;
    let mut jumps = 0;
    let mut filler = 0;
    let mut jump_table = 0;
    for block in iter {
        match block {
            Block::Code(code_block) => match code_block.typ {
                CodeBlockType::Function => funcs += 1,
                CodeBlockType::Entry => funcs += 1,
                CodeBlockType::Jump => jumps += 1,
                CodeBlockType::Filler => filler += 1,
            },
            Block::JumpTable(_) => jump_table += 1,
        }
    }
    CountBlocks {
        funcs,
        jumps,
        filler,
        jump_table,
    }
}

fn main() {
    let data = std::fs::read("../../Barnyard/Barnyard.exe").unwrap();
    let pe = pe_parser::pe::parse_portable_executable(&data).unwrap();
    let binary = parse_binary(&data, &pe).unwrap();

    println!(
        "Binary: entry: {} {:#?}",
        binary.entry_point, binary.sections
    );

    let blocks = instr::new_cfg::cut_blocks_as_sausage(&binary);
    let count = count_blocks(blocks.iter());

    println!(
        "Total {} blocks ({} funcs, {} jumps, {} fillers, {} jump tables)",
        blocks.len(),
        count.funcs,
        count.jumps,
        count.filler,
        count.jump_table
    );

    let mut last_addr_end = binary.sections.text.address;

    let mut unique_instructions = BTreeSet::new();

    for block in blocks {
        match block.addr().0.checked_sub(last_addr_end.0) {
            Some(0) => {} // OK
            Some(skipped) => {
                println!("_skipped {} bytes:", skipped);
                print_asm(&binary, last_addr_end, skipped.try_into().unwrap(), None);
            }
            _ => println!("... OVERLAP"),
        }

        let addr = block.addr();
        let size = block.size();
        last_addr_end = block.addr() + block.size() as u32;

        match block {
            Block::Code(code_block) => {
                match code_block.typ {
                    CodeBlockType::Entry => println!("_start ({size}):"),
                    CodeBlockType::Function => println!("_func_{:x} ({size}):", addr.0),
                    CodeBlockType::Jump => println!("_jump_{:x} ({size}):", addr.0),
                    CodeBlockType::Filler => println!("_filler_{:x} ({size}):", addr.0),
                }
                print_asm(&binary, addr, size, Some(&mut unique_instructions));
            }
            Block::JumpTable(jump_table) => {
                println!("_jump_table_{:x} ({}):", jump_table.addr.0, size);
                let mut line_addr = addr;
                for target in &jump_table.jumps {
                    println!("   - {line_addr} -> {target}");
                    line_addr += 4;
                }
            }
        }
    }

    println!(".rdata:");
    for obj in &binary.rdata_objects {
        println!("  - {:?}", obj);
    }

    println!(".data:");
    for obj in &binary.data_objects {
        println!("  - {:?}", obj);
    }

    println!(".unique_instructions ({}):", unique_instructions.len());
    for ins in unique_instructions {
        println!("  - {}", ins);
    }
}

fn print_asm(
    binary: &instr::Binary<'_>,
    addr: Addr,
    size: usize,
    mut out: Option<&mut BTreeSet<String>>,
) {
    let code = binary.sections.text.slice(addr, size);
    let decoder = Decoder::with_ip(32, code, addr.0.into(), iced_x86::DecoderOptions::NONE);

    for ins in decoder {
        let sig = instruction_signature_full(&ins);
        let instrformat = ins.to_string();
        let internal = parse_instruction(&ins);
        if !matches!(internal, Instruction::IcedX86) {
            println!(
                "    0x{:x} {: <30} | {} | {:?}",
                ins.ip(),
                instrformat,
                sig,
                internal
            );
        } else {
            println!(
                "    0x{:x} {: <30} | {} ({:?})",
                ins.ip(),
                instrformat,
                sig,
                ins.mnemonic()
            );
        }

        if let Some(out) = &mut out {
            out.insert(instruction_signature(&ins));
        }
    }
}
