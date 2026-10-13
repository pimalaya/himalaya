//! # pimdir attachment list
//!
//! The `pimdir attachment list` command, tabling one page of an account's
//! attachments, newest message first, and the output `pimdir attachment
//! search` shares.

use std::fmt;

use anyhow::Result;
use chrono::{DateTime, Local};
use clap::Parser;
use humansize::{BINARY, format_size};
use io_pimdir::client::reader::PimdirAccountAttachment;
use pimalaya_cli::{
    printer::Printer,
    table::{Cell, CellAlignment, ColumnConstraint, ContentArrangement, Row, Table, sanitize},
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    pimdir::{
        attachment::{AttachmentQuery, PimdirAttachmentPage, page_token, parse_page_token},
        client::PimdirClient,
    },
    shared::table::style_from_preset,
};

/// List the files the account's messages attach, newest message first.
///
/// Each file is one a message's body carries as an attachment, recorded
/// by the store as the body entered it: a message whose body is not
/// downloaded yet attaches nothing here. The listing reads no body.
///
/// A page ends with the token of the next one, which `--page-token`
/// takes; the last page has none. A token stays valid across syncs: the
/// next page starts after the last file shown, whatever arrived since.
///
/// A file's bytes are its message's part: read the message raw with
/// `message read --raw -m <MAILBOX> <MESSAGE-ID>`, then pipe it to
/// `message parse --section <SECTION> -`.
#[derive(Debug, Parser)]
pub struct PimdirAttachmentListCommand {
    #[command(flatten)]
    pub page: PageArgs,
}

/// The paging flags `list` and `search` share.
#[derive(Debug, Parser)]
pub struct PageArgs {
    /// Maximum number of files per page.
    ///
    /// Omitted, the configured `envelope.list.page-size` answers, and 25
    /// is the hard fallback.
    #[arg(long = "page-size", short = 's')]
    #[arg(value_name = "N", value_parser = clap::value_parser!(u32).range(1..))]
    pub page_size: Option<u32>,
    /// Start after the page that printed this token as its `nextPage`.
    #[arg(long = "page-token", short = 't')]
    #[arg(value_name = "TOKEN")]
    pub page_token: Option<String>,
    /// Maximum width of the rendered table, in terminal columns.
    #[arg(long = "max-width", short = 'w')]
    #[arg(value_name = "COLUMNS")]
    pub max_width: Option<u16>,
}

impl PageArgs {
    /// Reads the page these flags ask for, under `query` when given, and
    /// prints it.
    pub(super) fn print(
        self,
        printer: &mut impl Printer,
        account: &Account,
        client: &PimdirClient,
        query: Option<&AttachmentQuery>,
    ) -> Result<()> {
        let after = self
            .page_token
            .as_deref()
            .map(parse_page_token)
            .transpose()?;
        let limit = self
            .page_size
            .unwrap_or_else(|| account.envelopes_list_page_size())
            .max(1) as usize;
        let page = client.list_attachments(after.as_ref(), limit, query)?;

        let mut output = PimdirAttachments::from(page);
        output.preset = account.table_preset().to_string();
        output.arrangement = account.table_arrangement();
        output.max_width = self.max_width;
        output.datetime_fmt = account.datetime_fmt().to_string();
        output.datetime_local_tz = account.datetime_local_tz();
        printer.out(output)
    }
}

impl PimdirAttachmentListCommand {
    /// Lists one page of the account's attachments and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut PimdirClient,
    ) -> Result<()> {
        self.page.print(printer, account, client, None)
    }
}

/// One file a message attaches.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirAttachmentEntry {
    /// The file's public id in the store.
    pub id: String,
    /// Its name, decoded; `null` when the part names none.
    pub name: Option<String>,
    /// Its media type, lowercased, parameters dropped; `null` when the
    /// part states none.
    pub media_type: Option<String>,
    /// Its size in octets, transfer encoding undone.
    pub size: Option<u64>,
    /// Its IMAP section in the message (RFC 3501 §6.4.5: `2`, `1.2`),
    /// which `message parse --section` takes.
    pub section: Option<String>,
    /// The id of the message attaching it, which the message commands
    /// take with `mailbox`, as `envelope list` shows it.
    pub message_id: String,
    /// The mailbox of that message, a collection id.
    pub mailbox: String,
    /// The message's sender address.
    pub sender: Option<String>,
    /// The sender's display name.
    pub sender_name: Option<String>,
    /// The message's date, RFC 3339.
    pub date: Option<String>,
}

