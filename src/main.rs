use std::collections::BTreeSet;

use iced_x86::Decoder;
use instr::{
    SectionData,
    addr::Addr,
    cfg::{Block, BlockType},
    cfg_func::GraphFunctionCollector,
    ins::{Instruction, Op, parse_instruction},
    instruction_signature, instruction_signature_full, lir, new_cfg,
    obj::{self, ObjDatabase, Object, ObjectTyp},
    parse_binary, third_cfg,
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

struct AsHexdump<'a>(&'a [u8]);

impl<'a> std::fmt::Display for AsHexdump<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for word in self.0.chunks(4) {
            for b in word {
                write!(f, "{:02x} ", b)?;
            }
        }

        write!(f, "| ")?;

        for (i, word) in self.0.chunks(4).enumerate() {
            if i != 0 {
                write!(f, " ")?;
            }

            for c in word {
                if c.is_ascii_graphic() {
                    write!(f, "{}", *c as char)?;
                } else {
                    write!(f, ".")?;
                }
            }
        }
        Ok(())
    }
}

fn main() {
    env_logger::init();

    let data = std::fs::read("../../Barnyard/Barnyard.exe").unwrap();
    let pe = pe_parser::pe::parse_portable_executable(&data).unwrap();
    let binary = parse_binary(&data, &pe).unwrap();

    println!(
        "Binary: entry: {} {:#?}",
        binary.entry_point, binary.sections
    );

    let mut db = obj::ObjDatabase::new();
    let data_strings = instr::string::collect_data_strings(binary.sections.data);
    let rdata_strings = instr::string::collect_data_strings(binary.sections.rdata);

    for string in data_strings.into_iter().chain(rdata_strings) {
        db.insert(Object::new(string.addr, ObjectTyp::String(string.typ)));
    }
    for lib in &binary.imports {
        obj::fill_database_with_import(&mut db, lib);
    }

    let blocks = third_cfg::walk_code_blocks(binary.sections.text, binary.entry_point);

    let functions = third_cfg::derive_functions(&blocks, binary.entry_point);

    for func in functions.iter() {
        println!(
            "------------------------------ SSA {} ------------------------------",
            func.addr()
        );
        let mut ssa_func = lir::flat::func_from_flat(func, &blocks);
        ssa_func.resolve();
        lir::analysis::reconstruct_comp::exec(&mut ssa_func);
        lir::analysis::dce::exec(&mut ssa_func);
        lir::analysis::propagate_constant::exec(&mut ssa_func);
        println!("{}", ssa_func.fmt());
    }

    println!(".funcs: # Detected {} functions", functions.len());
    for func in functions {
        println!(
            "------------------------------ func_{} ------------------------------",
            func.addr()
        );

        for block in func.blocks().iter() {
            let block = blocks.get(block).unwrap();
            let code = binary.sections.text.slice(block.addr(), block.len());

            println!("_block_{}:", block.addr());
            println!("{}", block.asm_fmt(code));
        }
    }

    println!(".text:");

    let mut printer = PrinterOfSkipped::new(binary.sections.text);
    for block in blocks.values() {
        printer.print_skipped(block.addr());

        if block.addr() == binary.entry_point {
            println!("_start:");
        } else {
            println!("_block_{}:", block.addr());
        }
        let code = binary.sections.text.slice(block.addr(), block.len());
        println!("{}", block.asm_fmt(code));
        printer.advance(block.addr(), block.len());
    }

    println!(".rdata:");
    let mut printer = PrinterOfSkipped::new(binary.sections.rdata);
    for obj in db.range(binary.sections.rdata.to_range()) {
        printer.print_skipped(obj.addr());
        println!("    {} {:?}", obj.addr(), obj.typ());
        printer.advance(obj.addr(), obj.len());
    }
    printer.print_skipped(binary.sections.rdata.end());

    println!(".data:");
    let mut printer = PrinterOfSkipped::new(binary.sections.data);
    for obj in db.range(binary.sections.data.to_range()) {
        printer.print_skipped(obj.addr());
        println!("    {} {:?}", obj.addr(), obj.typ());
        printer.advance(obj.addr(), obj.len());
    }
    printer.print_skipped(binary.sections.data.end());

    return;
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
                print_asm(
                    &binary,
                    &db,
                    last_addr_end,
                    skipped.try_into().unwrap(),
                    None,
                );
            }
            _ => println!("... OVERLAP"),
        }

        let addr = block.addr();
        let size = block.size();
        last_addr_end = block.addr() + block.size() as u32;

        match block.typ() {
            BlockType::Entry => {
                println!("_start ({size}):");
                print_asm(&binary, &db, addr, size, Some(&mut unique_instructions));
            }
            BlockType::Function => {
                println!("_func_{:x} ({size}):", addr.0);
                print_asm(&binary, &db, addr, size, Some(&mut unique_instructions));
            }
            BlockType::Jump => {
                println!("_jump_{:x} ({size}):", addr.0);
                print_asm(&binary, &db, addr, size, Some(&mut unique_instructions));
            }
            BlockType::Filler => {
                println!("_filler_{:x} ({size}):", addr.0);
                print_asm(&binary, &db, addr, size, Some(&mut unique_instructions));
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

fn print_jump_table(table: &new_cfg::JumpTable) {
    for entry in &table.entries {
        println!("    {} -> {}", entry.addr, entry.target);
    }
    println!();
}

fn print_asm(
    binary: &instr::Binary<'_>,
    db: &ObjDatabase,
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

        print!(
            "    0x{:x} {: <30} | {} ({:?})",
            ins.ip(),
            instrformat,
            sig,
            ins.mnemonic()
        );

        match internal {
            Instruction::Call(Op::Mem(mem)) => {
                if let Some(obj) = mem.to_absolute().map(Addr).and_then(|addr| db.get(addr)) {
                    print!(" | call {}", obj.typ());
                } else {
                    print!(" | {:?}", internal);
                }
            }

            Instruction::Jump(Op::Mem(mem)) | Instruction::JumpConditional(Op::Mem(mem), _) => {
                print!(" | {:?}", internal);
                if let Some(obj) = mem.to_absolute().map(Addr).and_then(|addr| db.get(addr)) {
                    print!(" ({})", obj.typ());
                }
            }

            Instruction::Call(_)
            | Instruction::Return(_)
            | Instruction::Jump(_)
            | Instruction::JumpConditional(_, _)
            | Instruction::Loop(_)
            | Instruction::PushImm(_)
            | Instruction::MovImm(_)
            | Instruction::Int3
            | Instruction::Invalid => {
                print!(" | {:?}", internal);
            }
            Instruction::IcedX86 => {}
        }
        println!();

        if let Some(out) = &mut out {
            out.insert(instruction_signature(&ins));
        }
    }
}

struct PrinterOfSkipped<'a> {
    last_addr: Addr,
    section: SectionData<'a>,
}

