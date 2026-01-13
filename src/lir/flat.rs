use std::collections::{BTreeMap, HashMap};

use crate::{
    addr::Addr,
    flat_ir::{self, AnnotatedInstr, Flagx},
    lir::{
        block::BlockId,
        flags::{Flags, FlagsGroup},
        func::SsaFunction,
        ins::{BinOp, Imm, JumpTarget, MemSpace},
        ssa_builder::{SsaBuilder, VarId},
        ty::Ty,
        value::ValueId,
    },
    third_cfg,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FlatVar {
    Reg(flat_ir::Reg),
    Temp(flat_ir::TempId),
    Flags,
}

impl FlatVar {
    fn ty(self, block: &flat_ir::Block) -> Ty {
        match self {
            FlatVar::Reg(reg) => size_to_ssa(reg.size()),
            FlatVar::Temp(temp_id) => {
                let temp = block.temp(temp_id);
                size_to_ssa(temp.size)
            }
            FlatVar::Flags => Ty::Flags,
        }
    }
}

fn size_to_ssa(size: flat_ir::Size) -> Ty {
    match size {
        flat_ir::Size::U1 => Ty::Bool,
        flat_ir::Size::U8 => Ty::U8,
        flat_ir::Size::U16 => Ty::U16,
        flat_ir::Size::U32 => Ty::U32,
        flat_ir::Size::U64 => Ty::U64,
        flat_ir::Size::I8 => Ty::I8,
        flat_ir::Size::I16 => Ty::I16,
        flat_ir::Size::I32 => Ty::I32,
        flat_ir::Size::I64 => Ty::I64,
        flat_ir::Size::F32 => Ty::F32,
        flat_ir::Size::F64 => Ty::F64,
    }
}

impl std::fmt::Display for FlatVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlatVar::Reg(reg) => write!(f, "{}", reg),
            FlatVar::Temp(temp_id) => write!(f, "{}", temp_id),
            FlatVar::Flags => write!(f, "flags"),
        }
    }
}

#[derive(Debug)]
struct State {
    blocks: HashMap<Addr, BlockId>,
    registers: HashMap<FlatVar, VarId>,
    builder: SsaBuilder,
}

struct BlockState<'a> {
    flat_block: &'a flat_ir::Block,
    state: &'a mut State,
}

const REGS: [flat_ir::Reg; 8] = [
    flat_ir::Reg::Esp,
    flat_ir::Reg::Eax,
    flat_ir::Reg::Ebx,
    flat_ir::Reg::Ecx,
    flat_ir::Reg::Edx,
    flat_ir::Reg::Esi,
    flat_ir::Reg::Edi,
    flat_ir::Reg::Ebp,
];

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

    {
        let flat_entry = func.blocks().first().unwrap();
        let entry = state.blocks[flat_entry];
        state.builder.switch(entry);

        let mut block_state = BlockState {
            flat_block: &blocks[flat_entry],
            state: &mut state,
        };

        for reg in REGS {
            let var = block_state.get_var(FlatVar::Reg(reg));
            let param = block_state.state.builder.new_param(Ty::U32);
            block_state.state.builder.add_block_param(entry, param);
            block_state.state.builder.write_var(var, param);
        }
    }

    for block_addr in func.blocks() {
        let flat_block = &blocks[block_addr];
        let ssa_block = state.blocks[block_addr];
        state.builder.switch(ssa_block);

        let mut block_state = BlockState {
            flat_block,
            state: &mut state,
        };

        for AnnotatedInstr { addr: _, ins } in flat_block.instr() {
            match ins {
                &flat_ir::Instr::BinOp {
                    op,
                    dst,
                    lhs,
                    rhs,
                    flags,
                } => block_state.lower_bin(op, dst, lhs, rhs, flags),
                &flat_ir::Instr::Assign { dst, src } => block_state.lower_assign(dst, src),
                &flat_ir::Instr::Load { dst, addr, space } => {
                    block_state.lower_load(dst, addr, space)
                }
                _ => {
                    block_state.state.builder.ins().unimplemented();
                }
            }
        }

        block_state.lower_terminator(flat_block.terminator());
    }

    state.builder.finalise()
}

impl<'a> BlockState<'a> {
    pub fn get_var(&mut self, var: FlatVar) -> VarId {
        *self.state.registers.entry(var).or_insert_with(|| {
            let ty = var.ty(self.flat_block);
            let id = self.state.builder.declare_var(ty);
            log::trace!("> New var: {var} -> {id} ({ty})");
            id
        })
    }

