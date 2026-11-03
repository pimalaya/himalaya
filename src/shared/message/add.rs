//! # Message add
//!
//! The `message add` command, appending a raw RFC 5322 message to a
//! mailbox.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    email::flag::Flag,
    shared::{
        client::EmailClient,
        flag::arg::FlagArg,
        message::{
            arg::MessageArg,
            handler::{self, Outcome},
        },
    },
};

/// Add a raw RFC 5322 message to a mailbox.
///
/// The message comes from a file path, an inline string or piped standard
/// input, and the mailbox is resolved through the account's aliases.
#[derive(Debug, Parser)]
pub struct MessageAddCommand {
    /// Destination mailbox name or alias.
    #[arg(long = "mailbox", short = 'm', value_name = "NAME")]
    pub mailbox: String,
    /// Flags to set on the new message.
    #[arg(long = "flag", short = 'f', value_name = "FLAG", num_args = 0..)]
    pub flag: Vec<FlagArg>,
    /// Send the message too, before appending it, as `message send
    /// --save` does.
    #[arg(long)]
    pub send: bool,
    #[command(flatten)]
    pub message: MessageArg,
}

impl MessageAddCommand {
    /// Appends the message, sends it when asked, and reports its new id.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let raw = self.message.parse()?;
        let flags: Vec<Flag> = self.flag.iter().map(Into::into).collect();
        let queued = client.queues_writes();
        let outcome = handler::apply(account, client, raw, &flags, Some(&self.mailbox), self.send)?;
        let Outcome::Saved { id, sent } = outcome else {
            unreachable!("--mailbox is mandatory; handler::apply always reports Saved");
        };
        printer.out(MessageAddOutput { id, sent, queued })
    }
}

/// The `message add` output, naming the message that was appended.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MessageAddOutput {
    id: Option<String>,
    sent: bool,
    /// Whether a sync engine applies the write later rather than now.
    queued: bool,
}

impl fmt::Display for MessageAddOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.queued {
            let suffix = if self.sent { " and sending" } else { "" };
            return match &self.id {
                Some(id) => writeln!(f, "Message {id} queued for adding{suffix}"),
                None => writeln!(f, "Message queued for adding{suffix}"),
            };
        }

        let suffix = if self.sent { " and sent" } else { "" };
        match &self.id {
            Some(id) => writeln!(f, "Message {id} successfully added{suffix}"),
            None => writeln!(f, "Message successfully added{suffix}, id unavailable"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::shared::message::add::MessageAddOutput;

    #[test]
    fn a_queued_add_says_it_is_queued() {
        let output = |queued| MessageAddOutput {
            id: Some("d1@x.org".into()),
            sent: true,
            queued,
        };

        assert_eq!(
            output(true).to_string(),
            "Message d1@x.org queued for adding and sending\n"
        );
        assert_eq!(
            output(false).to_string(),
            "Message d1@x.org successfully added and sent\n"
        );
        assert_eq!(
            serde_json::to_string(&output(true)).unwrap(),
            r#"{"id":"d1@x.org","sent":true,"queued":true}"#
        );
    }
}
