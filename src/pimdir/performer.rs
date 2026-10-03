//! # pimdir performer
//!
//! The `pimdir performer` command, showing or choosing which source of
//! the account performs an intent such as sending (pimdir STORAGE §15.6).

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::pimdir::client::PimdirClient;

/// Show or choose the source performing an intent for this account.
///
/// An intent such as sending a message is performed by one source. When
/// several of the account's sources can, Himalaya refuses to pick and
/// this records the choice, applied on the sync engine's next run.
#[derive(Debug, Parser)]
pub struct PimdirPerformerCommand {
    /// The intent, `mail.submit` for sending.
    #[arg(value_name = "CAPABILITY")]
    pub capability: String,
    /// The source to perform it; omitted, the candidates are listed.
    #[arg(value_name = "SOURCE")]
    pub source: Option<String>,
    /// Withdraw the recorded choice.
    #[arg(long, conflicts_with = "source")]
    pub clear: bool,
}

impl PimdirPerformerCommand {
    /// Lists the candidates, or queues the choice.
    pub fn execute(self, printer: &mut impl Printer, client: &mut PimdirClient) -> Result<()> {
        if self.source.is_none() && !self.clear {
            let (candidates, chosen) = client.performers(&self.capability)?;
            return printer.out(PimdirPerformers {
                capability: self.capability,
                candidates,
                chosen,
            });
        }

        client.set_performer(&self.capability, self.source.as_deref())?;
        printer.out(PimdirPerformerChosen {
            capability: self.capability,
            source: self.source,
        })
    }
}

/// The `pimdir performer` output without a source.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirPerformers {
    /// The intent.
    pub capability: String,
    /// The sources able to perform it.
    pub candidates: Vec<String>,
    /// The one the user chose, if any.
    pub chosen: Option<String>,
}

impl fmt::Display for PimdirPerformers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.candidates.is_empty() {
            return write!(f, "No source of this account performs {}", self.capability);
        }
        write!(f, "{}: {}", self.capability, self.candidates.join(", "))?;
        match &self.chosen {
            Some(chosen) => write!(f, " (chosen: {chosen})"),
            None => Ok(()),
        }
    }
}

/// The `pimdir performer` output with a source or `--clear`.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirPerformerChosen {
    /// The intent.
    pub capability: String,
    /// The source chosen, `None` when the choice was withdrawn.
    pub source: Option<String>,
}

impl fmt::Display for PimdirPerformerChosen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.source {
            Some(source) => write!(
                f,
                "{source} now performs {} for this account",
                self.capability
            ),
            None => write!(
                f,
                "The choice of performer for {} is withdrawn",
                self.capability
            ),
        }
    }
}
