use std::collections::BTreeMap;

use crate::lir::value::ValueId;

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
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct IoValues {
    vals: BTreeMap<Io, ValueId>,
}

impl IoValues {
    pub fn iter(&self) -> impl Iterator<Item = (&Io, &ValueId)> {
        self.vals.iter()
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
