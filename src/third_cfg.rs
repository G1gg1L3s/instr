use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::{
    SectionData,
    addr::Addr,
    flat_ir::{self, Block, Imm, Value},
};

pub fn walk_code_blocks(text: SectionData<'_>, start: Addr) -> BTreeMap<Addr, Block> {
    let mut to_visit = vec![start];
    let mut block_starts = BTreeSet::from([start]);

    let mut blocks = BTreeMap::<Addr, Block>::new();

    while let Some(addr) = to_visit.pop() {
        if blocks.contains_key(&addr) {
            continue;
        }
        println!(">> Processing {addr}");

        if let Some((&overlapping, block)) = blocks.range(..=addr).next_back() {
            if block.contains(addr) {
                println!("    >> Addr {addr} overlaps block {overlapping}, removign block...");
                blocks.remove(&overlapping);
                to_visit.push(overlapping);
            }
        }

        let code = if let Some(next) = block_starts.range(addr..).skip(1).next() {
            let size = next.0 - addr.0;
            text.slice(addr, size as _)
        } else {
            text.slice_to_end(addr)
        };

        println!("    >> Size of {addr}: {}", code.len());

        let block = flat_ir::lower_block(code, addr);

        match block.terminator.inner {
            flat_ir::Terminator::Cond {
                then_bb, else_bb, ..
            } => {
                if let Value::Imm(Imm::U32(u32)) = then_bb {
                    to_visit.push(Addr(u32));
                    block_starts.insert(Addr(u32));
                }
                if let Value::Imm(Imm::U32(u32)) = else_bb {
                    to_visit.push(Addr(u32));
                    block_starts.insert(Addr(u32));
                }
            }
            flat_ir::Terminator::Jump { target } => {
                if let Value::Imm(Imm::U32(u32)) = target {
                    to_visit.push(Addr(u32));
                    block_starts.insert(Addr(u32));
                }
            }
            flat_ir::Terminator::Ret { .. } => {}
            flat_ir::Terminator::Fallthrough { next } => {
                to_visit.push(next);
                block_starts.insert(next);
            }
        }

        for ins in &block.instr {
            match &ins.ins {
                flat_ir::Instr::Call {
                    target: Value::Imm(Imm::U32(addr)),
                } => {
                    to_visit.push(Addr(*addr));
                    block_starts.insert(Addr(*addr));
                }
                _ => {}
            }
        }

        blocks.insert(block.addr, block);
    }

    blocks
}
