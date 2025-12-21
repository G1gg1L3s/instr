use std::collections::HashSet;

use crate::{
    SectionData,
    addr::Addr,
    flat_ir::{self, Imm, Value},
};

pub fn walk_code_blocks(text: SectionData<'_>, start: Addr) {
    let mut to_visit = vec![start];

    let mut visited = HashSet::new();

    while let Some(addr) = to_visit.pop() {
        let new = visited.insert(addr);
        if !new {
            continue;
        }

        let code = text.slice_to_end(addr);
        let block = flat_ir::lower_block(code, addr);
        eprintln!("{block}");

        match block.terminator.inner {
            flat_ir::Terminator::Cond {
                then_bb, else_bb, ..
            } => {
                if let Value::Imm(Imm::U32(u32)) = then_bb {
                    to_visit.push(Addr(u32));
                }
                if let Value::Imm(Imm::U32(u32)) = else_bb {
                    to_visit.push(Addr(u32));
                }
            }
            flat_ir::Terminator::Jump { target } => {
                if let Value::Imm(Imm::U32(u32)) = target {
                    to_visit.push(Addr(u32));
                }
            }
            flat_ir::Terminator::Ret { .. } => {}
        }
    }
}
