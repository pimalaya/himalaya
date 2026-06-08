use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::shared::{
    client::EmailClient,
    message::{arg::MessageArg, handler},
};

/// Send a message via the active account.
///
/// Routes through SMTP or JMAP depending on the account's configured
/// outgoing backend. The envelope sender is taken from the `From:`
/// header and recipients are collected from `To:` / `Cc:` / `Bcc:`.
///
/// The message can be passed as a positional file path, an inline
/// raw string, or piped via stdin (see [`MessageArg`] for resolution
/// order). Pass `--save <MAILBOX>` to also append a copy of the
/// sent message to a mailbox; the mailbox name is resolved through
/// the account's `[mailbox.alias]` map before the backend call.
#[derive(Debug, Parser)]
pub struct MessageSendCommand {
    /// Append a copy of the sent message to this mailbox.
    #[arg(long, value_name = "MAILBOX")]
    pub save: Option<String>,

    #[command(flatten)]
    pub message: MessageArg,
}

impl MessageSendCommand {
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
