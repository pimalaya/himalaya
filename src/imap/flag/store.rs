//! # IMAP store
//!
//! The `imap store` command, RFC 3501 `STORE`.

use anyhow::Result;
use clap::{Parser, ValueEnum};
use io_imap::client::ImapClient as _;
use io_imap::{
    rfc3501::{select::ImapMailboxSelectOptions, store::ImapMessageStoreOptions},
    types::{
        IntoStatic,
        flag::{Flag, StoreType},
    },
};
use pimalaya_cli::printer::Printer;

use crate::imap::{
    client::ImapClient,
    fetch::{FetchedMessage, ImapFetchOutput},
    mailbox::arg::{MailboxNameOptionalFlag, MailboxNoSelectFlag},
    outcome,
    utils::parse_sequence_set,
};

/// Store flags on messages (STORE, RFC 3501).
#[derive(Debug, Parser)]
pub struct ImapStoreCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameOptionalFlag,
    #[command(flatten)]
    pub mailbox_no_select: MailboxNoSelectFlag,
    /// The messages to store on, as `1`, `1,2,3` or `1:*`.
    #[arg(value_name = "SEQUENCE")]
    pub sequence_set: String,
    /// How to apply the flags.
    #[arg(long, value_name = "ACTION", default_value = "add")]
    pub action: StoreActionArg,
    /// Flags as raw RFC 3501 tokens.
    ///
    /// This is the raw IMAP API, not the unified
    /// `seen|answered|flagged|draft` enum: a system flag keeps its
    /// backslash, as in `-f '\Seen'`, and a bare word is a custom
    /// keyword, so `-f seen` stores the keyword `seen`.
    ///
    /// The unified `flag add -f seen` is the enum-mapped behaviour. Repeat
    /// `-f` for several flags.
    #[arg(short, long, required = true)]
    pub flag: Vec<String>,
    /// Read the sequence set as message numbers rather than UIDs.
    #[arg(long)]
    pub seq: bool,
}

/// How a `STORE` applies its flags.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
#[clap(rename_all = "lower")]
pub enum StoreActionArg {
    /// Add them to the existing set, `+FLAGS`.
    #[default]
    Add,
    /// Remove them from the existing set, `-FLAGS`.
    Remove,
    /// Replace the existing set, `FLAGS`.
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
    /// Selects the mailbox unless told not to, then stores the flags.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let mailbox = self.mailbox_name.inner.try_into()?;

        let sequence_set = parse_sequence_set(&self.sequence_set)?;
        let flags: Vec<Flag<'static>> = self
            .flag
            .iter()
            .map(|f| Flag::try_from(f.as_str()).map(|flag| flag.into_static()))
            .collect::<Result<_, _>>()?;

        let result = (|| -> Result<_> {
            if !self.mailbox_no_select.inner {
                client.select(mailbox, ImapMailboxSelectOptions::default())?;
            }

            let data = client.store(
                sequence_set,
                self.action.into(),
                flags,
                ImapMessageStoreOptions { uid: !self.seq },
            )?;

            let messages = data
                .into_iter()
                .map(|(seq, items)| FetchedMessage::from_items(seq.get(), items.into_iter()))
                .collect();

            Ok(ImapFetchOutput { messages })
        })();

        outcome::print(printer, result)
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::imap::flag::store::ImapStoreCommand;

    #[test]
    fn a_flag_before_the_sequence_leaves_it_positional() {
        let args = ["store", "-f", "\\Flagged", "10", "-f", "kw"];
        let cmd = ImapStoreCommand::try_parse_from(args).unwrap();

        assert_eq!(cmd.sequence_set, "10");
        assert_eq!(cmd.flag, ["\\Flagged", "kw"]);
    }
}
