//! # pimdir tag
//!
//! The `pimdir tag` command, adding or removing flags on every message of
//! the account a query matches, through the store's queue.

use std::fmt;

use anyhow::Result;
use clap::{ArgGroup, Parser};
use pimalaya_cli::{
    fields::Fields,
    printer::Printer,
    records::{Record, Records},
    sanitize,
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    pimdir::{
        client::PimdirClient,
        flag::parse_flag,
        index::{PimdirMailCoverage, PimdirTagged, local_clock, parse_query},
    },
};

/// Add or remove flags on every message a query matches.
///
/// Takes the query `pimdir search` takes. Each mailbox holding a matched
/// message gets one flag change, queued for the sync engine to apply on
/// its next run, as `pimdir flag` queues one; every change is checked
/// against what the mailbox's source declares before the first is
/// queued, so a refusal queues nothing. A change is queued only where the
/// one source syncing the mailbox is known and the message's flags were
/// synced; the others are reported and left alone.
///
/// `read`, `flagged`, `answered` and `draft` name `\Seen`, `\Flagged`,
/// `\Answered` and `\Draft`, as `is:` does; any other flag is written as
/// `pimdir flag` takes it.
#[derive(Debug, Parser)]
#[command(group(ArgGroup::new("change").required(true).multiple(true).args(["add", "remove"])))]
pub struct PimdirTagCommand {
    /// Add this flag to every message; repeatable.
    #[arg(long, value_name = "FLAG")]
    pub add: Vec<String>,
    /// Remove this flag from every message; repeatable.
    #[arg(long, value_name = "FLAG")]
    pub remove: Vec<String>,
    /// Print what would be queued, and queue nothing.
    #[arg(long)]
    pub dry_run: bool,
    /// The query, its words joined by spaces.
    #[arg(value_name = "QUERY", required = true)]
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    pub query: Vec<String>,
}

impl PimdirTagCommand {
    /// Plans the change, queues it unless dry, and prints each message.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let add = flags(&self.add)?;
        let remove = flags(&self.remove)?;
        let query = parse_query(&self.query, printer.is_json())?;

        let tag = client.tag_mail(query, &add, &remove, self.dry_run, &local_clock())?;
        printer.out(PimdirTagOutput {
            tagged: tag.tagged,
            coverage: tag.coverage,
        })
    }
}

/// The flags of a tag in the spelling the store takes, each checked as
/// `pimdir flag` checks it, the `is:` names passing through for the index
/// to map.
fn flags(values: &[String]) -> Result<Vec<String>> {
    values
        .iter()
        .map(|value| match value.as_str() {
            "read" | "flagged" | "answered" | "draft" => Ok(value.clone()),
            raw => Ok(parse_flag(raw)?.raw().to_string()),
        })
        .collect()
}

/// The `pimdir tag` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirTagOutput {
    /// One entry per mailbox holding each matched message.
    pub tagged: Vec<PimdirTagged>,
    /// What the query could not see: the tag reaches only what it found.
    pub coverage: PimdirMailCoverage,
}

impl fmt::Display for PimdirTagOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut records = Records::default();

        for tagged in &self.tagged {
            let mut fields = Fields::default();
            fields.push("Mailbox", sanitize::text(&tagged.mailbox));
            fields.push("Flags", sanitize::text(&tagged.flags.join(", ")));

            if let Some(row) = tagged.queue_id {
                fields.push("Row", row);
            }

            records.push(Record {
                title: sanitize::text(&tagged.id).into_owned(),
                state: vec![tagged.status.to_string()],
                fields,
                body: sanitize::text(&tagged.subject).into_owned(),
            });
        }

        self.coverage.write_under(f, &records)
    }
}
