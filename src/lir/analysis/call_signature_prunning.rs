use std::collections::{BTreeMap, HashMap, HashSet};

use petgraph::{
    prelude::DiGraphMap,
    visit::{DfsPostOrder, Walker},
};

use crate::{
    addr::Addr,
    lir::{
        func::SsaFunction,
        ins::{BinOp, CallTarget, InsKind, JumpTarget, TerminatorKind},
        io::Io,
        value::{Imm, Value},
    },
};

pub fn run(funcs: &mut [SsaFunction], entry: Addr) -> bool {
    log::trace!("> call_signature_prunning pass");
    let mut changed = false;

    let call_graph = build_call_graph(&funcs);
    let mut funcs = BTreeMap::from_iter(funcs.iter_mut().map(|f| (f.addr, f)));

    let mut call_signatures = HashMap::new();

    for func in DfsPostOrder::new(&call_graph, entry).iter(&call_graph) {
        let Some(func) = funcs.get_mut(&func) else {
            continue;
        };
        let func_signature = call_signature(&call_signatures, func);
        call_signatures.insert(func.addr, func_signature);

        log::trace!(">> Prunning {}", func.addr);

        let mut removed_outputs = vec![];
        let mut adjusted_esp = vec![];

        for (ins_id, ins) in func.ins.iter_mut() {
            let InsKind::Call {
                result,
                target: CallTarget::Known { addr },
                args,
            } = &mut ins.kind
            else {
                continue;
            };

            let Some(call_signature) = call_signatures.get(addr) else {
                // TODO: library functions
                continue;
            };

            if let Some(adjust) = call_signature.stack_adjust {
                if !call_signature.outputs.contains(&Io::Esp) {
                    if let Some(input_esp) = args.get(Io::Esp) {
                        // If function outputs esp, we need to adjust it.
                        // Otherwise, it was previously adjusted
                        if let Some(output_esp) = result.get(Io::Esp) {
                            adjusted_esp.push((ins_id, input_esp, output_esp, adjust));
                        };
                    }
                }
            }

            result.retain(|io, output| {
                if call_signature.outputs.contains(io) {
                    true
                } else {
                    let input = args
                        .get(*io)
                        .expect("output value should always have input Io (at least in theory)");
                    removed_outputs.push((input, *output));
                    false
                }
            });

            args.retain(|io, _| call_signature.inputs.contains(io));
        }

        for block in func.blocks.values_mut() {
            match &mut block.terminator_mut().kind {
                TerminatorKind::Jump(JumpTarget::Tailcall { addr, args }) => {
                    if let Some(call_signature) = call_signatures.get(addr) {
                        args.retain(|io, _| call_signature.inputs.contains(io));
                    };
                }

                TerminatorKind::Brif {
                    cond: _,
                    thenb,
                    elseb,
                } => {
                    if let JumpTarget::Tailcall { addr, args } = thenb {
                        if let Some(call_signature) = call_signatures.get(addr) {
                            args.retain(|io, _| call_signature.inputs.contains(io));
                        };
                    }
                    if let JumpTarget::Tailcall { addr, args } = elseb {
                        if let Some(call_signature) = call_signatures.get(addr) {
                            args.retain(|io, _| call_signature.inputs.contains(io));
                        };
                    }
                }

                _ => {}
            }
        }

        log::trace!(">> Going to remove {} outputs", removed_outputs.len());
        for (input_arg, output_arg) in removed_outputs {
            func.values[output_arg] = Value::Alias { to: input_arg };
            changed = true;
        }

        for (ins_id, input_esp, output_esp, adjust) in adjusted_esp {
            let (block_id, idx) = func
                .blocks
                .values_mut()
                .filter_map(|b| {
                    let idx = b.ins.iter().position(|ins| *ins == ins_id)?;
                    Some((b.id, idx))
                })
                .next()
                .unwrap();

            let ins_addr = func.ins[ins_id]
                .addr
                .expect("call instruction always has addr");

            let adjust = func
                .ins_addr_idx(block_id, ins_addr, idx)
                .iconst(Imm::U32(adjust.into()));

            let (updated_esp, _) = func.ins_addr_idx(block_id, ins_addr, idx + 1).bin(
                BinOp::Add,
                input_esp,
                adjust,
                None,
            );

            func.values[output_esp] = Value::Alias { to: updated_esp };
            changed = true;
        }
    }

    changed
}

#[derive(Debug)]
struct CallSignature {
    inputs: HashSet<Io>,
    outputs: HashSet<Io>,
    stack_adjust: Option<u16>,
}

fn call_signature(
    known_call_signatures: &HashMap<Addr, CallSignature>,
    succ: &SsaFunction,
) -> CallSignature {
    let inputs = succ.inputs.keys().collect();
    let mut outputs = HashSet::new();
    let mut stack_adjust = None;

    for block in succ.blocks.values() {
        let mut update_stack_adjust = |new: Option<u16>| {
            if let Some(new) = new {
                let old = stack_adjust.replace(new);
                if let Some(old) = old
                    && old != new
                {
                    panic!("function {} has inconsistent stack adjusts", succ.addr)
                }
            }
        };

        match &block.terminator().kind {
            TerminatorKind::Jump(JumpTarget::Tailcall { addr, .. }) => {
                let Some(call_signature) = known_call_signatures.get(addr) else {
                    // TODO: handle library
                    continue;
                };
                outputs.extend(call_signature.outputs.iter().copied());
                update_stack_adjust(call_signature.stack_adjust);
            }

            TerminatorKind::Brif {
                cond: _,
                thenb,
                elseb,
            } => {
                if let JumpTarget::Tailcall { addr, .. } = thenb {
                    if let Some(call_signature) = known_call_signatures.get(addr) {
                        outputs.extend(call_signature.outputs.iter().copied());
                        update_stack_adjust(call_signature.stack_adjust)
                    }
                }

                if let JumpTarget::Tailcall { addr, .. } = elseb {
                    if let Some(call_signature) = known_call_signatures.get(addr) {
                        outputs.extend(call_signature.outputs.iter().copied());
                        update_stack_adjust(call_signature.stack_adjust)
                    }
                }
            }

            TerminatorKind::Ret { adjust, args } => {
                // TODO: outputs from unknown blocks or jumps can be larger, so we
                // rely on heuristic in some sense.
                outputs.extend(args.keys());
                update_stack_adjust(Some(*adjust))
            }
            _ => continue,
        };
    }

    CallSignature {
        inputs,
        outputs,
        stack_adjust,
    }
}

fn build_call_graph(funcs: &[SsaFunction]) -> DiGraphMap<Addr, ()> {
    let mut res = DiGraphMap::new();
    for func in funcs {
        for ins in func.ins.values() {
            if let InsKind::Call {
                result: _,
                target: CallTarget::Known { addr },
                args: _,
            } = &ins.kind
            {
                res.add_edge(func.addr, *addr, ());
            }
        }

        for block in func.blocks.values() {
            let mut process_jump_target = |jt: &JumpTarget| {
                if let JumpTarget::Tailcall { addr, args: _ } = jt {
                    res.add_edge(func.addr, *addr, ());
                }
            };

            match &block.terminator().kind {
                TerminatorKind::Jump(jt) => {
                    process_jump_target(jt);
                }

                TerminatorKind::Brif {
                    cond: _,
                    thenb,
                    elseb,
                } => {
                    process_jump_target(thenb);
                    process_jump_target(elseb);
                }
                _ => {}
            }
        }
    }
    res
}
