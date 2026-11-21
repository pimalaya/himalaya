//! # Envelope search
//!
//! The `envelope search` command, listing a mailbox through the pimdir
//! SEARCH query language (§8) and an order.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use clap::Parser;
use jiff::tz::TimeZone;
use pimalaya_cli::{clap::args::PageTokenArg, printer::Printer};

use crate::{
    account::context::Account,
    error::{ErrorCode, page_token_invalid, partial_when},
    unified::{
        capability::Capability,
        client::EmailClient,
        email::{
            envelope::EnvelopePage,
            outcome::Outcome,
            search::query::{self, SearchEmailsOrder, SearchEmailsQuery},
        },
        envelope::list::EnvelopeListOutput,
        mailbox::arg::MailboxArg,
    },
};

/// Search the envelopes of a mailbox with the pimdir query language.
///
/// The query is pimdir SEARCH §8, the language `pimdir search`, Calendula
/// and Cardamum read: `from:alice date:2026-11.. is:unread "weekly sync"`.
/// A text term is matched as the backend matches text: whole words on
/// Maildir, mbox, pimdir, Gmail and Microsoft Graph, a substring on IMAP
/// and JMAP. A date reads the `Date:` header, the sent-at, but where the
/// server compares the received date: on Gmail, and on JMAP under `or` or
/// `not`. A field the backend cannot search is refused with
/// `search-unsupported`, never dropped.
///
/// A message the listing cannot read is an entry of `errors`, the rest
/// of the page still listed; the command then exits 3.
#[derive(Debug, Parser)]
pub struct EnvelopeSearchCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    /// Page number, starting at 1.
    #[arg(long, short = 'p')]
    #[arg(value_name = "N", default_value = "1", conflicts_with = "page_token")]
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
    /// The order of the envelopes, the backend's own when omitted.
    #[arg(long, value_enum, value_name = "ORDER")]
    pub sort: Option<SearchEmailsOrder>,
    /// Query, pimdir SEARCH §8: words and `field:value` terms, `or`,
    /// `not`, `-`, `( )` and "phrases". See --help.
    ///
    /// Fields:
    ///
    /// - `subject:`, `body:`: the subject, a text body; a bare word or
    ///   phrase searches the whole message;
    ///
    /// - `from:`, `to:`, `cc:`, `bcc:`, `with:` (any of the four): a name,
    ///   an address, or `@domain`;
    ///
    /// - `is:unread`, `is:read`, `is:flagged`, `is:answered`, `is:draft`,
    ///   `tag:<keyword>`: the flags;
    ///
    /// - `has:attachment`: holding an attachment;
    ///
    /// - `date:`: a day `2026-11-14`, a month `2026-11`, a year, `today`,
    ///   `yesterday`, an instant `2026-11-14T09:30` (seconds and offset
    ///   optional), an amount back from now `30min`, `5h`, `3d`, `2w`,
    ///   `1mo`, `1y`, or a range `a..b`, either side open;
    ///
    /// - `mid:`: the message whose `Message-ID:` is the id, its angle
    ///   brackets optional.
    ///
    /// Days and instants with no offset are read in the local zone (`TZ`,
    /// then the system's). A trailing `*` makes a word a prefix.
    #[arg(value_name = "QUERY")]
    pub query: Option<Vec<String>>,
}

impl EnvelopeSearchCommand {
    /// `envelope.search` and the options its query and order use; a
    /// refused query fails here already, its error one line under `json`.
    pub fn capabilities(&self, json: bool) -> Result<Vec<Capability>> {
        let mut capabilities = vec![Capability::EnvelopeSearch];
        let query = self.search_query(json)?;
        capabilities.extend(query.options().into_iter().map(Capability::Search));
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
                .ok_or_else(|| page_token_invalid(token))?,
            None => self.page,
        };
        let page_size = self
            .page_size
            .or(Some(account.envelopes_list_page_size()))
            .filter(|p| *p > 0);
        let mailbox = self.mailbox.resolve(account);
        let query = self.search_query(printer.is_json())?;
        let text = query_text(self.query.as_deref());

