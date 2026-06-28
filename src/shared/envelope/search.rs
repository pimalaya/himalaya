use std::io::{IsTerminal, stdout};

use anyhow::{Result, bail};
use ariadne::{Color, Config, Label, Report, ReportKind, Source};
use clap::Parser;
use io_email::search::{error::Error as SearchQueryError, query::SearchEmailsQuery};
use pimalaya_cli::printer::Printer;

use crate::account::context::Account;
use crate::shared::{
    client::EmailClient,
    envelope::list::{EnvelopeColors, Envelopes, FlagChars},
    mailbox::arg::MailboxArg,
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
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let page = Some(self.page).filter(|p| *p > 0);
        let page_size = self
            .page_size
            .or(Some(account.envelopes_list_page_size()))
            .filter(|p| *p > 0);
        let mailbox = self.mailbox.resolve(account)?;
        let query = parse_query(self.query.as_deref())?;

        let envelopes = client.search_envelopes(
            &mailbox,
            query.as_ref(),
            page,
            page_size,
            self.has_attachment,
        )?;

        let envelopes = Envelopes {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            max_width: self.max_width,
            datetime_fmt: account.datetime_fmt().to_string(),
            datetime_local_tz: account.datetime_local_tz(),
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

        printer.out(envelopes)
    }
}

/// Joins the trailing-positional words and feeds them to
/// [`SearchEmailsQuery::from_str`]. Returns `Ok(None)` when the input
/// is empty (no query) so `client.search_envelopes` keeps its default
/// behaviour, or bails with the ariadne-rendered parse error.
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
        Err(err) => bail!(render_query_parse_error(&err)),
    }
}

/// Pretty-prints a `chumsky` parse error with ariadne, one labelled
/// report per inner error, into a single returned string so the caller
/// can surface it through the normal error channel (stdout). Color is
/// disabled when stdout is not a terminal.
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
