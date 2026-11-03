//! # IMAP close
//!
//! The `imap close` command, RFC 3501 `CLOSE`.

use anyhow::Result;
use clap::Parser;
use io_imap::client::ImapClient as _;
use pimalaya_cli::printer::Printer;

use crate::imap::{
    client::ImapClient,
    outcome::{self, ImapDoneOutput},
};

/// Close the selected mailbox (CLOSE, RFC 3501).
///
/// Expunges the messages flagged `\Deleted` and returns to the
/// authenticated state. A mailbox has to be selected, so this wants a
/// stateful IMAP session such as `himalaya repl` or a Sirup proxy.
#[derive(Debug, Parser)]
pub struct ImapMailboxCloseCommand;

impl ImapMailboxCloseCommand {
    /// Closes the selected mailbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let result = client.close().map_err(Into::into).map(|()| ImapDoneOutput {
            done: "IMAP mailbox successfully closed",
        });

        outcome::print(printer, result)
    }
}
