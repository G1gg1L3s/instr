use std::collections::{HashMap, HashSet, VecDeque};

use crate::lir::{
    analysis::def_use,
    block::Blocks,
    func::SsaFunction,
    ins::{InsKind, JumpTarget, TerminatorKind},
    io::IoValues,
    value::{Value, ValueId},
};

fn compute_uses(func: &SsaFunction) -> HashMap<ValueId, usize> {
    let mut uses = HashMap::with_capacity(func.values.len());
    let mut add_use = |v: ValueId| {
        *uses.entry(v).or_insert(0) += 1;
    };

    for block in func.blocks.values() {
        for ins_id in &block.ins {
            let ins = &func.ins[*ins_id];

            ins.visit_arg_values(&mut add_use);
        }
        block.terminator().visit_values(&mut add_use);
    }

    uses
}

pub fn exec(func: &mut SsaFunction) {
    let def_of = def_use::compute(func);
    let mut uses = compute_uses(func);

    for (_, ins) in func.ins.iter_mut() {
        if let InsKind::BinOp { flags, .. } = &mut ins.kind
            && let Some(f) = flags
            && uses.get(f).copied().unwrap_or(0) == 0
        {
            *flags = None;
        }
    }

    let mut worklist = VecDeque::new();
    for (ins_id, ins) in func.ins.iter() {
        if ins.has_side_effects() {
            continue;
        }

        let ins_results = ins.instr_result();
        let all_uses_0 = ins_results
            .into_iter()
            .map(|dst| uses.get(&dst).copied().unwrap_or(0))
            .all(|uses| uses == 0);

        if all_uses_0 {
            worklist.push_back(ins_id);
        }
    }

    let mut removed_ins = HashSet::with_capacity(worklist.len());

    while let Some(ins_id) = worklist.pop_front() {
        let new = removed_ins.insert(ins_id);
        if !new {
            continue;
        }

        let ins = &func.ins[ins_id];
        ins.visit_arg_values(|v| {
            if let Some(c) = uses.get_mut(&v) {
                *c -= 1;

                if *c != 0 {
                    return;
                }

                match def_of.get(&v) {
                    Some(def_use::ValueSource::Ins(def_ins_id)) => {
                        if !func.ins[*def_ins_id].has_side_effects() {
                            worklist.push_back(*def_ins_id);
                        }
                    }
                    Some(x) => {
                        log::warn!("> Value {v} has no uses and its source is {x:?}")
                    }
                    _ => {}
                }
            }
        });
    }

    for ins_id in removed_ins {
        for val in func.ins[ins_id].instr_result() {
            func.values[val] = Value::Invalid;
        }

        func.ins[ins_id].kind = InsKind::Hole;
    }

    for block in func.blocks.values_mut() {
        block
            .ins
            .retain(|&ins_id| !matches!(func.ins[ins_id].kind, InsKind::Hole));
    }
    log::trace!(">> Uses before transitive outputs: {uses:?}");
    remove_transitive_outputs(func, &mut uses);
    log::trace!(">> Uses after transitive outputs: {uses:?}");

    remove_inputs_with_0_uses(func, uses);
}

fn remove_inputs_with_0_uses(func: &mut SsaFunction, uses: HashMap<ValueId, usize>) {
    let mut inputs_to_remove = vec![];
    func.blocks.first_mut().params.retain(|val| {
        let uses = uses.get(&val).copied().unwrap_or(0);
        if uses == 0 {
            inputs_to_remove.push(*val);
            false
        } else {
            true
        }
    });

    func.inputs.retain(|io, v| {
        if inputs_to_remove.contains(v) {
            log::trace!(">> Removing input {io}:{v} from {}", func.addr);
            false
        } else {
            true
        }
    });

    for val in inputs_to_remove {
        func.values[val] = Value::Invalid;
    }
}

fn return_args(blocks: &mut Blocks) -> Vec<&mut IoValues> {
    blocks
        .values_mut()
        .filter_map(|b| match &mut b.terminator_mut().kind {
            TerminatorKind::Ret { adjust: _, args } => Some(args),
            TerminatorKind::Jump(JumpTarget::Tailcall {
                pass_returns: pass, ..
            }) => Some(pass),
            _ => None,
        })
        .collect()
}

fn remove_transitive_outputs(func: &mut SsaFunction, uses: &mut HashMap<ValueId, usize>) {
    let mut return_args = return_args(&mut func.blocks);

    let mut to_delete = vec![];

    for (io, val) in func.inputs.iter() {
        let mut returns = vec![];
        for return_io_values in &return_args {
            if let Some(val) = return_io_values.get(*io) {
                returns.push(val);
            }
        }

        let Some((head, tail)) = returns.split_first() else {
            // Input value is not used in any return, so we don't need to delete it,
            // so we are done
            continue;
        };

        if head == val && tail.iter().all(|x| x == head) {
            to_delete.push((*io, *val));
        }
    }

    for (io, val) in to_delete {
        log::trace!(">> Removing {io}:{val} from function outputs");
        let uses = uses
            .get_mut(&val)
            .expect("entry should exist because terminator has uses");

        for return_io_values in &mut return_args {
            if return_io_values.remove(io).is_some() {
                *uses -= 1;
            }
        }
    }
}
