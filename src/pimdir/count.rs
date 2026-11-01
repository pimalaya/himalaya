//! # pimdir count
//!
//! The `pimdir count` command, counting the account's messages a query
//! matches through the store's search index.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::pimdir::{
    client::PimdirClient,
    index::{PimdirMailCoverage, parse_query},
};

/// Count the account's messages a query matches.
///
/// Takes the query `pimdir search` takes, and answers from the same
/// index, with what the count could not see.
#[derive(Debug, Parser)]
pub struct PimdirCountCommand {
    /// The query, its words joined by spaces.
    #[arg(value_name = "QUERY", required = true)]
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    pub query: Vec<String>,
}

impl PimdirCountCommand {
    /// Counts and prints the count.
    pub fn execute(self, printer: &mut impl Printer, client: &mut PimdirClient) -> Result<()> {
        let query = parse_query(&self.query, printer.is_json())?;
        let (count, coverage) = client.count_mail(query)?;
        printer.out(PimdirCountOutput { count, coverage })
    }
}

/// The `pimdir count` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirCountOutput {
    /// How many messages the query matches.
    pub count: u64,
    /// What the count could not see.
    pub coverage: PimdirMailCoverage,
}

impl fmt::Display for PimdirCountOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} message(s)", self.count)?;
        write!(f, "{}", self.coverage)
    }
}
