use anyhow::Result;
use clap::{Parser, ValueEnum};
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

/// Store IMAP flags on message(s) (STORE, RFC 3501).
///
/// Adds (`+FLAGS`), removes (`-FLAGS`) or replaces (`FLAGS`) the given
/// flags on every message in the sequence set, depending on --action.
#[derive(Debug, Parser)]
pub struct ImapStoreCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,

    /// The sequence set of messages (e.g. "1", "1,2,3", "1:*").
    #[arg(value_name = "SEQUENCE")]
    pub sequence_set: String,

    /// How to apply the flags.
    #[arg(long, value_name = "ACTION", default_value = "add")]
    pub action: StoreActionArg,

    /// The flags (e.g. "\\Seen", "\\Flagged").
    #[arg(short, long, required = true, num_args = 1..)]
    pub flag: Vec<String>,

    /// Use sequence numbers instead of UIDs.
    #[arg(long)]
    pub seq: bool,
}

/// STORE action: add (+FLAGS), remove (-FLAGS) or set (FLAGS).
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
#[clap(rename_all = "lower")]
pub enum StoreActionArg {
    #[default]
    Add,
    Remove,
    Set,
}

impl From<StoreActionArg> for StoreType {
    fn from(action: StoreActionArg) -> Self {
        match action {
            StoreActionArg::Add => StoreType::Add,
            StoreActionArg::Remove => StoreType::Remove,
            StoreActionArg::Set => StoreType::Replace,
        }
    }
}

impl ImapStoreCommand {
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
            self.action.into(),
            flags,
            ImapMessageStoreOptions { uid: !self.seq },
        )?;

        let outcome = match self.action {
            StoreActionArg::Add => "added",
            StoreActionArg::Remove => "removed",
            StoreActionArg::Set => "replaced",
        };

        printer.out(Message::new(format!("Flag(s) successfully {outcome}")))
    }
}
