use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::{msgraph::client::MsgraphClient, shared::message::arg::MessageArg};

/// Create a Microsoft Graph draft message from raw MIME (`POST
/// /me/messages`).
#[derive(Debug, Parser)]
pub struct MsgraphMessageCreateCommand {
    /// Create the draft in this folder id or well-known name (e.g.
    /// `drafts`). Defaults to the mailbox root.
    #[arg(short = 'f', long, value_name = "ID")]
    pub folder: Option<String>,

    #[command(flatten)]
    pub message: MessageArg,
}

impl MsgraphMessageCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let raw = self.message.parse()?.into_bytes();
        let message = client
            .message_create_mime(self.folder.as_deref(), &raw)?
            .response;
        printer.out(Message::new(format!(
            "Microsoft Graph draft `{}` successfully created",
            message.id
        )))
    }
}
