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

/// Send a raw RFC 5322 message as it is.
///
/// Nothing is built: the message goes out as given, its line endings
/// made CRLF, through the JMAP, Gmail or Microsoft Graph backend of the
/// account, queued on pimdir, and through SMTP otherwise. The envelope
/// sender comes from the `From:` header and the recipients from `To:`,
/// `Cc:` and `Bcc:`. A copy goes where `message.send.save-copy` puts it,
/// unless `--save` names another mailbox or `--no-save` keeps none.
///
/// The message comes from a file path, the message itself or piped
/// standard input. `message compose`, `reply` and `forward` build one
/// from flags instead.
#[derive(Debug, Parser)]
pub struct MessageSendCommand {
    /// Save a copy of the sent message to this mailbox, a name, alias or
    /// role, instead of where `message.send.save-copy` puts it.
    #[arg(long, value_name = "MAILBOX")]
    pub save: Option<String>,
    /// Keep no copy of the sent message, whatever
    /// `message.send.save-copy` says.
    #[arg(long, conflicts_with = "save")]
    pub no_save: bool,
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
        let raw = self.message.parse()?;
        let save = account.resolve_save(self.save.as_deref(), self.no_save, true);
        handler::route(printer, account, client, raw, save, true)
    }
}
