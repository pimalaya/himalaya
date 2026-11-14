//! # Envelope search
//!
//! The `envelope search` command, listing a mailbox through the shared
//! filter and sort query.

use std::ops::Range;

use anyhow::{Context, Result};
use ariadne::{Config, IndexType, Label, Report, ReportKind, Source};
use chumsky::error::{Rich, RichPattern, RichReason};
use clap::Parser;
use pimalaya_cli::{clap::args::PageTokenArg, error::CodedError, printer::Printer};

use crate::{
    account::context::Account,
    capability::Capability,
    email::search::{error::Error as SearchQueryError, query::SearchEmailsQuery},
    error::ErrorCode,
    shared::{client::EmailClient, envelope::list::EnvelopeListOutput, mailbox::arg::MailboxArg},
};

/// Search the envelopes of a mailbox with the shared query language.
///
/// A date clause reads the `Date:` header, the sent-at. A text clause is
/// matched as the backend matches text: a case-insensitive substring on
/// Maildir, mbox and pimdir, as the server matches it on IMAP (RFC
/// 3501 asks for a substring, some servers match whole words), whole words
/// over the whole message on Gmail, KQL terms on Microsoft Graph, and as the
/// server defines it on JMAP.
/// A clause the backend cannot search is refused with `search-unsupported`,
/// never dropped: every flag and header on Microsoft Graph, `flag answered` and keywords
/// on Gmail, `attachment` on IMAP.
#[derive(Debug, Parser)]
pub struct EnvelopeSearchCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    /// Page number, starting at 1.
    #[arg(long, short = 'p')]
    #[arg(value_name = "N", default_value = "1")]
    #[arg(value_parser = clap::value_parser!(u32).range(1..))]
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
    /// Render recipients instead of senders.
    #[arg(long, short)]
    pub recipient: bool,
    /// Tell which envelopes carry an attachment, which on some backends
    /// costs one extra lookup per envelope.
    #[arg(long = "has-attachment")]
    pub has_attachment: bool,
    /// Query: date, after, before, from, to, cc, bcc, address, subject, body,
    /// flag, attachment, message-id; and, or, not, ( ); order by. See --help.
    ///
    /// Clauses:
    ///
    /// - `date <yyyy-mm-dd>`: sent on that day;
    ///
    /// - `after <yyyy-mm-dd>`, `before <yyyy-mm-dd>`: sent strictly after,
    ///   strictly before that day;
    ///
    /// - `from <pattern>`, `to <pattern>`, `cc <pattern>`, `bcc <pattern>`:
    ///   that header holds the pattern, in a name or an address;
    ///
    /// - `address <pattern>`: any of those four headers holds it;
    ///
    /// - `subject <pattern>`, `body <pattern>`: the subject, a text body;
    ///
    /// - `flag <name>`: carrying the flag, `seen`, `answered`, `flagged`,
    ///   `draft` or a keyword;
    ///
    /// - `attachment`: holding an attachment;
    ///
    /// - `message-id <id>`: the message whose `Message-ID:` is the id, its
    ///   angle brackets optional.
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
    pub query: Option<Vec<String>>,
}

impl EnvelopeSearchCommand {
    /// What the search reaches: `envelope.search` and the options its
    /// query uses, a query that does not parse failing here already.
    pub fn capabilities(&self) -> Result<Vec<Capability>> {
        let mut capabilities = vec![Capability::EnvelopeSearch];

        if let Some(query) = parse_query(query_text(self.query.as_deref()).as_deref())? {
            capabilities.extend(query.options().into_iter().map(Capability::Search));
        }

        Ok(capabilities)
    }

    /// Searches the mailbox and prints one page of hits.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let page = match &self.page_token.token {
            Some(token) => token
                .parse()
                .ok()
                .filter(|page: &u32| *page > 0)
                .with_context(|| format!("Invalid page token {token}"))?,
            None => self.page,
        };
        let page = Some(page);
        let page_size = self
            .page_size
            .or(Some(account.envelopes_list_page_size()))
            .filter(|p| *p > 0);
        let mailbox = self.mailbox.resolve(account);
        let text = query_text(self.query.as_deref());
        let query = parse_query(text.as_deref())?;

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

        let resolution = client.resolution();

        printer.out(EnvelopeListOutput {
            datetime_fmt: account.datetime_fmt().to_string(),
            datetime_local_tz: account.datetime_local_tz(),
            account: resolution.account.clone(),
            backend: resolution.storage,
            mailbox,
            query: text,
            page,
            page_size,
            recipient: self.recipient,
            envelopes,
            next_page,
        })
    }
}

/// Joins the trailing positional into the query text, `None` when it is
/// empty so the search keeps its default behaviour.
fn query_text(words: Option<&[String]>) -> Option<String> {
    let joined = words?.join(" ");
    let trimmed = joined.trim();
    Some(trimmed.to_owned()).filter(|text| !text.is_empty())
}

