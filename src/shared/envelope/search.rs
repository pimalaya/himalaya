//! # Envelope search
//!
//! The `envelope search` command, listing a mailbox through the shared
//! filter and sort query.

use std::io::{IsTerminal, stdout};

use anyhow::{Context, Result};
use ariadne::{Color, Config, Label, Report, ReportKind, Source};
use clap::Parser;
use pimalaya_cli::{
    clap::args::PageTokenArg, error::CodedError, output::Paginated, printer::Printer,
};

use crate::{
    account::context::Account,
    capability::Capability,
    email::search::{error::Error as SearchQueryError, query::SearchEmailsQuery},
    error::ErrorCode,
    shared::{
        client::EmailClient,
        envelope::list::{EnvelopeColors, EnvelopeListOutput, FlagChars},
        mailbox::arg::MailboxArg,
    },
};

/// Search the envelopes of a mailbox with the shared query language.
///
/// A date clause reads the `Date:` header, the sent-at. A text clause is
/// matched as the backend matches text: a case-insensitive substring on IMAP,
/// Maildir, m2dir, mbox and pimdir, whole words over the whole message on
/// Gmail, KQL terms on Microsoft Graph, and as the server defines it on JMAP.
/// A clause the backend cannot search is refused with `search-unsupported`,
/// never dropped: every flag on Microsoft Graph, `flag answered` and keywords
/// on Gmail, `attachment` on IMAP.
#[derive(Debug, Parser)]
pub struct EnvelopeSearchCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    /// Page number, starting at 1.
    #[arg(long, short = 'p')]
    #[arg(value_name = "N", default_value = "1")]
    pub page: u32,
    /// Maximum number of envelopes per page.
    ///
    /// Omitted, the configured `envelope.list.page-size` answers, and 25
    /// is the hard fallback.
    #[arg(long = "page-size", short = 's')]
    #[arg(value_name = "N")]
    pub page_size: Option<u32>,
    #[command(flatten)]
    pub page_token: PageTokenArg,
    /// Maximum width of the rendered table, in terminal columns.
    #[arg(long = "max-width", short = 'w')]
    #[arg(value_name = "COLUMNS")]
    pub max_width: Option<u16>,
    /// Render recipients instead of senders.
    #[arg(long, short)]
    pub recipient: bool,
    /// Fill the ATT column.
    #[arg(long = "has-attachment")]
    pub has_attachment: bool,
    /// Query: date, after, before, from, to, cc, bcc, address, subject, body,
    /// flag, attachment; and, or, not, ( ); order by. See --help.
    ///
    /// Clauses:
    ///
    /// - `date <yyyy-mm-dd>`: sent on that day;
    /// - `after <yyyy-mm-dd>`, `before <yyyy-mm-dd>`: sent strictly after,
    ///   strictly before that day;
    /// - `from <pattern>`, `to <pattern>`, `cc <pattern>`, `bcc <pattern>`:
    ///   that header holds the pattern, in a name or an address;
    /// - `address <pattern>`: any of those four headers holds it;
    /// - `subject <pattern>`, `body <pattern>`: the subject, a text body;
    /// - `flag <name>`: carrying the flag, `seen`, `answered`, `flagged`,
    ///   `draft` or a keyword;
    /// - `attachment`: holding an attachment.
    ///
    /// Dates also read `yyyy/mm/dd`, `dd-mm-yyyy` and `dd/mm/yyyy`. A pattern
    /// holding spaces is quoted, `"weekly report"`, or its spaces are
    /// escaped, `weekly\ report`.
    ///
    /// Clauses join with `not`, `and` and `or`, binding in that order, and
    /// group with parentheses: `from alice and (subject report or
    /// attachment)`.
    ///
    /// The sort comes last, `order by <date|from|to|subject> [asc|desc]...`,
    /// several keys applying left to right: `from alice order by date desc`.
    #[arg(value_name = "QUERY")]
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    pub query: Option<Vec<String>>,
}

