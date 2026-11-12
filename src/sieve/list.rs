//! # ManageSieve list
//!
//! The `sieve list` command, RFC 5804 `LISTSCRIPTS`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account, backend::Backend, listing::Listing, sieve::client::SieveClient,
};

/// List all server-side Sieve scripts and mark the active script.
#[derive(Debug, Parser)]
pub struct SieveScriptListCommand;

impl SieveScriptListCommand {
    /// Lists the scripts, marking the active one.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &Account,
        client: &mut SieveClient,
    ) -> Result<()> {
        let scripts = client
            .list_scripts()?
            .into_iter()
            .map(|script| SieveScript {
                name: script.name,
                active: script.active,
            })
            .collect();

        printer.out(SieveListOutput {
            account: account.name().to_string(),
            scripts,
        })
    }
}

/// One stored script, by name and by whether it filters incoming mail.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SieveScript {
    /// The script name.
    pub name: String,
    /// Whether this is the script the server runs on incoming mail.
    pub active: bool,
}

/// The `sieve list` output, the scripts the account holds.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SieveListOutput {
    /// The name of the account listed.
    #[serde(skip)]
    pub account: String,
    /// The scripts the account holds, the active one marked.
    pub scripts: Vec<SieveScript>,
}

impl fmt::Display for SieveListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Sieve));

        for script in &self.scripts {
            let active = if script.active { "active" } else { "" };
            listing.items.push([active, &*sanitize::text(&script.name)]);
        }

        write!(f, "{listing}")
    }
}
