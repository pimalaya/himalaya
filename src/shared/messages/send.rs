use std::io::{stdin, BufRead, IsTerminal};

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::shared::{client::EmailClient, messages::output::extract_envelope};

/// Send a message via the active account.
///
/// Routes through SMTP or JMAP depending on the account's configured
/// outgoing backend. The envelope sender is taken from the `From:`
/// header and recipients are collected from `To:` / `Cc:` / `Bcc:`.
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

        let raw = raw.into_bytes();
        let (from, to) = extract_envelope(&raw)?;
        let to_refs: Vec<&str> = to.iter().map(String::as_str).collect();
        client.send_message(raw, &from, &to_refs)?;
        printer.out(Message::new("Message successfully sent"))
    }
}
