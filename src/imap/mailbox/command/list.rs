use std::{fmt, ops::Deref};

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{presets, Cell, ContentArrangement, Row, Table};
use crossterm::style::Color;
use io_imap::coroutines::{list::*, lsub::*};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::Printer;
use serde::{Serialize, Serializer};

use crate::{config::ImapConfig, imap::stream};

/// List mailboxes.
///
/// This command allows you to list mailboxes from your IMAP account.
/// By default, only subscribed mailboxes are listed. Use --all to
/// list all mailboxes.
#[derive(Debug, Parser)]
pub struct ListMailboxesCommand {
    /// List all mailboxes, not just subscribed ones.
    #[arg(short = 'A', long)]
    pub all: bool,
}

impl ListMailboxesCommand {
    pub fn execute(self, printer: &mut impl Printer, config: ImapConfig) -> Result<()> {
        let (context, mut stream) = stream::connect(config)?;

        let mailboxes = if self.all {
            let mut arg = None;
            let mut coroutine =
                ImapList::new(context, "".try_into().unwrap(), "*".try_into().unwrap());

            loop {
                match coroutine.resume(arg.take()) {
                    ImapListResult::Io(io) => arg = Some(handle(&mut stream, io)?),
                    ImapListResult::Ok { mailboxes, .. } => break mailboxes,
                    ImapListResult::Err { err, .. } => bail!(err),
                }
            }
        } else {
            let mut arg = None;
            let mut coroutine =
                ImapLsub::new(context, "".try_into().unwrap(), "*".try_into().unwrap());

            loop {
                match coroutine.resume(arg.take()) {
                    ImapLsubResult::Io(io) => arg = Some(handle(&mut stream, io)?),
                    ImapLsubResult::Ok { mailboxes, .. } => break mailboxes,
                    ImapLsubResult::Err { err, .. } => bail!(err),
                }
            }
        };

        let table = MailboxesTable::from(mailboxes);

        printer.out(table)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ListMailboxesTableConfig {
    pub preset: Option<String>,
    pub name_color: Option<Color>,
    pub desc_color: Option<Color>,
}

impl ListMailboxesTableConfig {
    pub fn preset(&self) -> &str {
        self.preset.as_deref().unwrap_or(presets::ASCII_MARKDOWN)
    }

    pub fn name_color(&self) -> comfy_table::Color {
        map_color(self.name_color.unwrap_or(Color::Blue))
    }

    pub fn desc_color(&self) -> comfy_table::Color {
        map_color(self.desc_color.unwrap_or(Color::Green))
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Mailbox {
    pub name: String,
    pub delimiter: String,
    pub attributes: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Mailboxes(Vec<Mailbox>);

impl<T: IntoIterator<Item = Mailbox>> From<T> for Mailboxes {
    fn from(mboxes: T) -> Self {
        Self(mboxes.into_iter().collect())
    }
}

impl Deref for Mailboxes {
    type Target = Vec<Mailbox>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct MailboxesTable {
    mailboxes: Mailboxes,
    width: Option<u16>,
    config: ListMailboxesTableConfig,
}

impl MailboxesTable {
    pub fn with_some_width(mut self, width: Option<u16>) -> Self {
        self.width = width;
        self
    }

    pub fn with_some_preset(mut self, preset: Option<String>) -> Self {
        self.config.preset = preset;
        self
    }

    pub fn with_some_name_color(mut self, color: Option<Color>) -> Self {
        self.config.name_color = color;
        self
    }

    pub fn with_some_desc_color(mut self, color: Option<Color>) -> Self {
        self.config.desc_color = color;
        self
    }
}

impl
    From<
        Vec<(
            io_imap::types::mailbox::Mailbox<'static>,
            Option<io_imap::types::core::QuotedChar>,
            Vec<io_imap::types::flag::FlagNameAttribute<'static>>,
        )>,
    > for MailboxesTable
{
    fn from(
        mailboxes: Vec<(
            io_imap::types::mailbox::Mailbox<'static>,
            Option<io_imap::types::core::QuotedChar>,
            Vec<io_imap::types::flag::FlagNameAttribute<'static>>,
        )>,
    ) -> Self {
        Self {
            mailboxes: mailboxes
                .into_iter()
                .map(|(mbox, delim, attrs)| Mailbox {
                    name: match mbox {
                        io_imap::types::mailbox::Mailbox::Inbox => "Inbox".into(),
                        io_imap::types::mailbox::Mailbox::Other(mbox) => {
                            String::from_utf8_lossy(mbox.inner().as_ref()).to_string()
                        }
                    },
                    delimiter: match delim {
                        Some(delim) => delim.inner().to_string(),
                        None => String::new(),
                    },
                    attributes: attrs.into_iter().map(|attr| attr.to_string()).collect(),
                })
                .into(),
            width: None,
            config: Default::default(),
        }
    }
}

impl fmt::Display for MailboxesTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(self.config.preset())
            .set_content_arrangement(ContentArrangement::DynamicFullWidth)
            .set_header(Row::from([
                Cell::new("NAME"),
                Cell::new("DELIMITER"),
                Cell::new("ATTRIBUTES"),
            ]))
            .add_rows(self.mailboxes.iter().map(|mbox| {
                let mut row = Row::new();
                row.max_height(1);

                row.add_cell(Cell::new(&mbox.name).fg(self.config.name_color()));
                row.add_cell(Cell::new(&mbox.delimiter).fg(self.config.desc_color()));
                row.add_cell(Cell::new(&mbox.attributes.join(", ")).fg(self.config.desc_color()));

                row
            }));

        if let Some(width) = self.width {
            table.set_width(width);
        }

        writeln!(f)?;
        write!(f, "{table}")?;
        writeln!(f)?;
        Ok(())
    }
}

impl Serialize for MailboxesTable {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.mailboxes.serialize(serializer)
    }
}

fn map_color(color: Color) -> comfy_table::Color {
    match color {
        Color::Reset => comfy_table::Color::Reset,
        Color::Black => comfy_table::Color::Black,
        Color::DarkGrey => comfy_table::Color::DarkGrey,
        Color::Red => comfy_table::Color::Red,
        Color::DarkRed => comfy_table::Color::DarkRed,
        Color::Green => comfy_table::Color::Green,
        Color::DarkGreen => comfy_table::Color::DarkGreen,
        Color::Yellow => comfy_table::Color::Yellow,
        Color::DarkYellow => comfy_table::Color::DarkYellow,
        Color::Blue => comfy_table::Color::Blue,
        Color::DarkBlue => comfy_table::Color::DarkBlue,
        Color::Magenta => comfy_table::Color::Magenta,
        Color::DarkMagenta => comfy_table::Color::DarkMagenta,
        Color::Cyan => comfy_table::Color::Cyan,
        Color::DarkCyan => comfy_table::Color::DarkCyan,
        Color::White => comfy_table::Color::White,
        Color::Grey => comfy_table::Color::Grey,
        Color::Rgb { r, g, b } => comfy_table::Color::Rgb { r, g, b },
        Color::AnsiValue(n) => comfy_table::Color::AnsiValue(n),
    }
}
