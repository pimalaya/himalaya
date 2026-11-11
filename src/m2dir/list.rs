//! # m2dir list
//!
//! The `m2dir list` command, listing the folders under the store root.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{columns::Columns, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, m2dir::client::M2dirClient};

/// List m2dir folders found under the store root.
#[derive(Debug, Parser)]
pub struct M2dirMailboxListCommand;

impl M2dirMailboxListCommand {
    /// Lists the folders under the store root and prints them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut M2dirClient,
    ) -> Result<()> {
        let store = client.open_store()?;
        let m2dirs = client.list_m2dirs()?;

        let rows = m2dirs
            .into_iter()
            .map(|m2dir| {
                let name = store
                    .decode_folder_name(m2dir.path())
                    .unwrap_or_else(|| m2dir.path().as_str().to_owned());
                let path = m2dir.path().as_str().to_owned();
                M2dirRow { name, path }
            })
            .collect();

        printer.out(M2dirListOutput { rows })
    }
}

/// The `m2dir list` output, the folders of the store.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct M2dirListOutput {
    /// The folders found under the store root.
    #[serde(rename = "m2dirs")]
    pub rows: Vec<M2dirRow>,
}

impl fmt::Display for M2dirListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut columns = Columns::new(["NAME", "PATH"]);

        for m2dir in &self.rows {
            columns.push([sanitize::text(&m2dir.name), sanitize::text(&m2dir.path)]);
        }

        write!(f, "{columns}")
    }
}

/// One folder of the list.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct M2dirRow {
    /// The folder name, relative to the store root.
    pub name: String,
    /// Its filesystem path.
    pub path: String,
}
