//! # pimdir tag
//!
//! The `pimdir tag` command, adding or removing flags on every message of
//! the account a query matches, through the store's queue.

use std::fmt;

use anyhow::Result;
use clap::{ArgGroup, Parser};
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    email::flag::Flag,
    listing::Listing,
    pimdir::{
        client::PimdirClient,
        flag::parse_flag,
        index::{PimdirMailCoverage, PimdirTagStatus, PimdirTagged, local_clock, parse_query},
    },
    shared::envelope::list::format_flags,
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
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let add = flags(&self.add)?;
        let remove = flags(&self.remove)?;
        let query = parse_query(&self.query, printer.is_json())?;

        let tag = client.tag_mail(query, &add, &remove, self.dry_run, &local_clock())?;
        printer.out(PimdirTagOutput {
            account: account.name().to_string(),
            query: self.query.join(" "),
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
    /// The name of the account tagged.
    #[serde(skip)]
    pub account: String,
    /// The query, its words joined by spaces.
    #[serde(skip)]
    pub query: String,
    /// One entry per mailbox holding each matched message.
    pub tagged: Vec<PimdirTagged>,
    /// What the query could not see: the tag reaches only what it found.
    pub coverage: PimdirMailCoverage,
}

impl fmt::Display for PimdirTagOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Pimdir));
        listing.head("Query", &self.query);

        for tagged in &self.tagged {
            // A placement whose flags were never read shows no flag.
            let flags = match tagged.status {
                PimdirTagStatus::UnknownFlags => String::from("    "),
                _ => {
                    let flags = tagged.flags.iter().map(Flag::from_raw).collect();
                    format_flags(&flags, None)
                }
            };

            let row = tagged.queue_id.map(|row| row.to_string());

            listing.items.push([
                sanitize::text(&tagged.id).into_owned(),
                flags,
                tagged.status.to_string(),
                sanitize::text(&tagged.mailbox).into_owned(),
                row.unwrap_or_default(),
                sanitize::text(&tagged.subject).into_owned(),
            ]);
        }

        self.coverage.note(&mut listing);

        write!(f, "{listing}")
    }
}
