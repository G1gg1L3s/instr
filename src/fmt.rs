pub struct MaybeUnknown<T>(pub Option<T>);

impl<T> std::fmt::Display for MaybeUnknown<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(x) => x.fmt(f),
            None => write!(f, "??"),
        }
    }
}
pub struct AsList<'a, T>(pub &'a [T]);

impl<'a, T: std::fmt::Display> std::fmt::Display for AsList<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        let mut comma = "";
        for a in self.0 {
            write!(f, "{comma}{a}")?;
            comma = ", ";
        }
        write!(f, "]")
    }
}
