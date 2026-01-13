bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Flags: u16 {
        /// Carry flag
        const CARRY =    1 << 0;
        /// Zero flag
        const ZERO =     1 << 1;
        /// Sign flag
        const SIGN =     1 << 2;
        /// Overflow flag
        const OVERFLOW = 1 << 3;
        /// Parity flag
        const PARITY =   1 << 4;

        // X87 C0
        const C0     =   1 << 5;
        // X87 C1
        const C1     =   1 << 6;
        // X87 C2
        const C2     =   1 << 7;
        // X87 C3
        const C3     =   1 << 8;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlagsGroup(Flags);

impl std::fmt::Display for FlagsGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            x if x == Self::ALL => write!(f, "f:*"),
            x if x == Self::NOCARRY => write!(f, "f:!c"),
            x if x == Self::CARRY_OVERFOW => write!(f, "f:co"),
            x if x == Self::X87_C1 => write!(f, "f:c1"),
            x if x == Self::X87_COM => write!(f, "f:x87com"),
            _ => {
                write!(f, "f:")?;
                for flag in self.0 {
                    write!(f, "{:?}", flag)?;
                }
                Ok(())
            }
        }
    }
}

impl FlagsGroup {
    pub const NONE: Self = Self(Flags::empty());

    pub const ALL: Self = Self(
        Flags::CARRY
            .union(Flags::ZERO)
            .union(Flags::SIGN)
            .union(Flags::OVERFLOW)
            .union(Flags::PARITY),
    );

    pub const NOCARRY: Self = Self(Self::ALL.0.difference(Flags::CARRY));
    pub const CARRY_OVERFOW: Self = Self(Flags::CARRY.union(Flags::OVERFLOW));

    pub const X87_C1: Self = Self(Flags::C1);
    pub const X87_COM: Self = Self(Flags::C0.union(Flags::C1).union(Flags::C2).union(Flags::C3));

    pub fn new(flags: Flags) -> Self {
        Self(flags)
    }

    pub fn is_empty(self) -> bool {
        self.0.is_empty()
    }

    pub fn flags(self) -> Flags {
        self.0
    }
}
