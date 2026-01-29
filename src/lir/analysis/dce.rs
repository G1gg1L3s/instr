use std::collections::{HashMap, HashSet, VecDeque};

use crate::lir::{analysis::def_use, func::SsaFunction, ins::InsKind, value::ValueId};

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

    let mut removed = HashSet::with_capacity(worklist.len());

    while let Some(ins_id) = worklist.pop_front() {
        let new = removed.insert(ins_id);
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

    for ins_id in removed {
        func.ins[ins_id].kind = InsKind::Hole;
    }

    for block in func.blocks.values_mut() {
        block
            .ins
            .retain(|&ins_id| !matches!(func.ins[ins_id].kind, InsKind::Hole));
    }

    // TODO: need to consider terminators
    // let mut inputs_to_remove = vec![];
    // func.blocks.first_mut().params.retain(|val| {
    //     let uses = uses.get(&val).copied().unwrap_or(0);
    //     if uses == 0 {
    //         inputs_to_remove.push(*val);
    //         false
    //     } else {
    //         true
    //     }
    // });

    // func.inputs.retain(|io, v| {
    //     if inputs_to_remove.contains(v) {
    //         log::trace!(">> Removing input {io}:{v} from {}", func.addr);
    //         false
    //     } else {
    //         true
    //     }
    // });
}
