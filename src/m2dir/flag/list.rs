//! # m2dir flag list
//!
//! The `m2dir flag list` command, naming the flags set on one message.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{columns::Columns, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    m2dir::{
        arg::{M2dirNameFlag, MessageIdArg},
        client::M2dirClient,
    },
};

/// List flags set on an m2dir message.
///
/// Reads the `.meta/<id>.flags` metadata file and prints one flag per
/// row. Returns an empty list when the file is missing.
#[derive(Debug, Parser)]
pub struct M2dirFlagListCommand {
    #[command(flatten)]
    pub m2dir: M2dirNameFlag,
    #[command(flatten)]
    pub id: MessageIdArg,
}

impl M2dirFlagListCommand {
    /// Reads the flags set on the message and lists them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut M2dirClient,
    ) -> Result<()> {
        let store = client.open_store()?;
        let path = store.resolve_folder_path(&self.m2dir.inner)?;
        let m2dir = client.open_m2dir(path)?;
        let flags = client.read_flags(&m2dir, &self.id.inner)?;

        printer.out(M2dirFlagListOutput {
            flags: flags.iter().map(str::to_owned).collect(),
        })
    }
}

/// The `m2dir flag list` output, the flags set on an m2dir message.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct M2dirFlagListOutput {
    flags: Vec<String>,
}

impl fmt::Display for M2dirFlagListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut columns = Columns::new(["FLAG"]);

        for flag in &self.flags {
            columns.push([sanitize::text(flag)]);
        }

        write!(f, "{columns}")
    }
}