        let Outcome {
            value: EnvelopePage { envelopes, more },
            errors,
        } = client.search_envelopes(
            &mailbox,
            Some(&query),
            Some(page),
            page_size,
            self.has_attachment,
        )?;

        let next_page = more.then(|| (page + 1).to_string());
        let resolution = client.resolution();

        let output = EnvelopeListOutput {
            datetime_fmt: account.datetime_fmt().to_string(),
            datetime_local_tz: account.datetime_local_tz(),
            account: resolution.account.clone(),
            backend: resolution.storage,
            mailbox,
            query: text,
            page: Some(page),
            page_size,
            recipient: self.recipient,
            envelopes,
            next_page,
            total: None,
            errors,
        };

        printer.out(&output)?;
        partial_when(&output.errors)
    }

    /// The query and the order, lowered, read against the clock and the
    /// local zone.
    fn search_query(&self, json: bool) -> Result<SearchEmailsQuery> {
        let query = match query_text(self.query.as_deref()) {
            Some(text) => Some(query::parse(&[text], json, ErrorCode::SearchInvalid)?),
            None => None,
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs() as i64);
        SearchEmailsQuery::new(query, self.sort, now, TimeZone::system())
    }
}

/// Joins the trailing positional into the query text, `None` when it is
/// empty so the search keeps its default behaviour.
fn query_text(words: Option<&[String]>) -> Option<String> {
    let joined = words?.join(" ");
    let trimmed = joined.trim();
    Some(trimmed.to_owned()).filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, Parser};

    use super::EnvelopeSearchCommand;

    fn refusal(args: &[&str], json: bool) -> String {
        let command = EnvelopeSearchCommand::parse_from([&["search"], args].concat());
        let err = command.capabilities(json).unwrap_err();
        err.to_string()
    }

    /// A refused query is drawn once over the query, its §8.4 code named.
    #[test]
    fn a_refused_query_is_drawn_over_the_query() {
        let rendered = refusal(&["from:alice", "foo:bar"], false);
        assert!(rendered.contains("unknown-field"), "{rendered}");
        assert!(!rendered.contains("Error:"), "{rendered}");
        assert!(rendered.contains("fields: "), "{rendered}");
    }

    /// Under JSON the refusal is one line with its span.
    #[test]
    fn a_refused_query_is_one_line_under_json() {
        let line = refusal(&["date:tomorrow"], true);
        assert!(!line.contains('\n'), "{line}");
        assert!(
            line.starts_with("Query refused, invalid-date at 5..13"),
            "{line}"
        );
    }

    /// A field outside mail is refused before any backend is reached.
    #[test]
    fn a_field_outside_mail_is_refused() {
        let message = refusal(&["status:completed"], false);
        assert_eq!(message, "Cannot search `status` in mail");
    }

    /// The query and the order name the options a backend declares.
    #[test]
    fn a_query_names_its_options() {
        let command = EnvelopeSearchCommand::parse_from([
            "search",
            "--sort",
            "oldest-first",
            "--",
            "from:alice",
            "date:5h",
            "-is:read",
        ]);
        let names: Vec<String> = command
            .capabilities(false)
            .unwrap()
            .into_iter()
            .map(|capability| capability.name())
            .collect();
        assert_eq!(
            names,
            [
                "envelope.search",
                "envelope.search.from",
                "envelope.search.date.instant",
                "envelope.search.is.read",
                "envelope.search.sort.oldest-first",
            ]
        );
    }

    #[test]
    fn the_help_prints_one_field_group_per_line() {
        let help = EnvelopeSearchCommand::command()
            .render_long_help()
            .to_string();
        let bullets = help
            .lines()
            .filter(|line| line.trim_start().starts_with("- `"))
            .count();

        assert_eq!(bullets, 6, "{help}");
    }
}
