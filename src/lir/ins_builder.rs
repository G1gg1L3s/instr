use crate::lir::{
    block::BlockId,
    func::SsaFunction,
    ins::{BinOp, Imm, Ins, JumpTarget, Terminator},
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

    pub fn bin(&mut self, op: BinOp, lhs: ValueId, rhs: ValueId) -> ValueId {
        let dst = if let Some(ty) = self.func.val_ty(lhs) {
            self.func.values.add(Value::Temp { ty })
        } else {
            self.func.values.add(Value::Invalid)
        };
        self.emit(Ins::BinOp { op, dst, lhs, rhs });
        dst
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

    pub fn ret(&mut self, adjust: u16, args: Vec<ValueId>) {
        self.terminator(Terminator::Ret { adjust, args });
    }
}
