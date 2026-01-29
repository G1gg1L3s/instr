use std::collections::{BTreeSet, HashMap, HashSet};

use crate::{
    addr::Addr,
    lir::{
        func::SsaFunction,
        ins::{CallTarget, InsKind, MemSpace},
        io::Io,
        value::{Imm, Value},
    },
};

pub fn run(funcs: &[SsaFunction]) {
    let mut values = vec![];
    let mut funcs_to_visit = vec![];

    for func in funcs {
        log::trace!("> Collecting allocations on {}", func.addr);

        for ins in func.ins.values() {
            let InsKind::Call {
                result,
                target:
                    CallTarget::Known {
                        addr: Addr(0x6b5540),
                    },
                args: _,
            } = &ins.kind
            else {
                continue;
            };

            if let Some(eax) = result.get(Io::Eax) {
                values.push(eax);
            }
        }

        log::trace!(">> Allocated values: {values:?}");

        for ins in func.ins.values() {
            let InsKind::Call {
                result: _,
                target: CallTarget::Known { addr: func_addr },
                args,
            } = &ins.kind
            else {
                continue;
            };

            args.iter().for_each(|(io, val)| {
                if values.contains(val) {
                    funcs_to_visit.push((*func_addr, *io));
                }
            })
        }

        values.clear();
    }

    let funcs = funcs
        .iter()
        .map(|func| (func.addr, func))
        .collect::<HashMap<_, _>>();

    let mut visited = HashSet::with_capacity(funcs_to_visit.len());
    let mut vtable_candidates = BTreeSet::new();

    println!("> Funcs to visit:");

    for (func, io) in funcs_to_visit {
        println!("  - {func}:{io}");
        let new = visited.insert((func, io));
        if !new {
            continue;
        }

        let func = funcs[&func];
        let Some(io_val) = func.inputs.get(io) else {
            continue;
        };

        for ins in func.ins.values() {
            let InsKind::Store {
                dst_mem: _,
                src_mem: _,
                addr,
                value,
                space: MemSpace::Default,
            } = &ins.kind
            else {
                continue;
            };

            if *addr != io_val {
                continue;
            }

            let Value::Imm(Imm::U32(addr)) = func.val(*value) else {
                continue;
            };

            vtable_candidates.insert(Addr(*addr));
        }
    }

    println!(">> Vtable candidates");
    for candidate in vtable_candidates {
        println!("  - {}", candidate);
    }
}
