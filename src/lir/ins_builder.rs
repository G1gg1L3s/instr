use crate::{
    addr::Addr,
    lir::{
        block::BlockId,
        flags::{Flag, FlagsGroup},
        func::SsaFunction,
        ins::{
            BinOp, CallTarget, Condition, Ins, InsKind, JumpTableEntry, JumpTarget, MemSpace,
            RawSize, Terminator, TerminatorKind, UnOp,
        },
        io::{Io, IoValues},
        ty::Ty,
        value::{Imm, ValueId},
    },
};

use super::value::Value;

pub struct InsBuilder<'a> {
    pub func: &'a mut SsaFunction,
    pub block: BlockId,
    pub addr: Option<Addr>,
}

impl<'a> InsBuilder<'a> {
    pub fn prepend_uninit_read(&mut self, ty: Ty) -> ValueId {
        let dst = self.func.values.add(Value::Temp { ty });
        let ins = self
            .func
            .ins
            .add(Ins::new(self.addr, InsKind::Uninit { dst }));
        self.func.blocks[self.block].ins.insert(0, ins);
        dst
    }

    pub fn emit(&mut self, ins: InsKind) {
        let ins = self.func.ins.add(Ins::new(self.addr, ins));
        self.func.blocks[self.block].ins.push(ins);
    }

    pub fn iconst(&mut self, imm: Imm) -> ValueId {
        self.func.values.add(Value::Imm(imm))
    }

    pub fn bin(
        &mut self,
        op: BinOp,
        lhs: ValueId,
        rhs: ValueId,
        flags: Option<FlagsGroup>,
    ) -> (ValueId, Option<ValueId>) {
        let dst = self.with_ty(lhs);

        let flags = flags.map(|f| {
            self.func.values.add(Value::Temp {
                ty: Ty::Flags(f.flags()),
            })
        });

        self.emit(InsKind::BinOp {
            op,
            dst,
            lhs,
            rhs,
            flags,
        });

        (dst, flags)
    }

    pub fn un(&mut self, op: UnOp, src: ValueId) -> ValueId {
        let dst = self.with_ty(src);
        self.emit(InsKind::UnOp { op, dst, src });
        dst
    }

    fn with_ty(&mut self, lhs: ValueId) -> ValueId {
        if let Some(ty) = self.func.val_ty(lhs) {
            self.func.values.add(Value::Temp { ty })
        } else {
            self.func.values.add(Value::Invalid)
        }
    }

    pub fn unimplemented(&mut self) -> ValueId {
        let dst = self.func.values.add(Value::Todo);
        self.emit(InsKind::Unimpl { dst });
        dst
    }

    pub fn terminator(&mut self, term: TerminatorKind) {
        match &term {
            TerminatorKind::Jump(JumpTarget::Known { block, args: _ }) => {
                self.func.blocks[*block].predecessors.push(self.block);
            }
            TerminatorKind::Jump(JumpTarget::Unknown { .. }) => {}
            TerminatorKind::Jump(JumpTarget::Tailcall { .. }) => {}
            TerminatorKind::Brif {
                cond: _,
                thenb,
                elseb,
            } => {
                if let JumpTarget::Known { block, args: _ } = thenb {
                    self.func.blocks[*block].predecessors.push(self.block);
                }
                if let JumpTarget::Known { block, args: _ } = elseb {
                    self.func.blocks[*block].predecessors.push(self.block);
                }
            }
            TerminatorKind::Ret { .. } => {}
            TerminatorKind::JumpTable {
                jump_addr: _,
                entries,
            } => {
                for entry in entries {
                    self.func.blocks[entry.target].predecessors.push(self.block);
                }
            }
        }

        self.func.blocks[self.block].terminator = Some(Terminator {
            kind: term,
            addr: self.addr,
        });
    }

    pub fn jump(&mut self, target: JumpTarget) {
        self.terminator(TerminatorKind::Jump(target));
    }
    pub fn brif(&mut self, cond: ValueId, thenb: JumpTarget, elseb: JumpTarget) {
        self.terminator(TerminatorKind::Brif { cond, thenb, elseb });
    }

    pub fn ret(&mut self, adjust: u16, args: IoValues) {
        self.terminator(TerminatorKind::Ret { adjust, args });
    }

    pub fn jump_table(&mut self, jump_addr: ValueId, entries: Vec<JumpTableEntry>) {
        self.terminator(TerminatorKind::JumpTable { jump_addr, entries });
    }

    pub fn call(&mut self, target: CallTarget, args: IoValues, return_types: &[Io]) -> IoValues {
        let result = return_types
            .iter()
            .map(|io| (*io, self.func.values.add(Value::Temp { ty: io.ty() })))
            .collect::<IoValues>();

        self.emit(InsKind::Call {
            result: result.clone(),
            target,
            args,
        });
        result
    }

