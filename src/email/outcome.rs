//! # Outcome
//!
//! What a backend did over several items: what it listed or acted on,
//! and the error of each item it could not.

use crate::error::ItemError;

/// A result beside the errors of the items left out of it.
#[derive(Clone, Debug, Default)]
pub struct Outcome<T> {
    /// What was listed, or how much was acted on.
    pub value: T,
    /// The error of each item that failed, in the order met.
    pub errors: Vec<ItemError>,
}

impl<T> Outcome<T> {
    /// Orders the errors as `ids` names their items, an error naming none
    /// of them last, so a write lists them in the order given.
    pub fn sorted_by(mut self, ids: &[&str]) -> Self {
        let position = |error: &ItemError| {
            let id = error.id.as_deref();
            ids.iter().position(|given| Some(*given) == id)
        };

        self.errors
            .sort_by_key(|error| position(error).unwrap_or(usize::MAX));
        self
    }

    /// Puts first the `errors` of the items left out before this outcome.
    #[cfg(any(feature = "gmail", feature = "jmap", feature = "msgraph"))]
    pub fn after(mut self, mut errors: Vec<ItemError>) -> Self {
        errors.append(&mut self.errors);
        self.errors = errors;
        self
    }

    /// Maps the value, the errors kept.
    #[cfg(backend)]
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Outcome<U> {
        Outcome {
            value: f(self.value),
            errors: self.errors,
        }
    }
}

impl<T> From<T> for Outcome<T> {
    fn from(value: T) -> Self {
        Self {
            value,
            errors: Vec::new(),
        }
    }
}
