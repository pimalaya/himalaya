//! # mbox list
//!
//! The `mbox list` command, listing the mbox files of the account.

use std::{fmt, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use io_mbox::mbox::Mbox;
use pimalaya_cli::{columns::Columns, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, mbox::client::MboxClient};

/// List mbox files.
///
/// Walks the account root and lists every mbox found, with its name and
/// filesystem path, plus the spool as `INBOX` when one is configured.
#[derive(Debug, Parser)]
pub struct MboxMailboxListCommand;

impl MboxMailboxListCommand {
    /// Lists the mbox files and prints them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut MboxClient,
    ) -> Result<()> {
        let mboxes = client.list_mboxes()?;

        printer.out(MboxListOutput {
            rows: mboxes.into_iter().map(From::from).collect(),
        })
    }
}

/// The `mbox list` output, the mbox files of the account.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MboxListOutput {
    /// The mbox files found.
    #[serde(rename = "mboxes")]
    pub rows: Vec<MboxRow>,
}

impl fmt::Display for MboxListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut columns = Columns::new(["NAME", "PATH"]);

        for mbox in &self.rows {
            let path = mbox.path.display().to_string();
            columns.push([sanitize::text(&mbox.name), sanitize::text(&path)]);
        }

        write!(f, "{columns}")
    }
}

/// One mbox file of the list.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MboxRow {
    /// The mbox name, relative to the account root.
    pub name: String,
    /// Its filesystem path.
    pub path: PathBuf,
}

impl From<Mbox> for MboxRow {
    fn from(mbox: Mbox) -> Self {
        Self {
            name: mbox.name.to_string(),
            path: PathBuf::from(mbox.path.as_str()),
        }
    }
}
