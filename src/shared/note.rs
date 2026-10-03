//! # Notes
//!
//! What a write comes back with beside its result: a capability one of
//! its sources supports in part (pimdir STORAGE §15.6), whose detail the
//! user should read before relying on the write.

use std::fmt;

use schemars::JsonSchema;
use serde::Serialize;

/// A command's output with the notes its write came back with.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct Noted<T> {
    /// The command's own output.
    #[serde(flatten)]
    pub output: T,
    /// What the user should know about the write, none on most backends.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl<T: fmt::Display> fmt::Display for Noted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let output = self.output.to_string();
        f.write_str(output.trim_end())?;
        for note in &self.notes {
            write!(f, "\nNote: {note}")?;
        }
        if output.ends_with('\n') {
            writeln!(f)?;
        }
        Ok(())
    }
}
