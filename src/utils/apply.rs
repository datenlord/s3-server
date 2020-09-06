//! `Apply` trait

/// Extends all sized types with an `apply` method
pub trait Apply: Sized {
    /// apply `f` to self
    #[inline]
    fn apply<R>(self, f: impl FnOnce(Self) -> R) -> R {
        f(self)
    }
}

impl<T: Sized> Apply for T {}