impl From<PimdirAccountAttachment> for PimdirAttachmentEntry {
    fn from(attachment: PimdirAccountAttachment) -> Self {
        let summary = attachment.summary.unwrap_or_default();
        Self {
            id: attachment.seq.to_string(),
            name: Some(summary.name).filter(|name| !name.is_empty()),
            media_type: summary.media_type,
            size: summary.size,
            section: summary.part,
            message_id: attachment.message_seq.to_string(),
            mailbox: attachment.message_collection,
            sender: attachment.sender,
            sender_name: attachment.sender_name,
            date: attachment.date,
        }
    }
}

/// The `pimdir attachment list` and `pimdir attachment search` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirAttachments {
    /// The `comfy_table` preset string the table renders with.
    #[serde(skip)]
    pub preset: String,
    /// The column arrangement the table renders with.
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    /// The width the table is capped at, when one was asked for.
    #[serde(skip)]
    pub max_width: Option<u16>,
    /// The chrono `strftime` format of the DATE column.
    #[serde(skip)]
    pub datetime_fmt: String,
    /// Whether a date is converted to the local timezone first.
    #[serde(skip)]
    pub datetime_local_tz: bool,
    /// The files of this page, newest message first.
    pub attachments: Vec<PimdirAttachmentEntry>,
    /// The token `--page-token` takes for the next page, `null` on the
    /// last one.
    pub next_page: Option<String>,
}

impl From<PimdirAttachmentPage> for PimdirAttachments {
    fn from(page: PimdirAttachmentPage) -> Self {
        Self {
            preset: String::new(),
            arrangement: ContentArrangement::Dynamic,
            max_width: None,
            datetime_fmt: String::new(),
            datetime_local_tz: false,
            attachments: page.attachments.into_iter().map(Into::into).collect(),
            next_page: page.next.as_ref().map(page_token),
        }
    }
}

impl PimdirAttachments {
    /// A date in the configured format, as stored when it does not read.
    fn format_date(&self, date: &str) -> String {
        match DateTime::parse_from_rfc3339(date) {
            Ok(date) if self.datetime_local_tz => date
                .with_timezone(&Local)
                .format(&self.datetime_fmt)
                .to_string(),
            Ok(date) => date.format(&self.datetime_fmt).to_string(),
            Err(_) => sanitize(date).into_owned(),
        }
    }
}

impl fmt::Display for PimdirAttachments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();
        table
            .load_style(style_from_preset(&self.preset))
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("NAME"),
                Cell::new("TYPE"),
                Cell::new("SIZE").set_alignment(CellAlignment::Right),
                Cell::new("SECTION"),
                Cell::new("MESSAGE"),
                Cell::new("MAILBOX"),
                Cell::new("FROM"),
                Cell::new("DATE"),
            ]))
            .add_rows(self.attachments.iter().map(|file| {
                let mut row = Row::new();
                row.max_height(1);
                row.add_cell(Cell::new(&file.id));
                row.add_cell(Cell::new(sanitize(
                    file.name.as_deref().unwrap_or_default(),
                )));
                row.add_cell(Cell::new(sanitize(
                    file.media_type.as_deref().unwrap_or_default(),
                )));
                row.add_cell(
                    Cell::new(
                        file.size
                            .map(|size| format_size(size, BINARY))
                            .unwrap_or_default(),
                    )
                    .set_alignment(CellAlignment::Right),
                );
                row.add_cell(Cell::new(sanitize(
                    file.section.as_deref().unwrap_or_default(),
                )));
                row.add_cell(Cell::new(&file.message_id));
                row.add_cell(Cell::new(sanitize(&file.mailbox)));
                let from = file.sender_name.as_deref().or(file.sender.as_deref());
                row.add_cell(Cell::new(sanitize(from.unwrap_or_default())));
                row.add_cell(Cell::new(
                    file.date
                        .as_deref()
                        .map(|date| self.format_date(date))
                        .unwrap_or_default(),
                ));
                row
            }));

        // NOTE: the ids are what follow-up commands take, so squeezing the
        // table into `--max-width` truncates any column but those.
        for at in [0, 5] {
            if let Some(column) = table.column_mut(at) {
                column.set_constraint(ColumnConstraint::ContentWidth);
            }
        }

        if let Some(width) = self.max_width {
            table.set_width(width);
        }

        writeln!(f)?;
        writeln!(f, "{table}")?;

        if let Some(token) = &self.next_page {
            writeln!(f, "Next page: {}", sanitize(token))?;
        }

        Ok(())
    }
}
