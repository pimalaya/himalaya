use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::messages::{GmailMessage, encode_raw};
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::{client::GmailClient, input::read_message};

/// Send a Gmail message (users.messages.send).
#[derive(Debug, Parser)]
pub struct GmailMessageSendCommand {
    /// The raw RFC 5322 message to send. Read from standard input when
    /// omitted.
    #[arg(value_name = "MESSAGE")]
    pub message: Option<String>,
}

impl GmailMessageSendCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let raw = read_message(self.message)?;
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
