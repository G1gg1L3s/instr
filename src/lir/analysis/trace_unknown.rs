use std::collections::{BTreeMap, HashSet};

use crate::{
    addr::Addr,
    lir::{
        func::SsaFunction,
        ins::{CallTarget, InsKind, JumpTarget, TerminatorKind},
    },
};

pub fn run(funcs: &[SsaFunction], entry: Addr) {
    println!(">> Tracing unknowns");

    let funcs = BTreeMap::from_iter(funcs.iter().map(|f| (f.addr, f)));

    let mut worklist = vec![(0, entry)];
    let mut visited = HashSet::new();

    while let Some((depth, func_addr)) = worklist.pop() {
        let is_new = visited.insert(func_addr);
        if !is_new {
            continue;
        }

        let Some(func) = funcs.get(&func_addr) else {
            continue;
        };
        let indent = Indent(depth);
        println!("{indent}==> [{func_addr}]",);

        for (block_id, block) in func.blocks.iter() {
            for ins_id in &block.ins {
                let ins = &func.ins[*ins_id];
                match &ins.kind {
                    InsKind::Call {
                        target: CallTarget::Unknown { addr },
                        ..
                    } => {
                        println!("{indent}==> [{func_addr}] Unknown call in {block_id}: {addr}");
                    }
                    InsKind::Call {
                        target: CallTarget::Known { addr },
                        ..
                    } => {
                        worklist.push((depth + 1, *addr));
                    }
                    _ => {}
                }
            }

            match &block.terminator().kind {
                TerminatorKind::Jump(JumpTarget::Unknown { addr, args: _ }) => {
                    println!("{indent}==> [{func_addr}] Unknown jump in {block_id}: {addr}");
                }
                TerminatorKind::Jump(JumpTarget::Tailcall { addr, .. }) => {
                    worklist.push((depth + 1, *addr));
                }
                _ => (),
            }
        }
    }
}

struct Indent(usize);

impl std::fmt::Display for Indent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for _ in 0..self.0 {
            write!(f, "=")?;
        }
        Ok(())
    }
}