    pub fn lower_val(&mut self, val: flat_ir::Value) -> ValueId {
        match val {
            flat_ir::Value::Reg(reg) => {
                let var = self.get_var(FlatVar::Reg(reg));
                self.state.builder.read_var(var)
            }

            flat_ir::Value::Imm(imm) => {
                let imm = imm_to_ssa(imm);
                self.state.builder.ins().iconst(imm)
            }
            flat_ir::Value::Temp(temp_id) => {
                let var = self.get_var(FlatVar::Temp(temp_id));
                self.state.builder.read_var(var)
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
        self.state.builder.write_var(var, val);
    }

    pub fn lower_bin(
        &mut self,
        op: flat_ir::BinOp,
        dst: Option<flat_ir::Value>,
        lhs: flat_ir::Value,
        rhs: flat_ir::Value,
        flags: flat_ir::FlagxGroup,
    ) {
        let lhs = self.lower_val(lhs);
        let rhs = self.lower_val(rhs);
        let op = op_to_ssa(op);

        let flags = if flags.is_empty() {
            None
        } else {
            Some(flags_to_ssa(flags))
        };

        let (res, flags) = self.state.builder.ins().bin(op, lhs, rhs, flags);

        if let Some(dst) = dst {
            self.lower_write_val(dst, res)
        }

        if let Some(flags) = flags {
            let var = self.get_var(FlatVar::Flags);
            self.state.builder.write_var(var, flags);
        }
    }

    fn lower_ret(&mut self, stack_adjust: u16) {
        let args = REGS
            .iter()
            .map(|reg| {
                let var = self.get_var(FlatVar::Reg(*reg));
                self.state.builder.read_var(var)
            })
            .collect();

        self.state.builder.ins().ret(stack_adjust, args);
    }

    fn lower_terminator(&mut self, terminator: &flat_ir::Terminator) {
        match terminator {
            flat_ir::Terminator::Cond {
                addr: _,
                cond: _,
                then_bb,
                else_bb,
            } => {
                let cond = self.state.builder.ins().unimplemented();

                let thenb = self.lower_branch_target(then_bb);
                let elseb = self.lower_branch_target(else_bb);

                self.state.builder.ins().brif(cond, thenb, elseb);
            }
            flat_ir::Terminator::Jump { addr: _, target } => {
                let target = self.lower_branch_target(target);
                self.state.builder.ins().jump(target);
            }
            flat_ir::Terminator::Ret {
                addr: _,
                stack_adjust,
            } => {
                self.lower_ret(*stack_adjust);
            }
            flat_ir::Terminator::Fallthrough { next } => {
                let block = self.state.blocks[&next];

                self.state.builder.ins().jump(JumpTarget::Known {
                    block,
                    args: vec![],
                });
            }
        }
    }

    fn lower_branch_target(&mut self, target: &flat_ir::Value) -> JumpTarget {
        if let Some(addr) = as_u32_addr(target) {
            let block = self.state.blocks[&addr];
            JumpTarget::Known {
                block,
                args: vec![],
            }
        } else {
            let addr = self.lower_val(*target);
            JumpTarget::Unknown { addr }
        }
    }

    fn lower_assign(&mut self, dst: flat_ir::Value, src: flat_ir::Value) {
        let val = self.lower_val(src);
        self.lower_write_val(dst, val);
    }

    fn lower_load(&mut self, dst: flat_ir::Value, addr: flat_ir::Value, space: flat_ir::MemSpace) {
        let space = match space {
            flat_ir::MemSpace::Default => MemSpace::Default,
            flat_ir::MemSpace::Fs => MemSpace::Fs,
        };

        let addr = self.lower_val(addr);
        let ty = self.value_ty(dst);

        let val = self.state.builder.ins().load(ty, addr, space);
        self.lower_write_val(dst, val);
    }

    fn value_ty(&self, value: flat_ir::Value) -> Ty {
        match value {
            flat_ir::Value::Reg(reg) => size_to_ssa(reg.size()),
            flat_ir::Value::Imm(imm) => size_to_ssa(imm.size()),
            flat_ir::Value::Temp(temp_id) => {
                let temp = self.flat_block.temp(temp_id);
                size_to_ssa(temp.size)
            }
            flat_ir::Value::Flag(_) => Ty::Bool,
            flat_ir::Value::X87StatusWord => todo!(),
        }
    }
}

fn flags_to_ssa(group: flat_ir::FlagxGroup) -> FlagsGroup {
    let flags = group.flags();

    let mut res = Flags::empty();

    if flags.contains(Flagx::CARRY) {
        res.insert(Flags::CARRY);
    }
    if flags.contains(Flagx::ZERO) {
        res.insert(Flags::ZERO);
    }
    if flags.contains(Flagx::SIGN) {
        res.insert(Flags::SIGN);
    }
    if flags.contains(Flagx::OVERFLOW) {
        res.insert(Flags::OVERFLOW);
    }
    if flags.contains(Flagx::PARITY) {
        res.insert(Flags::PARITY);
    }
    if flags.contains(Flagx::C0) {
        res.insert(Flags::C0);
    }
    if flags.contains(Flagx::C1) {
        res.insert(Flags::C1);
    }
    if flags.contains(Flagx::C2) {
        res.insert(Flags::C2);
    }
    if flags.contains(Flagx::C3) {
        res.insert(Flags::C3);
    }

    FlagsGroup::new(res)
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
