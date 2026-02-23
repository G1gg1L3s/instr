use std::collections::{BTreeMap, HashMap};

use crate::{
    addr::{Addr, MaybeUnknownAddr},
    flat_ir::{self, AnnotatedInstr, Flagx},
    lir::{
        block::BlockId,
        flags::{Flag, Flags, FlagsGroup},
        func::SsaFunction,
        ins::{
            BinOp, BranchTarget, CallTarget, Condition, JumpTableEntry, JumpTarget, MemSpace,
            RawSize,
        },
        ins_builder::Discard,
        io::{Io, IoValues},
        ssa_builder::{SsaBuilder, VarId},
        ty::Ty,
        value::{Imm, ValueId},
    },
    third_cfg,
};

use super::{ins::UnOp, ins_builder::InsBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FlatVar {
    Reg(flat_ir::Reg),
    Temp(flat_ir::TempId),
    Flags,
    Mem,
    X87Stack,
}

impl FlatVar {
    fn ty(self, block: &flat_ir::Block) -> Ty {
        match self {
            FlatVar::Reg(reg) => size_to_ssa(reg.size()),
            FlatVar::Temp(temp_id) => {
                let temp = block.temp(temp_id);
                size_to_ssa(temp.size)
            }
            FlatVar::Flags => panic!("must not be called"),
            FlatVar::Mem => Ty::Mem,
            FlatVar::X87Stack => Ty::X87Stack,
        }
    }

    fn to_io(&self) -> Option<Io> {
        match self {
            FlatVar::Reg(reg) => Some(reg_to_io(*reg)),
            FlatVar::Temp(_) => None,
            FlatVar::Flags => None,
            FlatVar::Mem => Some(Io::Mem),
            FlatVar::X87Stack => Some(Io::X87Stack),
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
            FlatVar::Mem => write!(f, "mem"),
            FlatVar::X87Stack => write!(f, "x87stack"),
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
    addr: Option<Addr>,
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

const DEFAULT_INPUT_OUTPUT: [FlatVar; 10] = [
    FlatVar::Mem,
    FlatVar::Reg(flat_ir::Reg::Esp),
    FlatVar::Reg(flat_ir::Reg::Eax),
    FlatVar::Reg(flat_ir::Reg::Ebx),
    FlatVar::Reg(flat_ir::Reg::Ecx),
    FlatVar::Reg(flat_ir::Reg::Edx),
    FlatVar::Reg(flat_ir::Reg::Esi),
    FlatVar::Reg(flat_ir::Reg::Edi),
    FlatVar::Reg(flat_ir::Reg::Ebp),
    FlatVar::X87Stack,
];

pub fn func_from_flat(
    func: &third_cfg::Function,
    blocks: &BTreeMap<Addr, flat_ir::Block>,
) -> SsaFunction {
    log::trace!("> Lowering function {}", func.addr());
    let mut state = State {
        blocks: Default::default(),
        registers: Default::default(),
        builder: SsaBuilder::new(func.addr()),
    };

    let entry = state.builder.new_block(func.addr());
    for block_addr in func.blocks() {
        let ssa = if *block_addr == func.addr() {
            entry
        } else {
            state.builder.new_block(*block_addr)
        };
        state.blocks.insert(*block_addr, ssa);
    }

    {
        let flat_entry = func.addr();
        state.builder.switch(entry);

        let mut block_state = BlockState {
            flat_block: &blocks[&flat_entry],
            state: &mut state,
            addr: None,
        };

        for var in DEFAULT_INPUT_OUTPUT {
            let param = block_state
                .state
                .builder
                .new_param(var.ty(block_state.flat_block));

            let var_id = block_state.get_var(var);
            block_state
                .state
                .builder
                .add_entry_param(entry, var.to_io().unwrap(), param);
            block_state.state.builder.write_var(var_id, param);
        }
    }

    for block_addr in func.blocks() {
        let flat_block = &blocks[block_addr];
        let ssa_block = state.blocks[block_addr];
        state.builder.switch(ssa_block);

        let mut block_state = BlockState {
            flat_block,
            state: &mut state,
            addr: None,
        };

        for AnnotatedInstr { addr, ins } in flat_block.instr() {
            block_state.addr = Some(*addr);

            match *ins {
                flat_ir::Instr::BinOp {
                    op,
                    dst,
                    lhs,
                    rhs,
                    flags,
                } => block_state.lower_bin(op, dst, lhs, rhs, flags),
                flat_ir::Instr::Assign { dst, src } => block_state.lower_assign(dst, src),
                flat_ir::Instr::Load { dst, addr, space } => {
                    block_state.lower_load(dst, addr, space)
                }
                flat_ir::Instr::Store { addr, src, space } => {
                    block_state.lower_store(addr, src, space)
                }
                flat_ir::Instr::Call { target } => block_state.lower_call(target),
                flat_ir::Instr::SliceBytes { dst, src, start } => {
                    block_state.lower_slice_bytes(dst, src, start)
                }
                flat_ir::Instr::SetBytes {
                    dst,
                    base,
                    value,
                    start,
                } => block_state.lower_set_bytes(dst, base, value, start),
                flat_ir::Instr::Convert { dst, src } => block_state.lower_convert(dst, src),
                flat_ir::Instr::Not { dst, src } => block_state.lower_not(dst, src),
                flat_ir::Instr::Nop => {}
                flat_ir::Instr::Memset { addr, value, count } => {
                    block_state.lower_memset(addr, value, count)
                }
                flat_ir::Instr::Memcpy {
                    dst_addr,
                    src_addr,
                    count,
                    size,
                } => block_state.lower_memcpy(dst_addr, src_addr, count, size),

                flat_ir::Instr::X87Push { src, flags } => block_state.lower_x87push(src, flags),
                flat_ir::Instr::X87Pop { dst, flags } => block_state.lower_x87pop(dst, flags),
                flat_ir::Instr::Unknown(_) => {
                    block_state.ins().unimplemented();
                }
                flat_ir::Instr::UnOp { .. } => unimplemented!(),
            }
        }

        block_state.addr = flat_block.terminator().addr();
        block_state.lower_terminator(flat_block.terminator());
    }

    state.builder.finalise()
}

fn reg_to_io(reg: flat_ir::Reg) -> Io {
    match reg {
        flat_ir::Reg::Eax => Io::Eax,
        flat_ir::Reg::Ebx => Io::Ebx,
        flat_ir::Reg::Ecx => Io::Ecx,
        flat_ir::Reg::Edx => Io::Edx,
        flat_ir::Reg::Esi => Io::Esi,
        flat_ir::Reg::Edi => Io::Edi,
        flat_ir::Reg::Ebp => Io::Ebp,
        flat_ir::Reg::Esp => Io::Esp,
        flat_ir::Reg::Eip => Io::Eip,
        flat_ir::Reg::St(_) => unreachable!(),
    }
}

const FUNC_ARGS: [FlatVar; 6] = [
    FlatVar::Mem,
    FlatVar::Reg(flat_ir::Reg::Esp),
    FlatVar::Reg(flat_ir::Reg::Eax),
    FlatVar::Reg(flat_ir::Reg::Ecx),
    FlatVar::Reg(flat_ir::Reg::Edx),
    FlatVar::X87Stack,
];

impl<'a> BlockState<'a> {
    pub fn get_var(&mut self, var: FlatVar) -> VarId {
        *self.state.registers.entry(var).or_insert_with(|| {
            let ty = var.ty(self.flat_block);
            let id = self.state.builder.declare_var(ty);
            log::trace!("> New var: {var} -> {id} ({ty})");
            id
        })
    }

    pub fn get_flags_var(&mut self, flags: Flags) -> VarId {
        let var = *self
            .state
            .registers
            .entry(FlatVar::Flags)
            .or_insert_with(|| {
                let id = self.state.builder.declare_var(Ty::Flags(flags));
                log::trace!("> New flags var: {id} ({})", FlagsGroup::new(flags));
                id
            });

        self.state.builder.set_var_ty(var, Ty::Flags(flags));
        var
    }

    pub fn lower_val(&mut self, val: flat_ir::Value) -> ValueId {
        match val {
            flat_ir::Value::Reg(flat_ir::Reg::St(idx)) => {
                let stack = self.get_var(FlatVar::X87Stack);
                let stack = self.state.builder.read_var(stack);
                self.ins().x87peek(stack, idx)
            }

            flat_ir::Value::Reg(reg) => {
                let var = self.get_var(FlatVar::Reg(reg));
                self.state.builder.read_var(var)
            }

            flat_ir::Value::Imm(imm) => {
                let imm = imm_to_ssa(imm);
                self.ins().iconst(imm)
            }
            flat_ir::Value::Temp(temp_id) => {
                let var = self.get_var(FlatVar::Temp(temp_id));
                self.state.builder.read_var(var)
            }
            #[allow(unused)]
            flat_ir::Value::Flags(cond) => {
                let ssa_flag = flag_to_ssa(todo!());
                let flags = self.get_flags_var(ssa_flag.to_flags());
                let flags_val = self.state.builder.read_var(flags);
                let ty = self.state.builder.func.val_ty(flags_val);
                let Some(Ty::Flags(flags)) = ty else {
                    log::warn!("invalid value {flags_val}: expected flags, found: {ty:?}");
                    return self.ins().unimplemented();
                };

                if !flags.contains(ssa_flag.to_flags()) {
                    panic!(
                        "required flag: {}, but only provided: {:?}",
                        ssa_flag,
                        FlagsGroup::new(flags)
                    );
                }

                self.ins().extract_flag(flags_val, ssa_flag)
            }
            flat_ir::Value::X87StatusWord => {
                let flags = self.get_flags_var(FlagsGroup::X87_COM.flags());
                let flags_val = self.state.builder.read_var(flags);
                let ty = self.state.builder.func.val_ty(flags_val);
                let Some(Ty::Flags(_)) = ty else {
                    log::warn!("invalid value {flags_val}: expected flags, found: {ty:?}");
                    return self.ins().unimplemented();
                };
                self.ins().x87status_word(flags_val)
            }
            flat_ir::Value::X87ControlWord => unimplemented!(),
        }
    }

    fn ins(&mut self) -> InsBuilder<'_> {
        if let Some(addr) = self.addr {
            self.state.builder.ins_addr(addr)
        } else {
            self.state.builder.ins()
        }
    }

    pub fn lower_write_val(&mut self, dst: flat_ir::Value, val: ValueId) {
        let var = match dst {
            flat_ir::Value::Reg(reg) => FlatVar::Reg(reg),
            flat_ir::Value::Imm(_imm) => unreachable!(),
            flat_ir::Value::Temp(temp_id) => FlatVar::Temp(temp_id),
            flat_ir::Value::Flags(_) => todo!(),
            flat_ir::Value::X87StatusWord => todo!(),
            flat_ir::Value::X87ControlWord => todo!(),
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
        group: flat_ir::FlagxGroup,
    ) {
        let lhs = self.lower_val(lhs);
        let rhs = self.lower_val(rhs);
        let op = op_to_ssa(op);

        let ssa_flags = if group.is_empty() {
            None
        } else {
            Some(FlagsGroup::new(flags_to_ssa(group.flags())))
        };

        let (res, flags) = self.ins().bin(op, lhs, rhs, ssa_flags);

        if let Some(dst) = dst {
            self.lower_write_val(dst, res)
        }

        if let Some(flags) = flags {
            let var = self.get_flags_var(ssa_flags.unwrap().flags());
            self.state.builder.write_var(var, flags);
        }
    }

    fn lower_ret(&mut self, stack_adjust: u16) {
        let args = DEFAULT_INPUT_OUTPUT
            .iter()
            .map(|flatvar| {
                let var = self.get_var(*flatvar);
                let val = self.state.builder.read_var(var);
                (flatvar.to_io().unwrap(), val)
            })
            .collect();

        self.ins().ret(stack_adjust, args);
    }

    fn lower_condition(&mut self, cond: flat_ir::Condition) -> ValueId {
        let required = flags_to_ssa(cond.required_flags());
        let flags = self.get_flags_var(required);
        let flags_val = self.state.builder.read_var(flags);

        let ty = self.state.builder.func.val_ty(flags_val);
        let Some(Ty::Flags(flags)) = ty else {
            log::warn!("invalid value {flags_val}: expected flags, found: {ty:?}");
            // TODO: return Invalid
            return self.ins().unimplemented();
        };

        if !flags.contains(required) {
            // TODO: somehow reference the correct flags
            panic!(
                "[{}] required flags: {:?}, but only provided: {:?}",
                MaybeUnknownAddr(self.addr),
                required,
                FlagsGroup::new(flags)
            );
        }

        let cond = cond_to_ssa(cond);
        self.ins().cond(cond, flags_val)
    }

    fn lower_terminator(&mut self, terminator: &flat_ir::Terminator) {
        match terminator {
            flat_ir::Terminator::Cond {
                addr: _,
                cond,
                then_bb,
                else_bb,
            } => {
                let cond = self.lower_condition(*cond);

                let thenb = self.lower_branch_target(then_bb);
                let elseb = self.lower_branch_target(else_bb);

                self.ins().brif(cond, thenb, elseb);
            }
            flat_ir::Terminator::Jump { addr: _, target } => {
                let target = self.lower_jump_target(target);
                self.ins().jump(target);
            }
            flat_ir::Terminator::Ret {
                addr: _,
                stack_adjust,
            } => {
                self.lower_ret(*stack_adjust);
            }
            flat_ir::Terminator::Fallthrough { next } => {
                let block = self.state.blocks[next];

                self.ins().jump(JumpTarget::Known {
                    block,
                    args: vec![],
                });
            }
            flat_ir::Terminator::JumpTable {
                addr: _,
                jump_addr,
                entries,
            } => {
                let jump_addr = self.lower_val(*jump_addr);

                let mut ssa_entries = vec![];
                for entry in entries {
                    let target = self.state.blocks[entry];
                    ssa_entries.push(JumpTableEntry {
                        target,
                        args: vec![],
                    })
                }
                self.ins().jump_table(jump_addr, ssa_entries);
            }
        }
    }

    fn lower_jump_target(&mut self, target: &flat_ir::Value) -> JumpTarget {
        let flat_vars = std::iter::once(FlatVar::Mem)
            .chain(REGS.iter().copied().map(FlatVar::Reg))
            .collect::<heapless::Vec<_, 32>>();

        if let Some(addr) = as_u32_addr(target) {
            if let Some(block) = self.state.blocks.get(&addr) {
                JumpTarget::Known {
                    block: *block,
                    args: vec![],
                }
            } else {
                let args = self.read_io_values(&FUNC_ARGS);
                JumpTarget::Tailcall {
                    addr,
                    args,
                    pass_returns: IoValues::default(),
                }
            }
        } else {
            let addr = self.lower_val(*target);
            let args = self.read_io_values(&flat_vars);
            JumpTarget::Unknown { addr, args }
        }
    }

    fn lower_branch_target(&mut self, target: &flat_ir::Value) -> BranchTarget {
        let Some(addr) = as_u32_addr(target) else {
            panic!("branch with unknown target: {target}")
        };

        let Some(block) = self.state.blocks.get(&addr) else {
            panic!("branch with taillcall: {addr}")
        };

        BranchTarget {
            block: *block,
            args: vec![],
        }
    }

    fn lower_assign(&mut self, dst: flat_ir::Value, src: flat_ir::Value) {
        let val = self.lower_val(src);
        self.lower_write_val(dst, val);
    }

    fn lower_load(&mut self, dst: flat_ir::Value, addr: flat_ir::Value, space: flat_ir::MemSpace) {
        let space = mem_space_to_ssa(space);

        let addr = self.lower_val(addr);
        let ty = self.value_ty(dst);

        let mem = self.get_var(FlatVar::Mem);
        let mem = self.state.builder.read_var(mem);

        let val = self.ins().load(ty, addr, mem, space);
        self.lower_write_val(dst, val);
    }

    fn lower_store(&mut self, addr: flat_ir::Value, src: flat_ir::Value, space: flat_ir::MemSpace) {
        let space = mem_space_to_ssa(space);
        let addr = self.lower_val(addr);
        let src = self.lower_val(src);

        let mem_var = self.get_var(FlatVar::Mem);
        let mem = self.state.builder.read_var(mem_var);

        let mem = self.ins().store(addr, src, mem, space);
        self.state.builder.write_var(mem_var, mem);
    }

    fn value_ty(&self, value: flat_ir::Value) -> Ty {
        match value {
            flat_ir::Value::Reg(reg) => size_to_ssa(reg.size()),
            flat_ir::Value::Imm(imm) => size_to_ssa(imm.size()),
            flat_ir::Value::Temp(temp_id) => {
                let temp = self.flat_block.temp(temp_id);
                size_to_ssa(temp.size)
            }
            flat_ir::Value::Flags(_) => Ty::Bool,
            flat_ir::Value::X87StatusWord => todo!(),
            flat_ir::Value::X87ControlWord => todo!(),
        }
    }

    fn lower_call(&mut self, target: flat_ir::Value) {
        let target = if let Some(addr) = as_u32_addr(&target) {
            CallTarget::Known { addr }
        } else {
            let addr = self.lower_val(target);
            CallTarget::Unknown { addr }
        };

        let return_types = FUNC_ARGS
            .iter()
            .map(|f| f.to_io().unwrap())
            .collect::<Vec<_>>();

        let args = self.read_io_values(&FUNC_ARGS);

        let res = self.ins().call(target, args, &return_types);

        for (io, res) in res.iter() {
            let flatvar = io_to_flatvar(*io);
            let var = self.get_var(flatvar);
            self.state.builder.write_var(var, *res);
        }
    }

    fn read_io_values(&mut self, flat_vars: &[FlatVar]) -> super::io::IoValues {
        flat_vars
            .iter()
            .map(|flatvar| {
                let var = self.get_var(*flatvar);
                let val = self.state.builder.read_var(var);
                (flatvar.to_io().unwrap(), val)
            })
            .collect::<_>()
    }

    fn lower_slice_bytes(&mut self, dst: flat_ir::Value, src: flat_ir::Value, start: u8) {
        let src = self.lower_val(src);
        let ty = self.value_ty(dst);

        let val = self.ins().extract(src, start, ty);
        self.lower_write_val(dst, val);
    }

    fn lower_set_bytes(
        &mut self,
        dst: flat_ir::Value,
        base: flat_ir::Value,
        value: flat_ir::Value,
        start: u8,
    ) {
        let base = self.lower_val(base);
        let value = self.lower_val(value);

        let val = self.ins().insert(base, value, start);
        self.lower_write_val(dst, val);
    }

    fn lower_convert(&mut self, dst: flat_ir::Value, src: flat_ir::Value) {
        let src = self.lower_val(src);
        let ty = self.value_ty(dst);

        let dst_val = self.ins().cast(src, ty);
        self.lower_write_val(dst, dst_val);
    }

    fn lower_not(&mut self, dst: flat_ir::Value, src: flat_ir::Value) {
        let src = self.lower_val(src);
        let dst_val = self.ins().un(UnOp::BitNot, src);
        self.lower_write_val(dst, dst_val);
    }

    fn lower_memset(&mut self, addr: flat_ir::Value, value: flat_ir::Value, count: flat_ir::Value) {
        let addr = self.lower_val(addr);
        let value = self.lower_val(value);
        let count = self.lower_val(count);

        let mem_var = self.get_var(FlatVar::Mem);
        let mem: ValueId = self.state.builder.read_var(mem_var);

        let mem = self.ins().memset(mem, addr, value, count);
        self.state.builder.write_var(mem_var, mem);
    }

    fn lower_memcpy(
        &mut self,
        dst_addr: flat_ir::Value,
        src_addr: flat_ir::Value,
        count: flat_ir::Value,
        size: flat_ir::Size,
    ) {
        let dst_addr = self.lower_val(dst_addr);
        let src_addr = self.lower_val(src_addr);
        let count = self.lower_val(count);

        let size = match size {
            flat_ir::Size::U8 => RawSize::U8,
            flat_ir::Size::U16 => RawSize::U16,
            flat_ir::Size::U32 => RawSize::U32,
            _ => unreachable!(),
        };

        let mem_var = self.get_var(FlatVar::Mem);
        let mem: ValueId = self.state.builder.read_var(mem_var);

        let mem = self.ins().memcpy(mem, dst_addr, src_addr, count, size);
        self.state.builder.write_var(mem_var, mem);
    }

    fn lower_x87push(&mut self, val: flat_ir::Value, group: flat_ir::FlagxGroup) {
        let stack_var = self.get_var(FlatVar::X87Stack);
        let stack = self.state.builder.read_var(stack_var);
        let ssa_flags = if group.is_empty() {
            None
        } else {
            Some(FlagsGroup::new(flags_to_ssa(group.flags())))
        };

        let val = self.lower_val(val);
        let (res_stack, res_flags) = self.ins().x87push(stack, val, ssa_flags);

        self.state.builder.write_var(stack_var, res_stack);
        if let Some(flags) = res_flags {
            let var = self.get_flags_var(ssa_flags.unwrap().flags());
            self.state.builder.write_var(var, flags);
        }
    }

    fn lower_x87pop(&mut self, dst: Option<flat_ir::Value>, group: flat_ir::FlagxGroup) {
        let stack_var = self.get_var(FlatVar::X87Stack);
        let stack = self.state.builder.read_var(stack_var);
        let ssa_flags = if group.is_empty() {
            None
        } else {
            Some(FlagsGroup::new(flags_to_ssa(group.flags())))
        };

        let discard = if dst.is_none() {
            Discard::Yes
        } else {
            Discard::No
        };
        let (res_stack, res_flags, val) = self.ins().x87pop(stack, ssa_flags, discard);

        self.state.builder.write_var(stack_var, res_stack);
        if let Some(flags) = res_flags {
            let var = self.get_flags_var(ssa_flags.unwrap().flags());
            self.state.builder.write_var(var, flags);
        }

        if let Some((dst, val)) = dst.zip(val) {
            self.lower_write_val(dst, val);
        }
    }
}

fn flag_to_ssa(flag: flat_ir::Flag) -> Flag {
    match flag {
        flat_ir::Flag::Cf => Flag::Carry,
        flat_ir::Flag::Zf => Flag::Zero,
        flat_ir::Flag::Sf => Flag::Sign,
        flat_ir::Flag::Of => Flag::Overflow,
    }
}

fn io_to_flatvar(io: Io) -> FlatVar {
    match io {
        Io::Mem => FlatVar::Mem,
        Io::Esp => FlatVar::Reg(flat_ir::Reg::Esp),
        Io::Eax => FlatVar::Reg(flat_ir::Reg::Eax),
        Io::Ebx => FlatVar::Reg(flat_ir::Reg::Ebx),
        Io::Ecx => FlatVar::Reg(flat_ir::Reg::Ecx),
        Io::Edx => FlatVar::Reg(flat_ir::Reg::Edx),
        Io::Esi => FlatVar::Reg(flat_ir::Reg::Esi),
        Io::Edi => FlatVar::Reg(flat_ir::Reg::Edi),
        Io::Ebp => FlatVar::Reg(flat_ir::Reg::Ebp),
        Io::Eip => FlatVar::Reg(flat_ir::Reg::Eip),
        Io::X87Stack => FlatVar::X87Stack,
    }
}

fn mem_space_to_ssa(space: flat_ir::MemSpace) -> MemSpace {
    match space {
        flat_ir::MemSpace::Default => MemSpace::Default,
        flat_ir::MemSpace::Fs => MemSpace::Fs,
    }
}

fn cond_to_ssa(cond: flat_ir::Condition) -> Condition {
    match cond {
        flat_ir::Condition::Equal => Condition::Equal,
        flat_ir::Condition::NotEqual => Condition::NotEqual,
        flat_ir::Condition::SignLess => Condition::SignLess,
        flat_ir::Condition::UnsignedLess => Condition::UnsignedLess,
        flat_ir::Condition::SignedLessEqual => Condition::SignedLessEqual,
        flat_ir::Condition::UnsignedLessEqual => Condition::UnsignedLessEqual,
        flat_ir::Condition::SignedGreaterEqual => Condition::SignedGreaterEqual,
        flat_ir::Condition::UnsignedGreaterEqual => Condition::UnsignedGreaterEqual,
        flat_ir::Condition::SignedGreater => Condition::SignedGreater,
        flat_ir::Condition::UnsignedGreater => Condition::UnsignedGreater,
        flat_ir::Condition::Negative => Condition::Negative,
        flat_ir::Condition::Positive => Condition::Positive,
        flat_ir::Condition::Overflow => Condition::Overflow,
        flat_ir::Condition::NoOverflow => Condition::NoOverflow,
        flat_ir::Condition::ParityEven => Condition::ParityEven,
        flat_ir::Condition::ParityOdd => Condition::ParityOdd,
    }
}

fn flags_to_ssa(flags: flat_ir::Flagx) -> Flags {
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

    res
}

fn imm_to_ssa(imm: flat_ir::Imm) -> Imm {
    match imm {
        flat_ir::Imm::U8(x) => Imm::U8(x),
        flat_ir::Imm::U16(x) => Imm::U16(x),
        flat_ir::Imm::U32(x) => Imm::U32(x),
        flat_ir::Imm::F64(_) => unimplemented!(),
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
        flat_ir::BinOp::Atan2 => unimplemented!(),
        flat_ir::BinOp::Fscale => unimplemented!(),
        flat_ir::BinOp::Fyl2x => unimplemented!(),
    }
}

fn as_u32_addr(value: &flat_ir::Value) -> Option<Addr> {
    if let flat_ir::Value::Imm(flat_ir::Imm::U32(addr)) = *value {
        Some(Addr(addr))
    } else {
        None
    }
}
