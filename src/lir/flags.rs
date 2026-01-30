#[derive(Debug, Clone, Copy)]
pub enum Flag {
    Carry,
    Zero,
    Sign,
    Overflow,
    Parity,
    C0,
    C1,
    C2,
    C3,
}

impl Flag {
    pub fn to_flags(self) -> Flags {
        match self {
            Flag::Carry => Flags::CARRY,
            Flag::Zero => Flags::ZERO,
            Flag::Sign => Flags::SIGN,
            Flag::Overflow => Flags::OVERFLOW,
            Flag::Parity => Flags::PARITY,
            Flag::C0 => Flags::C0,
            Flag::C1 => Flags::C1,
            Flag::C2 => Flags::C2,
            Flag::C3 => Flags::C3,
        }
    }
}

impl std::fmt::Display for Flag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Flag::Carry => write!(f, "CF"),
            Flag::Zero => write!(f, "ZF"),
            Flag::Sign => write!(f, "SF"),
            Flag::Overflow => write!(f, "OF"),
            Flag::Parity => write!(f, "PF"),
            Flag::C0 => write!(f, "C0"),
            Flag::C1 => write!(f, "C1"),
            Flag::C2 => write!(f, "C2"),
            Flag::C3 => write!(f, "C3"),
        }
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
            x if x == Self::ALL_AND_X87_COM => write!(f, "f:***"),
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

    pub const ALL_AND_X87_COM: Self = Self(Self::ALL.0.union(Self::X87_COM.0));

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
