use crate::lir::{
    block::BlockId,
    flags::FlagsGroup,
    func::SsaFunction,
    ins::{BinOp, CallTarget, Condition, Imm, Ins, JumpTarget, MemSpace, Terminator},
    io::{Io, IoValues},
    ty::Ty,
    value::ValueId,
};

use super::value::Value;

pub struct InsBuilder<'a> {
    pub func: &'a mut SsaFunction,
    pub block: BlockId,
}

impl<'a> InsBuilder<'a> {
    pub fn prepend_uninit_read(&mut self) -> ValueId {
        let dst = self.func.values.add(Value::Invalid);
        let ins = self.func.ins.add(Ins::Uninit { dst });
        self.func.blocks[self.block].ins.insert(0, ins);
        dst
    }

    pub fn emit(&mut self, ins: Ins) {
        let ins = self.func.ins.add(ins);
        self.func.blocks[self.block].ins.push(ins);
    }

    pub fn iconst(&mut self, imm: Imm) -> ValueId {
        let dst = self.func.values.add(Value::Temp { ty: imm.ty() });
        self.emit(Ins::Const { dst, val: imm });
        dst
    }

    pub fn bin(
        &mut self,
        op: BinOp,
        lhs: ValueId,
        rhs: ValueId,
        flags: Option<FlagsGroup>,
    ) -> (ValueId, Option<ValueId>) {
        let dst = if let Some(ty) = self.func.val_ty(lhs) {
            self.func.values.add(Value::Temp { ty })
        } else {
            self.func.values.add(Value::Invalid)
        };

        let flags = flags.map(|f| {
            self.func.values.add(Value::Temp {
                ty: Ty::Flags(f.flags()),
            })
        });

        self.emit(Ins::BinOp {
            op,
            dst,
            lhs,
            rhs,
            flags,
        });

        (dst, flags)
    }

    pub fn unimplemented(&mut self) -> ValueId {
        let dst = self.func.values.add(Value::Invalid);
        self.emit(Ins::Unimpl { dst });
        dst
    }

    pub fn terminator(&mut self, term: Terminator) {
        match &term {
            Terminator::Jump(JumpTarget::Known { block, args: _ }) => {
                self.func.blocks[*block].predecessors.push(self.block);
            }
            Terminator::Jump(JumpTarget::Unknown { .. }) => {}
            Terminator::Brif {
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
            Terminator::Ret { .. } => {}
        }

        self.func.blocks[self.block].terminator = Some(term);
    }

    pub fn jump(&mut self, target: JumpTarget) {
        let terminator = Terminator::Jump(target);
        self.terminator(terminator);
    }
    pub fn brif(&mut self, cond: ValueId, thenb: JumpTarget, elseb: JumpTarget) {
        let terminator = Terminator::Brif { cond, thenb, elseb };
        self.terminator(terminator);
    }

    pub fn ret(&mut self, adjust: u16, args: IoValues) {
        self.terminator(Terminator::Ret { adjust, args });
    }

    pub fn call(&mut self, target: CallTarget, args: IoValues, return_types: &[Io]) -> IoValues {
        let result = return_types
            .iter()
            .map(|io| (*io, self.func.values.add(Value::Temp { ty: io.ty() })))
            .collect::<IoValues>();

        self.emit(Ins::Call {
            result: result.clone(),
            target,
            args,
        });
        result
    }

    pub fn load(&mut self, ty: Ty, addr: ValueId, mem: ValueId, space: MemSpace) -> ValueId {
        let dst = self.func.values.add(Value::Temp { ty });
        self.emit(Ins::Load {
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
        let dst_mem = self.func.values.add(Value::Mem);
        self.emit(Ins::Store {
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
        self.emit(Ins::Cond { dst, flags, cond });
        dst
    }
}
