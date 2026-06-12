use anyhow::Result;
use clap::Parser;
use io_imap::{
    rfc3501::{select::ImapMailboxSelectOptions, store::ImapMessageStoreOptions},
    types::{
        IntoStatic,
        flag::{Flag, StoreType},
    },
};
use pimalaya_cli::printer::{Message, Printer};

use crate::imap::{
    client::ImapClient,
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag},
};

/// Add IMAP flag(s) to message(s).
///
/// This command adds the given flags to messages identified by the
/// given sequence set.
#[derive(Debug, Parser)]
pub struct ImapFlagAddCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,

    /// The sequence set of messages (e.g., "1", "1,2,3", "1:*").
    #[arg(value_name = "SEQUENCE")]
    pub sequence_set: String,
    /// The flags to add (e.g., "\\Seen", "\\Flagged").
    #[arg(short, long, required = true, num_args = 1..)]
    pub flag: Vec<String>,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

impl ImapFlagAddCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;

        if !self.mailbox_no_select.inner {
            client.select(mailbox, ImapMailboxSelectOptions::default())?;
        }

        let sequence_set = self.sequence_set.as_str().try_into()?;
        let flags: Vec<Flag<'static>> = self
            .flag
            .iter()
            .map(|f| Flag::try_from(f.as_str()).map(|flag| flag.into_static()))
            .collect::<Result<_, _>>()?;

        client.store(
            sequence_set,
            StoreType::Add,
            flags,
            ImapMessageStoreOptions { uid: !self.seq },
        )?;

        printer.out(Message::new("Flag(s) successfully added"))
    }
}
