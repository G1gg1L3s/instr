use std::collections::{BTreeMap, HashMap};

use crate::{
    addr::Addr,
    flat_ir::{self, AnnotatedInstr},
    lir::{
        block::BlockId,
        func::SsaFunction,
        ins::{BinOp, Imm, JumpTarget},
        ssa_builder::{SsaBuilder, VarId},
        value::ValueId,
    },
    third_cfg,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FlatVar {
    Reg(flat_ir::Reg),
    Temp(flat_ir::TempId),
}

impl std::fmt::Display for FlatVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlatVar::Reg(reg) => write!(f, "{}", reg),
            FlatVar::Temp(temp_id) => write!(f, "{}", temp_id),
        }
    }
}

#[derive(Debug)]
struct State {
    blocks: HashMap<Addr, BlockId>,
    registers: HashMap<FlatVar, VarId>,
    builder: SsaBuilder,
}

pub fn func_from_flat(
    func: &third_cfg::Function,
    blocks: &BTreeMap<Addr, flat_ir::Block>,
) -> SsaFunction {
    let mut state = State {
        blocks: Default::default(),
        registers: Default::default(),
        builder: SsaBuilder::new(func.addr()),
    };

    for block_addr in func.blocks() {
        let ssa = state.builder.new_block();
        state.blocks.insert(*block_addr, ssa);
    }

    for block_addr in func.blocks() {
        let ir_block = &blocks[block_addr];
        let ssa_block = state.blocks[block_addr];
        state.builder.switch(ssa_block);

        for AnnotatedInstr { addr: _, ins } in ir_block.instr() {
            match ins {
                &flat_ir::Instr::BinOp {
                    op,
                    dst,
                    lhs,
                    rhs,
                    flags,
                } => state.lower_bin(op, dst, lhs, rhs, flags),
                _ => {
                    state.builder.ins().unimplemented();
                }
            }
        }

        state.lower_terminator(ir_block.terminator());
    }

    state.builder.finalise()
}

impl State {
    pub fn get_var(&mut self, var: FlatVar) -> VarId {
        *self.registers.entry(var).or_insert_with(|| {
            let id = self.builder.declare_var();
            log::trace!("> New var: {var} -> {id}");
            id
        })
    }

    pub fn lower_val(&mut self, val: flat_ir::Value) -> ValueId {
        match val {
            flat_ir::Value::Reg(reg) => {
                let var = self.get_var(FlatVar::Reg(reg));
                self.builder.read_var(var)
            }

            flat_ir::Value::Imm(imm) => {
                let imm = imm_to_ssa(imm);
                self.builder.ins().iconst(imm)
            }
            flat_ir::Value::Temp(temp_id) => {
                let var = self.get_var(FlatVar::Temp(temp_id));
                self.builder.read_var(var)
            }
            flat_ir::Value::Flag(_flag) => todo!(),
            flat_ir::Value::X87StatusWord => todo!(),
        }
    }

    pub fn lower_write_val(&mut self, dst: flat_ir::Value, val: ValueId) {
        let var = match dst {
            flat_ir::Value::Reg(reg) => FlatVar::Reg(reg),
            flat_ir::Value::Imm(_imm) => unreachable!(),
            flat_ir::Value::Temp(temp_id) => FlatVar::Temp(temp_id),
            flat_ir::Value::Flag(_flag) => todo!(),
            flat_ir::Value::X87StatusWord => todo!(),
        };

        let var = self.get_var(var);
        self.builder.write_var(var, val);
    }

    pub fn lower_bin(
        &mut self,
        op: flat_ir::BinOp,
        dst: Option<flat_ir::Value>,
        lhs: flat_ir::Value,
        rhs: flat_ir::Value,
        _flags: flat_ir::FlagxGroup,
    ) {
        let lhs = self.lower_val(lhs);
        let rhs = self.lower_val(rhs);
        let op = op_to_ssa(op);

        let res = self.builder.ins().bin(op, lhs, rhs);

        if let Some(dst) = dst {
            self.lower_write_val(dst, res)
        }
    }

    fn lower_terminator(&mut self, terminator: &flat_ir::Terminator) {
        match terminator {
            flat_ir::Terminator::Cond {
                addr: _,
                cond: _,
                then_bb,
                else_bb,
            } => {
                let cond = self.builder.ins().unimplemented();

                let thenb = self.lower_branch_target(then_bb);
                let elseb = self.lower_branch_target(else_bb);

                self.builder.ins().brif(cond, thenb, elseb);
            }
            flat_ir::Terminator::Jump { addr: _, target } => {
                let target = self.lower_branch_target(target);
                self.builder.ins().jump(target);
            }
            flat_ir::Terminator::Ret {
                addr: _,
                stack_adjust,
            } => {
                self.builder.ins().ret(*stack_adjust);
            }
            flat_ir::Terminator::Fallthrough { next } => {
                let block = self.blocks[&next];

                self.builder.ins().jump(JumpTarget::Known {
                    block,
                    args: vec![],
                });
            }
        }
    }

    fn lower_branch_target(&mut self, target: &flat_ir::Value) -> JumpTarget {
        if let Some(addr) = as_u32_addr(target) {
            let block = self.blocks[&addr];
            JumpTarget::Known {
                block,
                args: vec![],
            }
        } else {
            let addr = self.lower_val(*target);
            JumpTarget::Unknown { addr }
        }
    }
}

fn imm_to_ssa(imm: flat_ir::Imm) -> Imm {
    match imm {
        flat_ir::Imm::U8(x) => Imm::U8(x),
        flat_ir::Imm::U16(x) => Imm::U16(x),
        flat_ir::Imm::U32(x) => Imm::U32(x),
    }
}

fn op_to_ssa(op: flat_ir::BinOp) -> BinOp {
    match op {
        flat_ir::BinOp::Add => BinOp::Add,
        flat_ir::BinOp::Sub => BinOp::Sub,
        flat_ir::BinOp::Mulu => BinOp::Mulu,
        flat_ir::BinOp::Muls => BinOp::Muls,
        flat_ir::BinOp::Div => BinOp::Div,
        flat_ir::BinOp::Xor => BinOp::Xor,
        flat_ir::BinOp::BitAnd => BinOp::BitAnd,
        flat_ir::BinOp::BitOr => BinOp::BitOr,
        flat_ir::BinOp::ShiftLeft => BinOp::ShiftLeft,
        flat_ir::BinOp::ShifRight => BinOp::ShifRight,
        flat_ir::BinOp::ShifArithRight => BinOp::ShifArithRight,
    }
}

fn as_u32_addr(value: &flat_ir::Value) -> Option<Addr> {
    if let flat_ir::Value::Imm(flat_ir::Imm::U32(addr)) = *value {
        Some(Addr(addr))
    } else {
        None
    }
}
