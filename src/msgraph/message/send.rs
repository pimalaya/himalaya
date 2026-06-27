use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::{msgraph::client::MsgraphClient, shared::message::arg::MessageArg};

/// Send a Microsoft Graph message from raw MIME (`POST /me/sendMail`);
/// Graph saves it to Sent Items.
#[derive(Debug, Parser)]
pub struct MsgraphMessageSendCommand {
    #[command(flatten)]
    pub message: MessageArg,
}

impl MsgraphMessageSendCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let raw = self.message.parse()?.into_bytes();
        client.send_mail_mime(&raw)?;
        printer.out(Message::new("Microsoft Graph message successfully sent"))
    }
}
