//! # IMAP list
//!
//! The `imap list` command, RFC 3501 `LIST` and `LSUB`.

use io_imap::client::ImapClient as _;
use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, Row, Table};
use io_imap::types::{core::QuotedChar, flag::FlagNameAttribute, mailbox::Mailbox};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account, email::mailbox::MailboxRole, imap::client::ImapClient,
    shared::table::style_from_preset,
};

/// List mailboxes (LIST and LSUB, RFC 3501).
///
/// `LSUB` lists the subscribed mailboxes, which is the default, and
/// `--all` switches to the `LIST` of every one.
#[derive(Debug, Parser)]
pub struct ImapMailboxListCommand {
    /// List every mailbox rather than the subscribed ones.
    #[arg(short = 'A', long)]
    pub all: bool,
    /// The reference name the listing starts from.
    #[arg(short, long, default_value = "")]
    pub reference: String,
    /// The name pattern to match, `*` and `%` being the wildcards.
    #[arg(short, long, default_value = "*")]
    pub pattern: String,
}

impl ImapMailboxListCommand {
    /// Lists the mailboxes and prints them as a table.
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

/// The `imap list` output, a table of mailboxes.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct MailboxesTable {
    /// The `comfy_table` preset string the table renders with.
    #[serde(skip)]
    pub preset: String,
    /// The color of the NAME column.
    #[serde(skip)]
    pub name_color: Color,
    /// The mailboxes, in the order the server returned them.
    pub mailboxes: Vec<MailboxRow>,
}

impl fmt::Display for MailboxesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_style(style_from_preset(&self.preset))
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

/// One row of the mailboxes table.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct MailboxRow {
    /// The mailbox name.
    pub name: String,
    /// The hierarchy delimiter, empty when the server reported none.
    pub delimiter: String,
    /// The name attributes the server reported, SPECIAL-USE included.
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
