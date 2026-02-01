use std::collections::BTreeMap;

use crate::lir::{ty::Ty, value::ValueId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Io {
    Mem,
    Esp,

    Eax,
    Ebx,
    Ecx,
    Edx,
    Esi,
    Edi,
    Ebp,
    Eip,

    X87Stack,
}
impl Io {
    pub fn ty(&self) -> Ty {
        match self {
            Io::Mem => Ty::Mem,
            Io::Esp => Ty::U32,
            Io::Eax => Ty::U32,
            Io::Ebx => Ty::U32,
            Io::Ecx => Ty::U32,
            Io::Edx => Ty::U32,
            Io::Esi => Ty::U32,
            Io::Edi => Ty::U32,
            Io::Ebp => Ty::U32,
            Io::Eip => Ty::U32,
            Io::X87Stack => Ty::X87Stack,
        }
    }
}

impl std::fmt::Display for Io {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Io::Mem => write!(f, "mem"),
            Io::Eax => write!(f, "eax"),
            Io::Ebx => write!(f, "ebx"),
            Io::Ecx => write!(f, "ecx"),
            Io::Edx => write!(f, "edx"),
            Io::Esi => write!(f, "esi"),
            Io::Edi => write!(f, "edi"),
            Io::Ebp => write!(f, "ebp"),
            Io::Esp => write!(f, "esp"),
            Io::Eip => write!(f, "eip"),
            Io::X87Stack => write!(f, "x87stack"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct IoValues {
    vals: BTreeMap<Io, ValueId>,
}

impl IoValues {
    pub fn get(&self, io: Io) -> Option<ValueId> {
        self.vals.get(&io).copied()
    }

    pub fn keys(&self) -> impl Iterator<Item = Io> {
        self.vals.keys().copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Io, &ValueId)> {
        self.vals.iter()
    }

    pub fn values(&self) -> impl Iterator<Item = ValueId> {
        self.vals.values().copied()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut ValueId> {
        self.vals.values_mut()
    }

    pub fn insert(&mut self, io: Io, val: ValueId) {
        self.vals.insert(io, val);
    }

    pub fn len(&self) -> usize {
        self.vals.len()
    }

    pub fn retain(&mut self, f: impl FnMut(&Io, &mut ValueId) -> bool) {
        self.vals.retain(f);
    }
}

impl std::fmt::Display for IoValues {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut comma = false;
        for (io, val) in &self.vals {
            if comma {
                write!(f, ", ")?;
            }
            write!(f, "{io}: {val}")?;
            comma = true;
        }
        Ok(())
    }
}

impl FromIterator<(Io, ValueId)> for IoValues {
    fn from_iter<T: IntoIterator<Item = (Io, ValueId)>>(iter: T) -> Self {
        Self {
            vals: FromIterator::from_iter(iter),
        }
    }
}
