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