    pub fn load(&mut self, ty: Ty, addr: ValueId, mem: ValueId, space: MemSpace) -> ValueId {
        let dst = self.func.values.add(Value::Temp { ty });
        self.emit(InsKind::Load {
            dst,
            addr,
            mem,
            space,
        });
        dst
    }

    pub fn store(
        &mut self,
        addr: ValueId,
        value: ValueId,
        mem: ValueId,
        space: MemSpace,
    ) -> ValueId {
        let dst_mem = self.func.values.add(Value::Temp { ty: Ty::Mem });
        self.emit(InsKind::Store {
            dst_mem,
            src_mem: mem,
            addr,
            value,
            space,
        });
        dst_mem
    }

    pub fn cond(&mut self, cond: Condition, flags: ValueId) -> ValueId {
        let dst = self.func.values.add(Value::Temp { ty: Ty::Bool });
        self.emit(InsKind::Cond { dst, flags, cond });
        dst
    }

    pub fn extract(&mut self, src: ValueId, offset: u8, ty: Ty) -> ValueId {
        let dst = self.func.values.add(Value::Temp { ty });
        self.emit(InsKind::Extract { dst, src, offset });
        dst
    }

    pub fn insert(&mut self, base: ValueId, value: ValueId, offset: u8) -> ValueId {
        let dst = self.with_ty(base);
        self.emit(InsKind::Insert {
            dst,
            base,
            value,
            offset,
        });
        dst
    }

    pub fn cast(&mut self, src: ValueId, ty: Ty) -> ValueId {
        let dst = self.func.values.add(Value::Temp { ty });
        self.emit(InsKind::Cast { dst, src });
        dst
    }

    pub fn extract_flag(&mut self, src: ValueId, flag: Flag) -> ValueId {
        let dst = self.func.values.add(Value::Temp { ty: Ty::Bool });
        self.emit(InsKind::ExtractFlag { dst, src, flag });
        dst
    }

    pub fn memset(
        &mut self,
        src_mem: ValueId,
        addr: ValueId,
        value: ValueId,
        count: ValueId,
    ) -> ValueId {
        let dst = self.func.values.add(Value::Temp { ty: Ty::Mem });
        self.emit(InsKind::Memset {
            dst_mem: dst,
            src_mem,
            addr,
            value,
            count,
        });
        dst
    }

    pub fn memcpy(
        &mut self,

        src_mem: ValueId,

        dst_addr: ValueId,
        src_addr: ValueId,
        count: ValueId,
        size: RawSize,
    ) -> ValueId {
        let dst = self.func.values.add(Value::Temp { ty: Ty::Mem });
        self.emit(InsKind::Memcpy {
            dst_mem: dst,
            src_mem,
            dst_addr,
            src_addr,
            count,
            size,
        });
        dst
    }

    pub fn x87push(
        &mut self,
        stack: ValueId,
        val: ValueId,
        group: Option<FlagsGroup>,
    ) -> (ValueId, Option<ValueId>) {
        let dst_stack = self.func.values.add(Value::Temp { ty: Ty::X87Stack });

        let dst_flags = group.map(|f| {
            self.func.values.add(Value::Temp {
                ty: Ty::Flags(f.flags()),
            })
        });

        self.emit(InsKind::X87Push {
            dst_stack,
            dst_flags,
            src_stack: stack,
            value: val,
        });

        (dst_stack, dst_flags)
    }

    pub fn x87pop(
        &mut self,
        stack: ValueId,
        group: Option<FlagsGroup>,
        discard: Discard,
    ) -> (ValueId, Option<ValueId>, Option<ValueId>) {
        let dst_stack = self.func.values.add(Value::Temp { ty: Ty::X87Stack });

        let dst_flags = group.map(|f| {
            self.func.values.add(Value::Temp {
                ty: Ty::Flags(f.flags()),
            })
        });
        let dst = if let Discard::Yes = discard {
            None
        } else {
            Some(self.func.values.add(Value::Temp { ty: Ty::F64 }))
        };

        self.emit(InsKind::X87Pop {
            dst_stack,
            dst_flags,
            src_stack: stack,
            dst,
        });

        (dst_stack, dst_flags, dst)
    }

    pub fn x87peek(&mut self, stack: ValueId, idx: u8) -> ValueId {
        let dst = self.func.values.add(Value::Temp { ty: Ty::F64 });
        self.emit(InsKind::X87Peek { dst, stack, idx });
        dst
    }

    pub fn x87status_word(&mut self, flags: ValueId) -> ValueId {
        let dst = self.func.values.add(Value::Temp { ty: Ty::U16 });
        self.emit(InsKind::X87StatusWord { dst, flags });
        dst
    }
}

pub enum Discard {
    Yes,
    No,
}
