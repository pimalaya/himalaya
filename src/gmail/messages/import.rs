//! # Gmail message import
//!
//! The `gmail messages import` command, `users.messages.import`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::messages::{GmailMessage, encode_raw, import::GmailMessageImport};
use pimalaya_cli::printer::{Message, Printer};

use crate::{gmail::client::GmailClient, shared::message::arg::MessageArg};

/// Import a Gmail message into the mailbox (users.messages.import).
#[derive(Debug, Parser)]
pub struct GmailMessageImportCommand {
    /// Label id to apply to the imported message. Can be repeated.
    #[arg(long = "label", value_name = "ID")]
    pub labels: Vec<String>,
    #[command(flatten)]
    pub message: MessageArg,
}

impl GmailMessageImportCommand {
    /// Imports the message, delivery rules and all.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let raw = self.message.parse()?.into_bytes();

        let message = GmailMessage {
            raw: Some(encode_raw(&raw)),
            label_ids: self.labels.clone(),
            ..Default::default()
        };

        let out = {
            let c = GmailMessageImport::new(
                &client.auth,
                &client.user_id,
                &message,
                None,
                false,
                false,
                false,
            )?;
            client.run(c)?
        };
        let message = out.response;

        printer.out(Message::new(format!(
            "Gmail message `{}` successfully imported",
            message.id
        )))
    }
}
