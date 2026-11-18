//! # JMAP set receipt
//!
//! What a `jmap` write acting on several objects prints: the objects the
//! server applied the write to, and the item error of each it refused, a
//! set applying in part (RFC 8620 §5.3). A refused object exits 3, the
//! receipt naming what was done beside what was not.

use std::fmt;

use anyhow::Result;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    error::{ErrorCode, ItemError, partial_when, write_item_errors},
    jmap::error::{JmapSetError, item_error},
};

/// The receipt of a JMAP set acting on several objects.
#[derive(Debug, Serialize, JsonSchema)]
pub struct JmapSetReceipt {
    /// The ids of the objects the server applied the write to, in the
    /// order it answered.
    pub ids: Vec<String>,
    /// The item error of each object the server refused, `[]` when none.
    pub errors: Vec<ItemError>,
    /// What the text form says was done, as in `JMAP emails deleted`.
    #[serde(skip)]
    pub done: &'static str,
}

impl JmapSetReceipt {
    /// The receipt of `ids` done, beside the objects the server refused,
    /// a `notFound` coded `not_found`.
    pub fn new<E: JmapSetError>(
        done: &'static str,
        ids: Vec<String>,
        refused: impl IntoIterator<Item = (String, E)>,
        not_found: Option<ErrorCode>,
    ) -> Self {
        let errors = refused
            .into_iter()
            .map(|(id, err)| item_error(&id, &err, not_found))
            .collect();

        Self { ids, errors, done }
    }

    /// Prints the receipt, then ends the command with the partial-outcome
    /// status when the server refused an object.
    pub fn print(self, printer: &mut impl Printer) -> Result<()> {
        let partial = partial_when(&self.errors);
        printer.out(self)?;
        partial
    }
}

impl fmt::Display for JmapSetReceipt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.ids.is_empty() {
            writeln!(f, "{}: {}", self.done, self.ids.join(", "))?;
        }

        write_item_errors(f, &self.errors)
    }
}
