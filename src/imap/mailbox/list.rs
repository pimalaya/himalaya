use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, Row, Table};
use io_email::mailbox::types::MailboxRole;
use io_imap::types::{core::QuotedChar, flag::FlagNameAttribute, mailbox::Mailbox};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::imap::client::ImapClient;

/// List mailboxes (LIST / LSUB, RFC 3501).
///
/// Lists subscribed mailboxes (LSUB) by default, or every mailbox
/// (LIST) with --all.
#[derive(Debug, Parser)]
pub struct ImapMailboxListCommand {
    /// List all mailboxes, not just subscribed ones.
    #[arg(short = 'A', long)]
    pub all: bool,
    /// The reference name for the LIST/LSUB command.
    #[arg(short, long, default_value = "")]
    pub reference: String,
    /// The mailbox name pattern with wildcards (* and %).
    #[arg(short, long, default_value = "*")]
    pub pattern: String,
}

impl ImapMailboxListCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut ImapClient,
    ) -> Result<()> {
        let reference = self.reference.try_into()?;
        let pattern = self.pattern.try_into()?;

        let mailboxes = if self.all {
            client.list(reference, pattern)?
        } else {
            client.lsub(reference, pattern)?
        };

        let table = MailboxesTable {
            preset: account.table_preset().to_string(),
            name_color: account.mailboxes_list_table_name_color(),
            mailboxes: mailboxes.into_iter().map(From::from).collect(),
        };

        printer.out(table)
    }
}

/// Renderable table of LIST/LSUB mailboxes.
#[derive(Clone, Debug, Serialize)]
pub struct MailboxesTable {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub name_color: Color,
    pub mailboxes: Vec<MailboxRow>,
}

impl fmt::Display for MailboxesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_header(Row::from([
                Cell::new("NAME"),
                Cell::new("DELIMITER"),
                Cell::new("ROLE"),
                Cell::new("ATTRIBUTES"),
            ]))
            .add_rows(self.mailboxes.iter().map(|mbox| {
                let mut row = Row::new();

                let role = mbox
                    .attributes
                    .iter()
                    .find_map(|raw| match MailboxRole::parse(raw) {
                        MailboxRole::Other(_) => None,
                        role => Some(format!("{role:?}")),
                    })
                    .unwrap_or_default();

                row.max_height(1)
                    .add_cell(Cell::new(&mbox.name).fg(self.name_color))
                    .add_cell(Cell::new(&mbox.delimiter))
                    .add_cell(Cell::new(role))
                    .add_cell(Cell::new(mbox.attributes.join(", ")));

                row
            }));

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

/// One row of the mailboxes table: name, delimiter and attributes.
#[derive(Clone, Debug, Serialize)]
pub struct MailboxRow {
    pub name: String,
    pub delimiter: String,
    pub attributes: Vec<String>,
}

impl
    From<(
        Mailbox<'static>,
        Option<QuotedChar>,
        Vec<FlagNameAttribute<'static>>,
    )> for MailboxRow
{
    fn from(
        (mbox, delim, attrs): (
            Mailbox<'static>,
            Option<QuotedChar>,
            Vec<FlagNameAttribute<'static>>,
        ),
    ) -> Self {
        Self {
            name: match mbox {
                Mailbox::Inbox => "Inbox".into(),
                Mailbox::Other(mbox) => String::from_utf8_lossy(mbox.inner().as_ref()).to_string(),
            },
            delimiter: match delim {
                Some(delim) => delim.inner().to_string(),
                None => String::new(),
            },
            attributes: attrs.into_iter().map(|attr| attr.to_string()).collect(),
        }
    }
}
