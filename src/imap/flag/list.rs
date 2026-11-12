//! # IMAP flag list
//!
//! The `imap flag` command, reading the flags a `SELECT` response
//! reports.

use io_imap::client::ImapClient as _;
use std::{collections::BTreeMap, fmt};

use anyhow::Result;
use clap::Parser;
use io_imap::{
    rfc3501::select::ImapMailboxSelectOptions,
    types::flag::{Flag, FlagPerm},
};
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::{Serialize, Serializer};

use crate::{
    account::context::Account,
    backend::Backend,
    imap::{client::ImapClient, mailbox::arg::MailboxNameArg, outcome},
    listing::Listing,
};

/// List the flags a mailbox allows (SELECT response, RFC 3501).
///
/// The `FLAGS` and `PERMANENTFLAGS` the server returns on selecting it.
#[derive(Debug, Parser)]
pub struct ImapFlagListCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapFlagListCommand {
    /// Selects the mailbox and lists the flags it reported.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut ImapClient,
    ) -> Result<()> {
        let name = self.mailbox_name.inner.clone();
        let mailbox = self.mailbox_name.inner.try_into()?;

        let result = (|| -> Result<_> {
            let data = client.select(mailbox, ImapMailboxSelectOptions::default())?;
            let flags = data.flags.unwrap_or_default();
            let permanent_flags = data.permanent_flags.unwrap_or_default();

            Ok(ImapFlagListOutput {
                account: account.name().to_string(),
                mailbox: name,
                flags,
                permanent_flags,
            })
        })();

        outcome::print(printer, result)
    }
}

/// The `imap flag` output, the flags a mailbox allows and the ones it
/// keeps across sessions.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImapFlagListOutput<'a> {
    /// The name of the account listed.
    #[serde(skip)]
    account: String,
    /// The mailbox selected.
    #[serde(skip)]
    mailbox: String,
    #[serde(serialize_with = "serialize_flags")]
    #[schemars(with = "Vec<String>")]
    flags: Vec<Flag<'a>>,
    #[serde(serialize_with = "serialize_permanent_flags")]
    #[schemars(with = "Vec<String>")]
    permanent_flags: Vec<FlagPerm<'a>>,
}

impl ImapFlagListOutput<'_> {
    /// Pairs each flag with whether it is permanent.
    fn build_entries(&self) -> Vec<(String, bool)> {
        let mut entries: BTreeMap<String, bool> = BTreeMap::new();

        for flag in &self.flags {
            entries.entry(flag.to_string()).or_insert(false);
        }

        for flag in &self.permanent_flags {
            let name = match flag {
                FlagPerm::Flag(f) => f.to_string(),
                FlagPerm::Asterisk => "\\*".to_string(),
            };
            entries.insert(name, true);
        }

        entries.into_iter().collect()
    }
}

impl fmt::Display for ImapFlagListOutput<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Imap));
        listing.head("Mailbox", &self.mailbox);

        for (flag, perm) in self.build_entries() {
            let perm = if perm { "permanent" } else { "" };
            listing.items.push([&*sanitize::text(&flag), perm]);
        }

        write!(f, "{listing}")
    }
}

/// Serializes flags as their wire spellings.
pub fn serialize_flags<S: Serializer>(
    flags: &Vec<Flag<'_>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    flags
        .iter()
        .map(|f| f.to_string())
        .collect::<Vec<_>>()
        .serialize(serializer)
}

/// Serializes permanent flags as their wire spellings.
fn serialize_permanent_flags<S: Serializer>(
    flags: &Vec<FlagPerm<'_>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    flags
        .iter()
        .map(|f| match f {
            FlagPerm::Flag(f) => f.to_string(),
            FlagPerm::Asterisk => "\\*".to_string(),
        })
        .collect::<Vec<_>>()
        .serialize(serializer)
}
