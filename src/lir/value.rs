use std::ops::{Index, IndexMut};

use crate::lir::{flags::FlagsGroup, ty::Ty};

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
    Mem,
    Temp { ty: Ty },
    Flags(FlagsGroup),
    Alias { to: ValueId },
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

    pub fn keys(&self) -> impl Iterator<Item = ValueId> {
        let max = self.0.len().try_into().unwrap();
        ValueKeys(0..max)
    }

    pub fn iter(&self) -> impl Iterator<Item = (ValueId, &Value)> {
        self.keys().zip(self.0.iter())
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
