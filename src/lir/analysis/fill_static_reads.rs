use crate::{
    SectionData,
    addr::Addr,
    lir::{
        func::SsaFunction,
        ins::{InsKind, MemSpace},
        ty::Ty,
        value::{Imm, Value},
    },
    obj::{ImportThunk, ObjDatabase, ObjectTyp},
};

pub fn exec(func: &mut SsaFunction, rdata: SectionData<'_>, obj: &ObjDatabase) {
    log::info!("> Filling static data in {}", func.addr);
    for (_, ins) in func.ins.iter_mut() {
        let &InsKind::Load {
            dst,
            addr,
            mem: _,
            space: MemSpace::Default,
        } = &ins.kind
        else {
            continue;
        };

        let &Value::Imm(Imm::U32(addr)) = &func.values[addr] else {
            continue;
        };
        let addr = Addr(addr);
        let Some(ty) = func.values.val_ty(dst) else {
            log::warn!(">> Cannot fill static reads of {dst} because of unknown type",);
            continue;
        };

        let replacement = match obj.get(addr).map(|o| o.typ()) {
            Some(ObjectTyp::ImportThunk(ImportThunk { vaddr, .. })) => {
                assert_eq!(ty, Ty::U32);
                Imm::U32(vaddr.0)
            }
            _ if rdata.contains(addr) => match ty {
                Ty::U8 => Imm::U8(rdata.read_u8(addr)),
                Ty::U16 => Imm::U16(u16::from_le_bytes(rdata.read_array(addr))),
                Ty::U32 => Imm::U32(u32::from_le_bytes(rdata.read_array(addr))),
                Ty::U64 | Ty::I8 | Ty::I16 | Ty::I32 | Ty::I64 | Ty::F32 | Ty::F64 => {
                    log::warn!(">> Failed to read static data of type: {ty} at {addr}");
                    continue;
                }
                _ => continue,
            },
            _ => continue,
        };

        log::trace!(">> Filling with value {dst} -> {replacement}");
        func.values[dst] = Value::Imm(replacement);

        ins.kind = InsKind::Hole;
    }
}
