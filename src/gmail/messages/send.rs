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
    /// Thread id to attach the sent message to.
    #[arg(long = "thread-id", value_name = "ID")]
    pub thread_id: Option<String>,
    #[command(flatten)]
    pub message: MessageArg,
}

impl GmailMessageSendCommand {
    /// Sends the message and reports the id it landed under.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let raw = self.message.parse()?;
        let message = GmailMessage {
            raw: Some(encode_raw(&raw)),
            thread_id: self.thread_id.clone(),
            ..Default::default()
        };
        let id = client.message_send(&message)?.response;
        printer.out(Message::new(format!(
            "Gmail message `{}` successfully sent",
            id.id
        )))
    }
}
