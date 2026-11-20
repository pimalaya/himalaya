//! # Message add
//!
//! The `message add` command, appending a raw RFC 5322 message to a
//! mailbox.

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;

use crate::{
    account::context::Account,
    email::flag::Flag,
    shared::{
        client::EmailClient,
        flag::arg::FlagArg,
        message::{
            arg::MessageArg,
            handler::{self, MessageRouteOutput},
        },
    },
};

/// Add a raw RFC 5322 message to a mailbox as it is.
///
/// Nothing is built: the message is saved as given, its line endings
/// made CRLF, with the flags `--flag` sets. It comes from a file path,
/// the message itself or piped standard input. `message compose`,
/// `reply` and `forward` build one from flags instead, and save it with
/// `--save`.
#[derive(Debug, Parser)]
pub struct MessageAddCommand {
    /// Destination mailbox name, alias or role.
    #[arg(long = "mailbox", short = 'm', value_name = "NAME")]
    pub mailbox: String,
    /// Flags to set on the new message.
    #[arg(long = "flag", short = 'f', value_name = "FLAG")]
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

        let Some(output) = MessageRouteOutput::new(outcome, queued) else {
            unreachable!("--mailbox is mandatory: handler::apply saves, or sends and fails")
        };

        output.print(printer)
    }
}

#[cfg(test)]
mod tests {
    use crate::shared::message::handler::{MessageRouteOutput, Outcome};

    /// A queued add carries the receipt keys of every send and save, its
    /// send not done yet.
    #[test]
    fn a_queued_add_says_it_is_queued() {
        let output = |queued| {
            let id = Some("d1@x.org".into());
            MessageRouteOutput::new(Outcome::Saved { id, sent: true }, queued).unwrap()
        };

        assert_eq!(
            output(true).to_string(),
            "Message queued for saving and sending\n"
        );
        assert_eq!(
            output(false).to_string(),
            "Message d1@x.org successfully saved and sent\n"
        );
        assert_eq!(
            serde_json::to_string(&output(true)).unwrap(),
            r#"{"id":"d1@x.org","saved":true,"sent":false,"queued":true,"errors":[]}"#
        );
    }
}
