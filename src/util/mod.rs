//! Utility types and functions

pub mod cache;
pub mod diagnostic;
pub mod span;

/// Spanned value wrapper
#[derive(Debug, Clone, Copy)]
pub struct Spanned<T> {
    /// The value
    pub value: T,
    /// Source span
    pub span: span::Span,
}

impl<T> Spanned<T> {
    /// Create a new spanned value
    #[inline]
    pub fn new(
        value: T,
        span: span::Span,
    ) -> Self {
        Self { value, span }
    }
}

impl<T> std::ops::Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> std::ops::DerefMut for Spanned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