impl EnvelopeSearchCommand {
    /// What the search reaches: `envelope.search` and the options its
    /// query uses, a query that does not parse failing here already.
    pub fn capabilities(&self) -> Result<Vec<Capability>> {
        let mut capabilities = vec![Capability::EnvelopeSearch];

        if let Some(query) = parse_query(self.query.as_deref())? {
            capabilities.extend(query.options().into_iter().map(Capability::Search));
        }

        Ok(capabilities)
    }

    /// Searches the mailbox and prints one page of hits as a table.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let page = match &self.page_token.token {
            Some(token) => token
                .parse()
                .with_context(|| format!("Invalid page token {token}"))?,
            None => self.page,
        };
        let page = Some(page).filter(|p| *p > 0);
        let page_size = self
            .page_size
            .or(Some(account.envelopes_list_page_size()))
            .filter(|p| *p > 0);
        let mailbox = self.mailbox.resolve(account);
        let query = parse_query(self.query.as_deref())?;

        let envelopes = client.search_envelopes(
            &mailbox,
            query.as_ref(),
            page,
            page_size,
            self.has_attachment,
        )?;

        // NOTE: the token is the next page number, given while a page comes
        // back full; the page after a full last one comes back empty.
        let next_page = match (page, page_size) {
            (Some(page), Some(size)) if envelopes.len() >= size as usize => {
                Some((page + 1).to_string())
            }
            _ => None,
        };

        let envelopes = EnvelopeListOutput {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            max_width: self.max_width,
            datetime_fmt: account.datetime_fmt().to_string(),
            datetime_local_tz: account.datetime_local_tz(),
            datetime_relative: account.datetime_relative(),
            recipient: self.recipient,
            with_attachment: self.has_attachment,
            chars: FlagChars {
                unseen: account.envelopes_list_table_unseen_char(),
                replied: account.envelopes_list_table_replied_char(),
                flagged: account.envelopes_list_table_flagged_char(),
                attachment: account.envelopes_list_table_attachment_char(),
            },
            colors: EnvelopeColors {
                id: account.envelopes_list_table_id_color(),
                flags: account.envelopes_list_table_flags_color(),
                att: account.envelopes_list_table_att_color(),
                subject: account.envelopes_list_table_subject_color(),
                from: account.envelopes_list_table_from_color(),
                to: account.envelopes_list_table_to_color(),
                date: account.envelopes_list_table_date_color(),
                size: account.envelopes_list_table_size_color(),
            },
            envelopes,
        };

        printer.out(Paginated {
            inner: envelopes,
            next_page,
        })
    }
}

/// Parses the trailing positional into a query, `None` when it is empty
/// so the search keeps its default behaviour.
fn parse_query(words: Option<&[String]>) -> Result<Option<SearchEmailsQuery>> {
    let Some(words) = words else {
        return Ok(None);
    };

    let joined = words
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    let trimmed = joined.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    match trimmed.parse::<SearchEmailsQuery>() {
        Ok(query) => Ok(Some(query)),
        Err(err) => {
            Err(CodedError::new(ErrorCode::SearchInvalid, render_query_parse_error(&err)).into())
        }
    }
}

/// Renders a parse error with ariadne, one labelled report per inner
/// error, into the string the caller raises through stdout.
///
/// Color is dropped when stdout is not a terminal.
fn render_query_parse_error(err: &SearchQueryError) -> String {
    let SearchQueryError::ParseError(errs, src) = err;
    let source_name = "query";
    let config = Config::default().with_color(stdout().is_terminal());
    let mut buf = Vec::new();

    for inner in errs {
        let range = inner.span().into_range();
        let _ = Report::build(ReportKind::Error, (source_name, range.clone()))
            .with_config(config)
            .with_message(err.to_string())
            .with_label(
                Label::new((source_name, range))
                    .with_message(inner.reason().to_string())
                    .with_color(Color::Red),
            )
            .finish()
            .write((source_name, Source::from(src.as_str())), &mut buf);
    }

    String::from_utf8_lossy(&buf).into_owned()
}
