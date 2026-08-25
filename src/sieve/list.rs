use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Row, Table};
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account, shared::table::style_from_preset, sieve::client::SieveClient,
};

/// List all server-side Sieve scripts and mark the active script.
#[derive(Debug, Parser)]
pub struct SieveScriptListCommand;

impl SieveScriptListCommand {
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

        printer.out(SieveScripts {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            scripts,
        })
    }
}

/// One stored script, by name and by whether it filters incoming mail.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct SieveScript {
    /// The script name.
    pub name: String,
    /// Whether this is the script the server runs on incoming mail.
    pub active: bool,
}

/// Structured output for `sieve list`.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct SieveScripts {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    pub scripts: Vec<SieveScript>,
}

impl fmt::Display for SieveScripts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();
        table
            .load_style(style_from_preset(&self.preset))
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([Cell::new("NAME"), Cell::new("ACTIVE")]));

        for script in &self.scripts {
            table.add_row(Row::from([
                Cell::new(&script.name),
                Cell::new(if script.active { "yes" } else { "" }),
            ]));
        }

        writeln!(f)?;
        write!(f, "{table}")
    }
}
