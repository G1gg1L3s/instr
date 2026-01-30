use std::collections::HashMap;

use crate::lir::{func::SsaFunction, ins::InsKind, value::Value};

pub fn run(func: &mut SsaFunction) -> bool {
    let mut changed = false;

    let mut inserts = HashMap::new();

    for ins in func.ins.values_mut() {
        let InsKind::Insert {
            dst,
            base: _,
            value,
            offset,
        } = &ins.kind
        else {
            continue;
        };

        let Some(ty) = func.values.val_ty(*value) else {
            continue;
        };

        inserts.insert((*dst, *offset, ty), func.values.resolve_alias(*value));
    }

    for ins in func.ins.values_mut() {
        let InsKind::Extract { dst, src, offset } = &ins.kind else {
            continue;
        };

        let Some(ty) = func.values.val_ty(*dst) else {
            continue;
        };

        let src = func.values.resolve_alias(*src);
        let Some(replacement) = inserts.get(&(src, *offset, ty)) else {
            continue;
        };

        log::trace!(
            ">> [func {}] Eliminating insert-extract pair: {dst} -> {replacement}",
            func.addr
        );

        func.values[*dst] = Value::Alias { to: *replacement };
        ins.kind = InsKind::Hole;
        changed = true;
    }

    changed
}
