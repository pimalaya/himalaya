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
    listing::{Listing, ListingPage},
    printer::Printer,
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
    unified::{backend::Backend, email::address::Address},
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
/// A file is its message's part: `attachment download -m <MAILBOX>
/// <MESSAGE-ID> <PART>` writes it, the part numbered as `attachment list`
/// and `message read` number it. Its IMAP section is printed too, which
/// `message parse --section` takes.
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
    #[arg(long = "page-token")]
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
        let parts = client.part_ids(&page.attachments)?;

        let mut output = PimdirAttachmentListOutput::from(page);
        for (file, part) in output.attachments.iter_mut().zip(parts) {
            file.part = part;
        }
        output.account = account.name().to_string();
        output.query = query.map(|query| query.words.join(" "));
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
    /// Its file name, decoded; `null` when the part names none.
    pub filename: Option<String>,
    /// Its MIME type, lowercased, parameters dropped; `null` when the
    /// part states none.
    pub mime: Option<String>,
    /// Its size in octets, transfer encoding undone.
    pub size: Option<u64>,
    /// The id of its part in the message, the one `attachment download`
    /// takes and `attachment list` and `message read` print; `null` when
    /// the message's body is not local or holds no part at `section`.
    pub part: Option<String>,
    /// Its IMAP section in the message (RFC 3501 §6.4.5: `2`, `1.2`),
    /// which `message parse --section` takes.
    pub section: Option<String>,
    /// The id of the message attaching it, which the message commands
    /// take with `mailbox`, as `envelope list` shows it.
    pub message: String,
    /// The mailbox of that message, a collection id.
    pub mailbox: String,
    /// The message's sender, `null` when the store records no address.
    pub from: Option<Address>,
    /// The message's date, RFC 3339.
    pub date: Option<String>,
}

impl From<PimdirAccountAttachment> for PimdirAttachmentEntry {
    fn from(attachment: PimdirAccountAttachment) -> Self {
        let summary = attachment.summary.unwrap_or_default();
        Self {
            id: attachment.seq.to_string(),
            filename: Some(summary.name).filter(|name| !name.is_empty()),
            mime: summary.media_type,
            size: summary.size,
            part: None,
            section: summary.part,
            message: attachment.message_seq.to_string(),
            mailbox: attachment.message_collection,
            from: attachment
                .sender
                .filter(|email| !email.is_empty())
                .map(|email| Address {
                    name: attachment.sender_name.filter(|name| !name.is_empty()),
                    email,
                }),
            date: attachment.date,
        }
    }
}

/// The `pimdir attachment list` and `pimdir attachment search` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PimdirAttachmentListOutput {
    /// The name of the account listed.
    #[serde(skip)]
    pub account: String,
    /// The words searched, `None` for a listing.
    #[serde(skip)]
    pub query: Option<String>,
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
            account: String::new(),
            query: None,
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
        let mut listing = Listing::new(&self.account, Some(Backend::Pimdir.as_str()));

        if let Some(query) = &self.query {
            listing.head("Query", query);
        }

        for file in &self.attachments {
            let text = |value: Option<&str>| sanitize::text(value.unwrap_or_default()).into_owned();
            let size = file
                .size
                .map(|size| format_size(size, BINARY.space_after_value(false)));
            let date = file.date.as_deref().map(|date| self.format_date(date));
            let from = file
                .from
                .as_ref()
                .map(|from| from.name.as_deref().unwrap_or(&from.email));

            listing.items.push([
                sanitize::text(&file.id).into_owned(),
                size.unwrap_or_default(),
                text(file.mime.as_deref()),
                text(file.part.as_deref()),
                sanitize::text(&file.message).into_owned(),
                sanitize::text(&file.mailbox).into_owned(),
                date.unwrap_or_default(),
                text(from),
                text(file.filename.as_deref()),
            ]);
        }

        let count = self.attachments.len();
        let next = self.next_page.as_deref();
        listing.page(ListingPage::Token, count, ["file", "files"], next);

        write!(f, "{listing}")
    }
}

#[cfg(test)]
mod tests {
    use clap::{Parser, error::ErrorKind};

    use crate::cli::Cli;

    /// `-t` is `--to` across Himalaya, so the token of a page has no short
    /// letter, in the listing and in the search.
    #[test]
    fn the_page_token_takes_no_short_letter() {
        for verb in [&["list"][..], &["search", "facture"]] {
            let args = [&["himalaya", "pimdir", "attachment"], verb].concat();

            let long = [&args[..], &["--page-token", "x"]].concat();
            assert!(Cli::try_parse_from(long).is_ok(), "{verb:?}");

            let short = [&args[..], &["-t", "x"]].concat();
            let err = Cli::try_parse_from(short).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::UnknownArgument, "{verb:?}");
        }
    }
}
