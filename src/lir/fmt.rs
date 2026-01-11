pub struct FmtList<'a, T>(pub &'a [T]);

impl<'a, T: std::fmt::Display> std::fmt::Display for FmtList<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() > 0 {
            write!(f, "(")?;
            for (i, arg) in self.0.iter().enumerate() {
                if i != 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{arg}")?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}
