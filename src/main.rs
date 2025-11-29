use std::collections::BTreeSet;

use iced_x86::Decoder;
use instr::{
    addr::Addr, collect_rdata_objects, extract_call_address, instruction_signature, parse_binary,
};

fn main() {
    let data = std::fs::read("../../Barnyard/Barnyard.exe").unwrap();
    let pe = pe_parser::pe::parse_portable_executable(&data).unwrap();
    let binary = parse_binary(&data, &pe).unwrap();

    println!("Binary: {:#?}", binary);

    let robjects = collect_rdata_objects(&binary.sections);
    for obj in robjects {
        println!(">> {:?}", obj);
    }

    let mut decoder = Decoder::with_ip(
        32,
        binary.sections.text.data,
        binary.sections.text.address.0.into(),
        iced_x86::DecoderOptions::NONE,
    );

    let mut variants = BTreeSet::new();

    for instr in &mut decoder {
        let sig = instruction_signature(&instr);
        let instrformat = instr.to_string();
        eprintln!("0x{:x} {: <30} | {}", instr.ip(), instrformat, sig);
        variants.insert(sig);

        if let Some(addr) = extract_call_address(&instr) {
            eprintln!(
                ">> 0x{addr:x} (addr in .text: {})",
                binary.sections.text.contains(Addr(addr as _))
            );
        }
    }

    // ----- PRINT RESULTS -----
    println!("\nUnique instruction variants: {}", variants.len());
    for v in variants {
        println!("{}", v);
    }
}
