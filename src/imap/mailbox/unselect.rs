//! # IMAP unselect
//!
//! The `imap unselect` command, RFC 3691 `UNSELECT`.

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use pimalaya_cli::printer::Printer;

use crate::imap::{
    client::ImapClient,
    outcome::{self, ImapDoneOutput},
};

/// Unselect the selected mailbox (UNSELECT, RFC 3691).
///
/// `CLOSE` without the expunge. A mailbox has to be selected, so this
/// wants a stateful IMAP session such as `himalaya repl` or a Sirup
/// proxy.
#[derive(Debug, Parser)]
pub struct ImapMailboxUnselectCommand;

impl ImapMailboxUnselectCommand {
    /// Unselects the selected mailbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let result = client
            .unselect()
            .map_err(Into::into)
            .map(|()| ImapDoneOutput {
                done: "IMAP mailbox successfully unselected",
            });

        outcome::print(printer, result)
    }
}
