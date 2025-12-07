use std::collections::BTreeSet;

use iced_x86::Decoder;
use instr::{
    addr::Addr,
    cfg::{Block, BlockType},
    cfg_func::GraphFunctionCollector,
    ins::{Instruction, parse_instruction},
    instruction_signature, instruction_signature_full, parse_binary,
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
        match block.typ() {
            BlockType::Function => funcs += 1,
            BlockType::Entry => funcs += 1,
            BlockType::Jump => jumps += 1,
            BlockType::Filler => filler += 1,
            BlockType::JumpTable => jump_table += 1,
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

    let mut blocks = instr::cfg::cut_blocks_as_sausage(&binary);

    eprintln!(">> Promoting function based on .rdata");
    instr::cfg_func::promote_functions_based_on_object_function_refs(
        &binary.rdata_objects,
        &mut blocks,
    );
    eprintln!(">> Promoting function based on .data");
    instr::cfg_func::promote_functions_based_on_object_function_refs(
        &binary.data_objects,
        &mut blocks,
    );
    instr::cfg_func::promote_functions_based_on_fillers(&mut blocks);

    let mut graph_collector = GraphFunctionCollector::with_capacity(blocks.len());
    graph_collector.insert_nodes(&blocks);
    graph_collector.insert_edges(&binary, &blocks);
    // eprintln!(">> Promoting functions based on call graph and tail calls");
    // graph_collector.promote_functions_based_on_tail_calls(&mut blocks);
    // graph_collector.print_block_to_func(&blocks);

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

        match block.typ() {
            BlockType::Entry => {
                println!("_start ({size}):");
                print_asm(&binary, addr, size, Some(&mut unique_instructions));
            }
            BlockType::Function => {
                println!("_func_{:x} ({size}):", addr.0);
                print_asm(&binary, addr, size, Some(&mut unique_instructions));
            }
            BlockType::Jump => {
                println!("_jump_{:x} ({size}):", addr.0);
                print_asm(&binary, addr, size, Some(&mut unique_instructions));
            }
            BlockType::Filler => {
                println!("_filler_{:x} ({size}):", addr.0);
                print_asm(&binary, addr, size, Some(&mut unique_instructions));
            }

            BlockType::JumpTable => {
                println!("_jump_table_{:x} ({}):", addr.0, size);
                for entry in block.jump_targets(&binary).unwrap() {
                    println!("   - {} -> {}", entry.addr, entry.target);
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
