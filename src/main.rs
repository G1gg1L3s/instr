use iced_x86::Decoder;
use instr::{
    addr::Addr, cfg::BlockType, ins::parse_instruction, instruction_signature, parse_binary,
};

fn main() {
    let data = std::fs::read("../../Barnyard/Barnyard.exe").unwrap();
    let pe = pe_parser::pe::parse_portable_executable(&data).unwrap();
    let binary = parse_binary(&data, &pe).unwrap();

    println!(
        "Binary: entry: {} {:#?}",
        binary.entry_point, binary.sections
    );

    let blocks = instr::cfg::derive_blocks(&binary);
    println!(
        "Total {} blocks ({} funcs)",
        blocks.len(),
        blocks
            .iter()
            .filter(|(_, block)| matches!(block.typ, BlockType::Function | BlockType::Entry))
            .count()
    );

    let mut last_addr_end = None;

    for (addr, block) in blocks {
        let Some(size) = block.size else {
            println!("!! {} not in text", addr);
            continue;
        };

        if let Some(skipped) = last_addr_end.map(|last: Addr| addr.0.checked_sub(last.0))
            && skipped != Some(0)
        {
            if let Some(skipped) = skipped {
                println!("... Skipped 0x{:x} ({}) bytes ...", skipped, skipped);
            } else {
                println!("... OVERLAP");
            }
        }

        last_addr_end = Some(addr + size as u32);

        let code = binary.sections.text.slice(addr, size);
        let decoder = Decoder::with_ip(32, code, addr.0.into(), iced_x86::DecoderOptions::NONE);

        match block.typ {
            instr::cfg::BlockType::Entry => println!("_start: ({size}):"),
            instr::cfg::BlockType::Function => println!("_func_{:x} ({size}):", addr.0),
            instr::cfg::BlockType::Jump => println!("_block_{:x} ({size}):", addr.0),
        }

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
}
