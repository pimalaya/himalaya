//! # pimdir attachment list
//!
//! The `pimdir attachment list` command, listing one page of an account's
//! attachments, newest message first, and the output `pimdir attachment
//! search` shares.

use std::fmt;

use anyhow::Result;
use chrono::{DateTime, Local};
use clap::Parser;
use humansize::{BINARY, format_size};
use io_pimdir::client::reader::PimdirAccountAttachment;
use pimalaya_cli::{
    fields::Fields,
    printer::Printer,
    records::{Record, Records},
    sanitize,
};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    pimdir::{
        attachment::{AttachmentQuery, PimdirAttachmentPage, page_token, parse_page_token},
        client::PimdirClient,
    },
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

        let mut output = PimdirAttachmentListOutput::from(page);
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
pub struct PimdirAttachmentListOutput {
    /// The chrono `strftime` format of a date.
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

impl From<PimdirAttachmentPage> for PimdirAttachmentListOutput {
    fn from(page: PimdirAttachmentPage) -> Self {
        Self {
            datetime_fmt: String::new(),
            datetime_local_tz: false,
            attachments: page.attachments.into_iter().map(Into::into).collect(),
            next_page: page.next.as_ref().map(page_token),
        }
    }
}

impl PimdirAttachmentListOutput {
    /// A date in the configured format, as stored when it does not read.
    fn format_date(&self, date: &str) -> String {
        match DateTime::parse_from_rfc3339(date) {
            Ok(date) if self.datetime_local_tz => date
                .with_timezone(&Local)
                .format(&self.datetime_fmt)
                .to_string(),
            Ok(date) => date.format(&self.datetime_fmt).to_string(),
            Err(_) => sanitize::text(date).into_owned(),
        }
    }
}

impl fmt::Display for PimdirAttachmentListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut records = Records::default();

        for file in &self.attachments {
            let text = |value: Option<&str>| sanitize::text(value.unwrap_or_default()).into_owned();
            let size = file.size.map(|size| format_size(size, BINARY));
            let date = file.date.as_deref().map(|date| self.format_date(date));
            let from = match (file.sender_name.as_deref(), file.sender.as_deref()) {
                (Some(name), Some(email)) if !name.is_empty() => format!("{name} <{email}>"),
                (name, email) => email.or(name).unwrap_or_default().to_owned(),
            };

            let mut fields = Fields::default();
            fields.push("Name", text(file.name.as_deref()));
            fields.push("Type", text(file.media_type.as_deref()));
            fields.push("Size", size.unwrap_or_default());
            fields.push("Section", text(file.section.as_deref()));
            fields.push("Message", sanitize::text(&file.message_id));
            fields.push("Mailbox", sanitize::text(&file.mailbox));
            fields.push("Date", date.unwrap_or_default());
            fields.push("From", sanitize::text(&from));

            records.push(Record {
                title: sanitize::text(&file.id).into_owned(),
                state: Vec::new(),
                fields,
                body: String::new(),
            });
        }

        write!(f, "{records}")?;

        if let Some(token) = &self.next_page {
            if !records.items.is_empty() {
                writeln!(f)?;
            }

            writeln!(f, "Next page: {}", sanitize::text(token))?;
        }

        Ok(())
    }
}
