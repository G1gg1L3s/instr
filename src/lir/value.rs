use std::ops::{Index, IndexMut};

use crate::lir::ty::Ty;

#[derive(Debug, Clone)]
pub struct Values(Vec<Value>);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueId(u16);

impl ValueId {
    pub fn id(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Invalid,
    Todo,
    Mem,
    Temp { ty: Ty },
    Alias { to: ValueId },
    Imm(Imm),
}

impl Values {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn add(&mut self, value: Value) -> ValueId {
        let id = self.0.len().try_into().expect("to much values");
        self.0.push(value);
        ValueId(id)
    }

    pub fn keys(&self) -> impl Iterator<Item = ValueId> + use<> {
        let max = self.0.len().try_into().unwrap();
        ValueKeys(0..max)
    }

    pub fn iter(&self) -> impl Iterator<Item = (ValueId, &Value)> {
        self.keys().zip(self.0.iter())
    }

    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.0.iter()
    }

    pub fn val_ty(&self, val: ValueId) -> Option<Ty> {
        let value = &self[val];
        match value {
            Value::Invalid => None,
            Value::Todo => None,
            Value::Temp { ty } => Some(*ty),
            Value::Alias { .. } => self.val_ty(self.resolve_alias(val)),
            Value::Mem => None,
            Value::Imm(x) => Some(x.ty()),
        }
    }

    pub fn resolve_alias(&self, mut val: ValueId) -> ValueId {
        for _ in self.keys() {
            match self[val] {
                Value::Alias { to } => val = to,
                _ => break,
            }
        }

        val
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug, Clone)]
pub struct ValueKeys(std::ops::Range<u16>);

impl Iterator for ValueKeys {
    type Item = ValueId;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(ValueId)
    }
}

impl Index<ValueId> for Values {
    type Output = Value;

    fn index(&self, index: ValueId) -> &Self::Output {
        self.0.index(usize::from(index.0))
    }
}

impl IndexMut<ValueId> for Values {
    fn index_mut(&mut self, index: ValueId) -> &mut Self::Output {
        self.0.index_mut(usize::from(index.0))
    }
}

impl std::fmt::Debug for ValueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl std::fmt::Display for ValueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Imm {
    U8(u8),
    U16(u16),
    U32(u32),
}

impl Imm {
    pub fn ty(self) -> Ty {
        match self {
            Imm::U8(_) => Ty::U8,
            Imm::U16(_) => Ty::U16,
            Imm::U32(_) => Ty::U32,
        }
    }
}

impl std::fmt::Display for Imm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Imm::U8(x) => write!(f, "{x}.u8"),
            Imm::U16(x) => write!(f, "{x}.u16"),
            Imm::U32(x) => write!(f, "{x}.u32"),
        }
    }
}
