use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::account::context::Account;
use crate::shared::{
    client::EmailClient, flag::arg::MessageIdsArg, mailbox::arg::resolve_mailbox_or_default,
};

/// Move message(s) from one mailbox to another within the active
/// account.
///
/// Both `--from` and `--to` are resolved through the account's
/// `[mailbox.alias]` map before the backend call. IMAP uses
/// `UID MOVE` (RFC 6851); JMAP uses `Email/set` patches that remove
/// the source and add the destination from each email's
/// `mailboxIds`; Maildir renames the underlying file. Cross-account
/// / cross-backend move is out of scope.
#[derive(Debug, Parser)]
pub struct MessageMoveCommand {
    #[command(flatten)]
    pub ids: MessageIdsArg,

    /// Source mailbox name or alias. Omit to fall back to the `inbox`
    /// alias (errors when none is configured, as the shared layer
    /// cannot guess a backend's inbox id).
    #[arg(long = "from", short = 'f', value_name = "NAME")]
    pub from: Option<String>,

    /// Destination mailbox name or alias. Mandatory.
    #[arg(long = "to", short = 't', value_name = "NAME")]
    pub to: String,
}

impl MessageMoveCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let from = resolve_mailbox_or_default(account, self.from.as_deref())?;
        let to = account.resolve_mailbox(&self.to).to_owned();
        let ids: Vec<&str> = self.ids.inner.iter().map(String::as_str).collect();
        let count = client.move_messages(&from, &to, &ids)?;
        let message = match count {
            0 => "No message moved: no id matched in the source mailbox".to_string(),
            1 => "1 message successfully moved".to_string(),
            n => format!("{n} messages successfully moved"),
        };
        printer.out(Message::new(message))
    }
}
