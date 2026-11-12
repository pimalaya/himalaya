//! # Maildir list
//!
//! The `maildir list` command, listing the Maildirs under the store
//! root.

use std::{fmt, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account, backend::Backend, listing::Listing, maildir::client::MaildirClient,
};

/// List Maildir folders.
///
/// Scans the account root and lists every folder found, with its name
/// and filesystem path.
#[derive(Debug, Parser)]
pub struct MaildirMailboxListCommand;

impl MaildirMailboxListCommand {
    /// Lists the Maildirs under the store root and prints them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MaildirClient,
    ) -> Result<()> {
        let maildirs = client.list_maildirs()?;

        let table = MaildirListOutput {
            account: account.name().to_string(),
            rows: maildirs
                .into_iter()
                .map(|maildir| MaildirRow {
                    name: client.mailbox_name(&maildir),
                    path: PathBuf::from(maildir.path().as_str()),
                })
                .collect(),
        };

        printer.out(table)
    }
}

/// The `maildir list` output, the Maildirs of the store.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MaildirListOutput {
    /// The name of the account listed.
    #[serde(skip)]
    pub account: String,
    /// The Maildirs found under the store root.
    #[serde(rename = "maildirs")]
    pub rows: Vec<MaildirRow>,
}

impl fmt::Display for MaildirListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Maildir));

        for maildir in &self.rows {
            let path = maildir.path.display().to_string();
            listing
                .items
                .push([sanitize::text(&maildir.name), sanitize::text(&path)]);
        }

        write!(f, "{listing}")
    }
}

/// One Maildir of the list.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MaildirRow {
    /// The Maildir name, relative to the store root.
    pub name: String,
    /// Its filesystem path.
    pub path: PathBuf,
}
