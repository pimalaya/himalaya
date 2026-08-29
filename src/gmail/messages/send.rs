//! # Gmail message send
//!
//! The `gmail messages send` command, `users.messages.send`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::messages::{GmailMessage, encode_raw};
use pimalaya_cli::printer::{Message, Printer};

use crate::{gmail::client::GmailClient, shared::message::arg::MessageArg};

/// Send a Gmail message (users.messages.send).
#[derive(Debug, Parser)]
pub struct GmailMessageSendCommand {
    #[command(flatten)]
    pub message: MessageArg,
}

impl GmailMessageSendCommand {
    /// Sends the message and reports the id it landed under.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let raw = self.message.parse()?.into_bytes();
        let message = GmailMessage {
            raw: Some(encode_raw(&raw)),
            ..Default::default()
        };
        let id = client.message_send(&message)?.response;
        printer.out(Message::new(format!(
            "Gmail message `{}` successfully sent",
            id.id
        )))
    }
}
