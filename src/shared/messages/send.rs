use std::io::{stdin, BufRead, IsTerminal};

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::shared::client::EmailClient;

/// Send a message via the active account.
///
/// Supported over JMAP. JMAP requires `identity-id` and
/// `drafts-mailbox-id` to be set on the account's `[jmap]` config block.
#[derive(Debug, Parser)]
pub struct MessageSendCommand {
    /// The raw message, including headers and body.
    #[arg(trailing_var_arg = true)]
    #[arg(name = "message", value_name = "MESSAGE")]
    pub message: Vec<String>,
}

impl MessageSendCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: EmailClient) -> Result<()> {
        let raw = if stdin().is_terminal() || printer.is_json() {
            self.message
                .join(" ")
                .replace('\r', "")
                .replace('\n', "\r\n")
        } else {
            stdin()
                .lock()
                .lines()
                .map_while(Result::ok)
                .collect::<Vec<String>>()
                .join("\r\n")
        };

        let opts = client.send_opts.clone();
        client.send_message(raw.into_bytes(), opts)?;
        printer.out(Message::new("Message successfully sent"))
    }
}