/// Parses the query text, `None` standing for no query.
fn parse_query(text: Option<&str>) -> Result<Option<SearchEmailsQuery>> {
    let Some(text) = text else {
        return Ok(None);
    };

    match text.parse::<SearchEmailsQuery>() {
        Ok(query) => Ok(Some(query)),
        Err(err) => {
            Err(CodedError::new(ErrorCode::SearchInvalid, render_query_parse_error(&err)).into())
        }
    }
}

/// Renders a parse error with ariadne, one label per inner error, into
/// the string the caller raises through stdout.
///
/// Rendered without colour, the string landing in a JSON error as often
/// as on a terminal. The report's own `Error:` head is dropped, the CLI
/// printing its own.
fn render_query_parse_error(err: &SearchQueryError) -> String {
    let SearchQueryError::ParseError(errs, src) = err;
    let source_name = "query";
    let config = Config::default()
        .with_color(false)
        .with_index_type(IndexType::Byte);
    let span = errs
        .first()
        .map(|inner| inner.span().into_range())
        .unwrap_or(0..0);
    let mut report = Report::build(ReportKind::Error, (source_name, span))
        .with_config(config)
        .with_message(err.to_string());

    for inner in errs {
        let range = word_range(src, inner.span().into_range());
        let label = Label::new((source_name, range)).with_message(describe(inner, src));
        report = report.with_label(label);
    }

    let mut buf = Vec::new();
    let _ = report
        .finish()
        .write((source_name, Source::from(src.as_str())), &mut buf);
    let rendered = String::from_utf8_lossy(&buf);

    match rendered.strip_prefix("Error: ") {
        Some(rendered) => rendered.to_owned(),
        None => rendered.into_owned(),
    }
}

/// Widens an error span to the whole word it starts, up to the next space
/// or parenthesis, chumsky merging the errors raised at one place over
/// the span of the first.
fn word_range(src: &str, range: Range<usize>) -> Range<usize> {
    let delimiter = |c: char| matches!(c, ' ' | '(' | ')');

    match src.get(range.start..) {
        Some(rest) if rest.starts_with(|c| !delimiter(c)) => {
            let len = rest.find(delimiter).unwrap_or(rest.len());
            range.start..(range.start + len).max(range.end)
        }
        _ => range,
    }
}

/// Says what the parser found and what it expected there, the found
/// part quoted from the query over the error's whole span.
fn describe(err: &Rich<'_, char>, src: &str) -> String {
    let RichReason::ExpectedFound { expected, .. } = err.reason() else {
        return err.reason().to_string();
    };

    let found = match src.get(word_range(src, err.span().into_range())) {
        Some(found) if !found.is_empty() => format!("`{found}`"),
        _ => String::from("the end of the query"),
    };

    let mut expected: Vec<String> = expected
        .iter()
        .map(|pattern| match pattern {
            RichPattern::Token(token) => format!("`{}`", **token),
            RichPattern::Label(label) => label.to_string(),
            RichPattern::Identifier(word) => format!("`{word}`"),
            RichPattern::EndOfInput => String::from("the end of the query"),
            _ => String::from("something else"),
        })
        .collect();
    expected.dedup();

    match expected.split_last() {
        None => format!("unexpected {found}"),
        Some((last, [])) => format!("found {found}, expected {last}"),
        Some((last, rest)) => format!("found {found}, expected {} or {last}", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use crate::email::search::query::SearchEmailsQuery;

    use clap::CommandFactory;

    use super::{EnvelopeSearchCommand, render_query_parse_error};

    fn render(query: &str) -> String {
        let err = query.parse::<SearchEmailsQuery>().unwrap_err();
        render_query_parse_error(&err)
    }

    #[test]
    fn a_refused_query_is_reported_once_over_the_whole_query() {
        let rendered = render("from alice order by size");

        assert!(
            rendered.starts_with("Cannot parse search query `from alice order by size`\n"),
            "{rendered}",
        );
        assert!(!rendered.contains("Error:"), "{rendered}");
        assert!(rendered.contains("query:1:21"), "{rendered}");
        assert!(
            rendered.contains("found `size`, expected `date`, `from`, `to` or `subject`"),
            "{rendered}",
        );
    }

    #[test]
    fn a_misspelt_word_is_quoted_whole_and_no_keyword_prefix_leaks() {
        let rendered = render("and");

        assert!(rendered.contains("found `and`, expected"), "{rendered}");
        assert!(!rendered.contains("'d'"), "{rendered}");
        assert!(!rendered.contains("'t'"), "{rendered}");
    }

    #[test]
    fn a_date_clause_expects_a_date() {
        let rendered = render("after foo");

        assert!(
            rendered.contains("found `foo`, expected date after `after`"),
            "{rendered}",
        );
    }

    #[test]
    fn the_help_prints_one_clause_per_line() {
        let help = EnvelopeSearchCommand::command()
            .render_long_help()
            .to_string();
        let bullets = help
            .lines()
            .filter(|line| line.trim_start().starts_with("- `"))
            .count();

        assert_eq!(bullets, 8, "{help}");
    }

    #[test]
    fn an_empty_sort_expects_a_key() {
        let rendered = render("from alice order by");

        assert!(
            rendered.contains("found the end of the query, expected sort key after `order by`"),
            "{rendered}",
        );
    }
}
