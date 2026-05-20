// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::process::exit;

use anyhow::Result;
use ariadne::{Color, Label, Report, ReportKind, Source};
use clap::Parser;
use io_email::search::{error::Error as SearchQueryError, query::SearchEmailsQuery};
use pimalaya_cli::printer::Printer;

use crate::shared::{
    client::EmailClient,
    envelopes::list::{EnvelopeColors, Envelopes, FlagChars},
    mailboxes::arg::MailboxArg,
};

/// Search envelopes for the active account using the shared search
/// query DSL, regardless of the underlying backend (IMAP, JMAP or
/// Maildir).
///
/// The trailing positional accepts a filter and/or sort clause that
/// targets the `Date:` header (sent-at) for date clauses and
/// case-insensitive substring matching for text clauses. See the
/// `[QUERY]...` argument help below for the full syntax.
#[derive(Debug, Parser)]
pub struct EnvelopeSearchCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,

    /// Page number, starting from 1.
    #[arg(long, short = 'p')]
    #[arg(value_name = "N", default_value = "1")]
    pub page: u32,

    /// Maximum number of envelopes per page.
    ///
    /// When omitted, the merged `envelope.list.page-size` config
    /// value is used; when neither is set, the hard fallback is 25.
    #[arg(long = "page-size", short = 's')]
    #[arg(value_name = "N")]
    pub page_size: Option<u32>,

    /// Maximum width of the rendered table, in terminal columns.
    #[arg(long = "max-width", short = 'w')]
    #[arg(value_name = "COLUMNS")]
    pub max_width: Option<u16>,

    /// Render recipients (`To:`) instead of senders (`From:`).
    #[arg(long, short)]
    pub recipient: bool,

    /// Populate the ATT column.
    #[arg(long = "has-attachment")]
    pub has_attachment: bool,

    /// Filter and/or sort query.
    ///
    /// Conditions: `date <yyyy-mm-dd>`, `after <yyyy-mm-dd>`,
    /// `from <pattern>`, `to <pattern>`, `subject <pattern>`,
    /// `body <pattern>`, `flag <seen|answered|flagged|draft>`. Combine
    /// with `and`, `or`, `not`, group with parentheses. Sort with
    /// `order by <date|from|to|subject> [asc|desc]…`.
    #[arg(value_name = "QUERY")]
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    pub query: Option<Vec<String>>,
}

impl EnvelopeSearchCommand {
    pub fn execute(self, printer: &mut impl Printer, mut client: EmailClient) -> Result<()> {
        let page = Some(self.page).filter(|p| *p > 0);
        let page_size = self
            .page_size
            .or(Some(client.account.envelopes_list_page_size()))
            .filter(|p| *p > 0);
        let mailbox = self.mailbox.resolve(&client.account)?;
        let query = parse_query(self.query.as_deref());

        let envelopes = client.search_envelopes(
            &mailbox,
            query.as_ref(),
            page,
            page_size,
            self.has_attachment,
        )?;

        let envelopes = Envelopes {
            preset: client.account.table_preset().to_string(),
            arrangement: client.account.table_arrangement(),
            max_width: self.max_width,
            datetime_fmt: client.account.datetime_fmt().to_string(),
            datetime_local_tz: client.account.datetime_local_tz(),
            recipient: self.recipient,
            with_attachment: self.has_attachment,
            chars: FlagChars {
                unseen: client.account.envelopes_list_table_unseen_char(),
                replied: client.account.envelopes_list_table_replied_char(),
                flagged: client.account.envelopes_list_table_flagged_char(),
                attachment: client.account.envelopes_list_table_attachment_char(),
            },
            colors: EnvelopeColors {
                id: client.account.envelopes_list_table_id_color(),
                flags: client.account.envelopes_list_table_flags_color(),
                att: client.account.envelopes_list_table_att_color(),
                subject: client.account.envelopes_list_table_subject_color(),
                from: client.account.envelopes_list_table_from_color(),
                to: client.account.envelopes_list_table_to_color(),
                date: client.account.envelopes_list_table_date_color(),
                size: client.account.envelopes_list_table_size_color(),
            },
            envelopes,
        };

        printer.out(envelopes)
    }
}

/// Joins the trailing-positional words, feeds them to
/// [`SearchEmailsQuery::from_str`], and renders any parse error with
/// ariadne before exiting with code 1. Returns `None` when the input
/// is empty (no query) so `client.search_envelopes` keeps its default
/// behaviour.
fn parse_query(words: Option<&[String]>) -> Option<SearchEmailsQuery> {
    let words = words?;
    let joined = words
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    let trimmed = joined.trim();
    if trimmed.is_empty() {
        return None;
    }

    match trimmed.parse::<SearchEmailsQuery>() {
        Ok(query) => Some(query),
        Err(err) => {
            render_query_parse_error(&err);
            exit(1);
        }
    }
}

/// Pretty-prints a `chumsky` parse error to stderr using ariadne, one
/// labelled report per inner error, then leaves the caller to decide
/// what to do (we [`exit`]).
fn render_query_parse_error(err: &SearchQueryError) {
    let SearchQueryError::ParseError(errs, src) = err;
    let source_name = "query";
    for inner in errs {
        let range = inner.span().into_range();
        let _ = Report::build(ReportKind::Error, (source_name, range.clone()))
            .with_message(err.to_string())
            .with_label(
                Label::new((source_name, range))
                    .with_message(inner.reason().to_string())
                    .with_color(Color::Red),
            )
            .finish()
            .eprint((source_name, Source::from(src.as_str())));
    }
}
