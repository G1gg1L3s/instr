use iced_x86::Decoder;
use instr::{
    addr::Addr,
    cfg::{Block, BlockType},
    ins::parse_instruction,
    instruction_signature, parse_binary,
};

struct CountBlocks {
    funcs: usize,
    jumps: usize,
    indirect: usize,
    jump_table: usize,
}

fn count_blocks<'a>(iter: impl Iterator<Item = &'a Block>) -> CountBlocks {
    let mut funcs = 0;
    let mut jumps = 0;
    let mut indirect = 0;
    let mut jump_table = 0;
    for block in iter {
        match block.typ {
            BlockType::Entry | BlockType::Function => funcs += 1,
            BlockType::Jump => jumps += 1,
            BlockType::Indirect => indirect += 1,
            BlockType::JumpTable => jump_table += 1,
        }
    }
    CountBlocks {
        funcs,
        jumps,
        indirect,
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

    let blocks = instr::cfg::derive_blocks(&binary);
    let count = count_blocks(blocks.values());

    println!(
        "Total {} blocks ({} funcs, {} jumps, {} indirect, {} jump tables)",
        blocks.len(),
        count.funcs,
        count.jumps,
        count.indirect,
        count.jump_table
    );

    let mut last_addr_end = Option::<Addr>::None;

    for (addr, block) in blocks {
        let Some(size) = block.size else {
            println!("!! {} not in text", addr);
            continue;
        };

        if let Some(last_addr_end) = last_addr_end {
            match addr.0.checked_sub(last_addr_end.0) {
                Some(0) => {} // OK
                Some(skipped) => {
                    println!("_skipped {} bytes:", skipped);
                    print_asm(&binary, last_addr_end, skipped.try_into().unwrap());
                }
                _ => println!("... OVERLAP"),
            }
        }

        last_addr_end = Some(addr + size as u32);

        if block.typ == BlockType::JumpTable {
            println!("_jump_table ({size}):");
            let table = binary.sections.text.slice(addr, size);
            let mut line_addr = addr;
            for target in table.as_chunks::<4>().0 {
                let target = Addr(u32::from_le_bytes(*target));
                println!("   - {line_addr} -> {target}");
                line_addr += 4;
            }
            continue;
        }

        match block.typ {
            instr::cfg::BlockType::Entry => println!("_start ({size}):"),
            instr::cfg::BlockType::Function => println!("_func_{:x} ({size}):", addr.0),
            instr::cfg::BlockType::Jump => println!("_block_{:x} ({size}):", addr.0),
            instr::cfg::BlockType::Indirect => println!("_indirect_{:x} ({size}):", addr.0),
            instr::cfg::BlockType::JumpTable => println!("_jump_table_{:x} ({size}):", addr.0),
        }

        print_asm(&binary, addr, size);
    }

    println!(".rdata:");
    for obj in &binary.rdata_objects {
        println!("  - {:?}", obj);
    }

    println!(".data:");
    for obj in &binary.data_objects {
        println!("  - {:?}", obj);
    }
}

fn print_asm(binary: &instr::Binary<'_>, addr: Addr, size: usize) {
    let code = binary.sections.text.slice(addr, size);
    let decoder = Decoder::with_ip(32, code, addr.0.into(), iced_x86::DecoderOptions::NONE);

    for ins in decoder {
        let sig = instruction_signature(&ins);
        let instrformat = ins.to_string();
        let internal = parse_instruction(&ins);
        if let Some(internal) = internal {
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
    }
}