impl<'a> PrinterOfSkipped<'a> {
    pub fn with_addr(section: SectionData<'a>, start: Addr) -> Self {
        Self {
            last_addr: start,
            section,
        }
    }

    pub fn new(section: SectionData<'a>) -> Self {
        Self {
            last_addr: section.address,
            section,
        }
    }

    pub fn print_skipped(&self, next_addr: Addr) {
        let skipped = i64::from(next_addr.0) - i64::from(self.last_addr.0);
        if skipped < 0 {
            println!("!!! Overlap: {} bytes", -skipped);
        } else if skipped > 0 {
            let block = self
                .section
                .slice(self.last_addr, usize::try_from(skipped).unwrap());

            let next_line = Addr(self.last_addr.0.next_multiple_of(4));
            let head_size = (next_line.0 - self.last_addr.0) as usize;

            let (head, tail) = if head_size > block.len() {
                (block, &b""[..])
            } else {
                block.split_at(head_size)
            };

            if !head.is_empty() {
                println!("    {} {}", self.last_addr, AsHexdump(head));
            }

            let mut addr_ctr = next_line;
            for word in tail.chunks(8) {
                println!("    {addr_ctr} {}", AsHexdump(word));
                addr_ctr += word.len() as u32;
            }
            println!()
        }
    }

    pub fn advance(&mut self, addr: Addr, size: usize) {
        self.last_addr = addr + u32::try_from(size).unwrap();
    }
}
