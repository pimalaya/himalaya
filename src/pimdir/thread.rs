//! # pimdir thread
//!
//! The `pimdir thread` command, listing the thread of one message through
//! the store's search index.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    pimdir::{
        client::PimdirClient,
        index::{PimdirMailThread, PimdirTableStyle},
    },
};

/// List the thread of a message, oldest first.
///
/// The thread is read from the store's search index, which joins a reply
/// to its parent by `In-Reply-To`, else `References`, never by subject. A
/// message no thread holds is listed alone.
#[derive(Debug, Parser)]
pub struct PimdirThreadCommand {
    /// The message id, as a listing prints it.
    #[arg(value_name = "ID")]
    pub id: String,
    /// Maximum width of the rendered table, in terminal columns.
    #[arg(long = "max-width", short = 'w')]
    #[arg(value_name = "COLUMNS")]
    pub max_width: Option<u16>,
}

impl PimdirThreadCommand {
    /// Reads the thread and prints its messages.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        let thread = client.mail_thread(&self.id)?;
        printer.out(PimdirThreadOutput {
            style: PimdirTableStyle::new(account, self.max_width),
            thread,
        })
    }
}

/// The `pimdir thread` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirThreadOutput {
    /// How the table renders.
    #[serde(skip)]
    pub style: PimdirTableStyle,
    /// The thread, every message of it in the account, oldest first.
    #[serde(flatten)]
    pub thread: PimdirMailThread,
}

impl fmt::Display for PimdirThreadOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;
        writeln!(f, "{}", self.style.hits(&self.thread.hits))
    }
}
