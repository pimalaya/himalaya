//! # Gmail message insert
//!
//! The `gmail messages insert` command, `users.messages.insert`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::messages::{GmailMessage, encode_raw, insert::GmailMessageInsert};
use pimalaya_cli::printer::{Message, Printer};

use crate::{gmail::client::GmailClient, shared::message::arg::MessageArg};

/// Insert a Gmail message into the mailbox without sending
/// (users.messages.insert).
#[derive(Debug, Parser)]
pub struct GmailMessageInsertCommand {
    /// Label id to apply to the inserted message. Can be repeated.
    #[arg(long = "label", value_name = "ID")]
    pub labels: Vec<String>,
    #[command(flatten)]
    pub message: MessageArg,
}

impl GmailMessageInsertCommand {
    /// Inserts the message without applying any delivery rule.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let raw = self.message.parse()?.into_bytes();

        let message = GmailMessage {
            raw: Some(encode_raw(&raw)),
            label_ids: self.labels.clone(),
            ..Default::default()
        };

        let out = {
            let c = GmailMessageInsert::new(&client.auth, &client.user_id, &message, None, false)?;
            client.run(c)?
        };
        let message = out.response;

        printer.out(Message::new(format!(
            "Gmail message `{}` successfully inserted",
            message.id
        )))
    }
}
