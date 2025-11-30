#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Addr(pub u32);

impl Addr {
    pub fn from_u64_assert(addr: u64) -> Self {
        Self(addr.try_into().unwrap())
    }
}

impl std::fmt::Debug for Addr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:x}", self.0)
    }
}

impl std::fmt::Display for Addr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:x}", self.0)
    }
}

impl std::ops::Add<Addr> for Addr {
    type Output = Addr;

    fn add(self, rhs: Self) -> Self::Output {
        Addr(self.0 + rhs.0)
    }
}

impl std::ops::Add<u32> for Addr {
    type Output = Addr;

    fn add(self, rhs: u32) -> Self::Output {
        Addr(self.0 + rhs)
    }
}

impl std::ops::AddAssign<u32> for Addr {
    fn add_assign(&mut self, rhs: u32) {
        self.0 += rhs;
    }
}
