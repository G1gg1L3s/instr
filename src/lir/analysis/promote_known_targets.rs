use crate::{
    addr::{Addr, MaybeUnknownAddr},
    lir::{
        func::SsaFunction,
        ins::{CallTarget, InsKind, JumpTarget, TerminatorKind},
        value::{Imm, Value, Values},
    },
};

pub fn run(func: &mut SsaFunction) {
    for ins in func.ins.values_mut() {
        let InsKind::Call { target, .. } = &mut ins.kind else {
            continue;
        };

        let CallTarget::Unknown { addr: addr_val } = *target else {
            continue;
        };

        let Value::Imm(Imm::U32(addr)) = func.values[addr_val] else {
            continue;
        };

        let addr = Addr(addr);
        log::trace!(
            ">> [function {}] Promoting call {} at {} to known addr {}",
            func.addr,
            addr_val,
            MaybeUnknownAddr(ins.addr),
            addr
        );
        *target = CallTarget::Known { addr };
    }

    for block in func.blocks.values_mut() {
        let terminator = block.terminator_mut();

        match &mut terminator.kind {
            TerminatorKind::Jump(jump_target) => {
                promote_jump_target(func.addr, terminator.addr, jump_target, &func.values)
            }
            TerminatorKind::Brif {
                cond: _,
                thenb,
                elseb,
            } => {
                promote_jump_target(func.addr, terminator.addr, thenb, &func.values);
                promote_jump_target(func.addr, terminator.addr, elseb, &func.values);
            }
            TerminatorKind::Ret { .. } => {}
        }
    }
}

fn promote_jump_target(
    func_addr: Addr,
    ins_addr: Option<Addr>,
    jump_target: &mut JumpTarget,
    values: &Values,
) {
    let JumpTarget::Unknown {
        addr: addr_val,
        args,
    } = jump_target
    else {
        return;
    };

    let Value::Imm(Imm::U32(addr)) = values[*addr_val] else {
        return;
    };

    let addr = Addr(addr);
    log::trace!(
        ">> [function {}] Promoting jump {} at {} to taill call to {}",
        func_addr,
        addr_val,
        MaybeUnknownAddr(ins_addr),
        addr
    );

    let args = std::mem::take(args);
    *jump_target = JumpTarget::Tailcall { addr, args };
}
