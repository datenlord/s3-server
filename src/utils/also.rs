//! `Also` trait

/// Extends all sized types with an `also` method
pub trait Also: Sized {
    /// mutate self by `f` and return self
    #[inline]
    fn also(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }

    /// try to mutate self by `f` and return `Result<Self, E>`
    #[inline]
    fn try_also<E>(mut self, f: impl FnOnce(&mut Self) -> Result<(), E>) -> Result<Self, E> {
        f(&mut self)?;
        Ok(self)
    }
}

impl<T: Sized> Also for T {}
