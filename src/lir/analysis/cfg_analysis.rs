use crate::lir::{
    func::SsaFunction,
    ins::{InsKind, JumpTarget, TerminatorKind},
};

pub fn run(funcs: &[SsaFunction]) {
    let mut leaves = vec![];
    for func in funcs {
        if is_leaf(func) {
            leaves.push(func);
        }
    }
    println!(">> Leaf functions:");
    for func in leaves {
        println!("=== LEAF SSA {} ===", func.addr);
        println!("{}", func.fmt())
    }
    println!("<< END of leaf functions");
}

fn is_leaf(func: &SsaFunction) -> bool {
    for block in func.blocks.values() {
        match &block.terminator().kind {
            TerminatorKind::Jump(JumpTarget::Tailcall { .. } | JumpTarget::Unknown { .. }) => {
                return false;
            }
            TerminatorKind::Jump(JumpTarget::Known { .. }) => {}

            TerminatorKind::Brif {
                cond: _,
                thenb: JumpTarget::Tailcall { .. } | JumpTarget::Unknown { .. },
                elseb: _,
            } => return false,
            TerminatorKind::Brif {
                cond: _,
                thenb: _,
                elseb: JumpTarget::Tailcall { .. } | JumpTarget::Unknown { .. },
            } => return false,
            TerminatorKind::Brif { .. } => {}

            TerminatorKind::Ret { .. } => {}
            TerminatorKind::JumpTable { .. } => {}
        }
    }

    for ins in func.ins.values() {
        if let InsKind::Call { .. } = &ins.kind {
            return false;
        }
    }

    true
}
