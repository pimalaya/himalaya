use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::messages::{GmailMessage, encode_raw, import::GmailMessageImport};
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::{client::GmailClient, input::read_message};

/// Import a Gmail message into the mailbox (users.messages.import).
#[derive(Debug, Parser)]
pub struct GmailMessageImportCommand {
    /// Label id to apply to the imported message. Can be repeated.
    #[arg(long = "label", value_name = "ID")]
    pub labels: Vec<String>,
    /// The raw RFC 5322 message to import. Read from standard input
    /// when omitted.
    #[arg(value_name = "MESSAGE")]
    pub message: Option<String>,
}

impl GmailMessageImportCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let raw = read_message(self.message)?;

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
