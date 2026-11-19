//! # pimdir performer
//!
//! The `pimdir performer` command, showing or choosing which source of
//! the account performs an intent such as sending (pimdir STORAGE §15.6).

use std::fmt;

use anyhow::Result;
use clap::{Parser, builder::PossibleValuesParser};
use io_pimdir::capability;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::pimdir::client::PimdirClient;

/// The intents Himalaya queues, each performed by one source (pimdir
/// STORAGE Annex B.2).
const INTENTS: [&str; 4] = [
    capability::MAIL_SUBMIT,
    capability::COLLECTION_CREATE,
    capability::COLLECTION_RENAME,
    capability::COLLECTION_DELETE,
];

/// Show or choose the source performing an intent for this account.
///
/// An intent such as sending a message is performed by one source. When
/// several of the account's sources can, Himalaya refuses to pick and
/// this records the choice, applied on the sync engine's next run.
#[derive(Debug, Parser)]
pub struct PimdirPerformerCommand {
    /// The intent: `mail.submit` for sending, `collection.create`,
    /// `collection.rename` or `collection.delete` for a mailbox write.
    #[arg(value_name = "CAPABILITY")]
    #[arg(value_parser = PossibleValuesParser::new(INTENTS))]
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
    #[schemars(extend("enum" = INTENTS))]
    pub capability: String,
    /// The sources able to perform it.
    pub candidates: Vec<String>,
    /// The one the user chose, if any.
    pub chosen: Option<String>,
}

impl fmt::Display for PimdirPerformers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.candidates.is_empty() {
            return writeln!(f, "No source of this account performs {}", self.capability);
        }
        write!(f, "{}: {}", self.capability, self.candidates.join(", "))?;
        if let Some(chosen) = &self.chosen {
            write!(f, " (chosen: {chosen})")?;
        }
        writeln!(f)
    }
}

/// The `pimdir performer` output with a source or `--clear`.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirPerformerChosen {
    /// The intent.
    #[schemars(extend("enum" = INTENTS))]
    pub capability: String,
    /// The source chosen, `null` when the choice was withdrawn.
    pub source: Option<String>,
}

impl fmt::Display for PimdirPerformerChosen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.source {
            Some(source) => writeln!(
                f,
                "{source} now performs {} for this account",
                self.capability
            ),
            None => writeln!(
                f,
                "The choice of performer for {} is withdrawn",
                self.capability
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::{Parser, error::ErrorKind};

    use crate::cli::Cli;

    /// An intent Himalaya does not queue is refused by the grammar, which
    /// lists the ones it knows, rather than answering no candidate.
    #[test]
    fn an_unknown_capability_is_refused_naming_the_known_ones() {
        let parse =
            |capability: &str| Cli::try_parse_from(["himalaya", "pimdir", "performer", capability]);

        let err = parse("bogus.cap").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidValue);
        let message = err.to_string();
        for known in super::INTENTS {
            assert!(message.contains(known), "{message}");
        }

        for known in super::INTENTS {
            assert!(parse(known).is_ok(), "{known}");
        }
    }
}
