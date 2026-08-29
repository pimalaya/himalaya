//! # Message send
//!
//! The `message send` command, pushing a raw RFC 5322 message through the
//! account's outgoing backend.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    shared::{
        client::EmailClient,
        message::{arg::MessageArg, handler},
    },
};

/// Send a message through the active account.
///
/// The route is SMTP or JMAP, whichever the account configures. The
/// envelope sender comes from the `From:` header and the recipients from
/// `To:`, `Cc:` and `Bcc:`.
///
/// The message comes from a file path, an inline string or piped standard
/// input.
#[derive(Debug, Parser)]
pub struct MessageSendCommand {
    /// Append a copy of the sent message to this mailbox name or alias.
    #[arg(long, value_name = "MAILBOX")]
    pub save: Option<String>,
    #[command(flatten)]
    pub message: MessageArg,
}

impl MessageSendCommand {
    /// Sends the message, saving a copy when asked.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let raw = self.message.parse()?.into_bytes();
        handler::route(printer, account, client, raw, self.save.as_deref(), true)
    }
}
