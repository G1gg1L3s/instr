use std::collections::{HashMap, HashSet};

use crate::{
    SectionData,
    addr::Addr,
    lir::{
        analysis::def_use::{self, ValueSource},
        block::Block,
        func::SsaFunction,
        ins::{BinOp, Condition, InsKind, JumpTarget, MemSpace, TerminatorKind},
        value::{Imm, Value, ValueId},
    },
    third_cfg,
};

pub fn run(func: &SsaFunction) -> Vec<JumpTableCandidate> {
    let def_use = def_use::compute(func);
    let mut res = vec![];

    for block in func.blocks.values() {
        let Some((ins_addr, base_addr, index)) = detect_jump_table(func, &def_use, block) else {
            continue;
        };
        let size = derive_size(func, index, block);
        res.push(JumpTableCandidate {
            ins_addr,
            base_addr,
            idx: index,
            size,
        });
    }
    res
}

fn get_ins<'a>(
    func: &'a SsaFunction,
    def_use: &HashMap<ValueId, ValueSource>,
    val: ValueId,
) -> Option<&'a InsKind> {
    let Some(ValueSource::Ins(ins_id)) = def_use.get(&val) else {
        return None;
    };
    Some(&func.ins[*ins_id].kind)
}

pub struct JumpTableCandidate {
    pub ins_addr: Addr,
    pub base_addr: Addr,
    pub idx: ValueId,
    pub size: Option<u32>,
}

fn detect_jump_table(
    func: &SsaFunction,
    def_use: &HashMap<ValueId, ValueSource>,
    block: &Block,
) -> Option<(Addr, Addr, ValueId)> {
    let terminator = block.terminator();
    let TerminatorKind::Jump(JumpTarget::Unknown { addr, args: _ }) = &terminator.kind else {
        return None;
    };

    let InsKind::Load {
        dst: _,
        addr,
        mem: _,
        space: MemSpace::Default,
    } = get_ins(func, def_use, *addr)?
    else {
        return None;
    };

    let InsKind::BinOp {
        op: BinOp::Add,
        dst: _,
        lhs: mul,
        rhs: base_addr,
        flags: _,
    } = get_ins(func, def_use, *addr)?
    else {
        return None;
    };

    let ValueSource::Imm(Imm::U32(imm_addr)) = def_use.get(base_addr)? else {
        return None;
    };

    let InsKind::BinOp {
        op: BinOp::Mulu,
        dst: _,
        lhs: idx,
        rhs: word_size,
        flags: _,
    } = get_ins(func, def_use, *mul)?
    else {
        return None;
    };

    let ValueSource::Imm(Imm::U32(4)) = def_use.get(word_size)? else {
        return None;
    };

    Some((
        terminator.addr.expect("jump should always have addr"),
        Addr(*imm_addr),
        *idx,
    ))
}

fn derive_size(func: &SsaFunction, jump_table_idx: ValueId, block: &Block) -> Option<u32> {
    let mut candidates = vec![];

    for block in &block.predecessors {
        let block = &func.blocks[*block];
        for ins_id in &block.ins {
            let ins = &func.ins[*ins_id];
            let InsKind::BinOp {
                op: BinOp::Condition(Condition::UnsignedLess | Condition::UnsignedGreater),
                dst: _,
                lhs,
                rhs,
                flags: _,
            } = ins.kind
            else {
                continue;
            };

            let (idx, size) = match (lhs, rhs) {
                (idx, size) if idx == jump_table_idx => (idx, size),
                (size, idx) if idx == jump_table_idx => (idx, size),
                _ => continue,
            };

            let Value::Imm(imm) = &func.values[size] else {
                log::warn!(
                    "> Cannot compute jump table size in function {}, index {}",
                    func.addr,
                    idx
                );
                continue;
            };

            match imm {
                Imm::U8(x) => candidates.push(u32::from(*x)),
                Imm::U16(x) => candidates.push(u32::from(*x)),
                Imm::U32(x) => candidates.push(u32::from(*x)),
                _ => continue,
            }
        }
    }

    match candidates.as_slice() {
        [] => None,
        [one] => Some(*one),
        multiple => {
            log::warn!(
                "> Cannot compute jump table size in function {}, index {}: many candidates available: {:?}",
                func.addr,
                jump_table_idx,
                multiple
            );
            None
        }
    }
}

pub fn jump_table_to_cfg(
    table: JumpTableCandidate,
    text: SectionData<'_>,
) -> Option<third_cfg::JumpTable> {
    let size = table.size.unwrap();
    let mut entries = Vec::with_capacity(size as usize);
    for idx in 0..size {
        let target = text.read_u32_le(table.base_addr + idx * 4);
        let target = Addr(target);
        assert!(text.contains(target));
        entries.push(target);
    }

    Some(third_cfg::JumpTable {
        ins_addr: table.ins_addr,
        entries,
        size: Some(size),
    })
}
