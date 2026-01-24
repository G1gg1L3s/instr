use crate::lir::{
    flags::{Flags, FlagsGroup},
    ins::Imm,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ty {
    Flags(Flags),

    Bool,

    U8,
    U16,
    U32,
    U64,

    I8,
    I16,
    I32,
    I64,

    F32,
    F64,

    Mem,
}

impl Ty {
    pub fn imm(self, x: u8) -> Imm {
        match self {
            Ty::U8 => Imm::U8(x.into()),
            Ty::U16 => Imm::U16(x.into()),
            Ty::U32 => Imm::U32(x.into()),
            _ => panic!("cannot convert {self} to immediate"),
        }
    }
}

impl std::fmt::Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ty::Flags(flags) => write!(f, "flags({})", FlagsGroup::new(*flags)),
            Ty::Bool => write!(f, "bool"),
            Ty::U8 => write!(f, "u8"),
            Ty::U16 => write!(f, "u16"),
            Ty::U32 => write!(f, "u32"),
            Ty::U64 => write!(f, "u64"),
            Ty::I8 => write!(f, "i8"),
            Ty::I16 => write!(f, "i16"),
            Ty::I32 => write!(f, "i32"),
            Ty::I64 => write!(f, "i64"),
            Ty::F32 => write!(f, "f32"),
            Ty::F64 => write!(f, "f64"),
            Ty::Mem => write!(f, "mem"),
        }
    }
}
