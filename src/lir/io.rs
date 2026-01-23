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
