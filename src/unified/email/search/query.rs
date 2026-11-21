//! # Search query
//!
//! Exposes [`SearchEmailsQuery`]: a pimdir SEARCH §8 query and its order,
//! the query lowered to the filter the server translations read, the
//! order to the sort chain the local sorts read.

use anyhow::Result;
use ariadne::{Config, Label, Report, ReportKind, Source};
use chrono::{DateTime, FixedOffset, NaiveDate};
use io_pimdir::search::query::{
    PimdirQuery, PimdirQueryDate, PimdirQueryDateRange, PimdirQueryError, PimdirQueryErrorCode,
    PimdirQueryPredicate, PimdirQueryTextField,
};
use jiff::{Timestamp, tz::TimeZone};
use pimalaya_cli::error::CodedError;

use crate::{
    error::ErrorCode,
    unified::{
        capability::SearchOption,
        email::{
            flag::Flag,
            search::{
                filter::query::SearchEmailsFilterQuery,
                sort::query::{
                    SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder,
                },
            },
        },
    },
};

/// The orders an envelope search answers (pimdir SEARCH §8.2).
#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum SearchEmailsOrder {
    /// The `Date:` header, the newest first.
    NewestFirst,
    /// The `Date:` header, the oldest first.
    OldestFirst,
    /// The subject, A to Z.
    Title,
    /// The sender's address, A to Z.
    Sender,
    /// The best text match first, where the backend ranks.
    Relevance,
}

impl SearchEmailsOrder {
    /// The order's name in pimdir SEARCH §8.2.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NewestFirst => "newest-first",
            Self::OldestFirst => "oldest-first",
            Self::Title => "title",
            Self::Sender => "sender",
            Self::Relevance => "relevance",
        }
    }

    /// The option of `envelope.search` asking the order.
    pub fn option(self) -> SearchOption {
        match self {
            Self::NewestFirst => SearchOption::SortNewestFirst,
            Self::OldestFirst => SearchOption::SortOldestFirst,
            Self::Title => SearchOption::SortTitle,
            Self::Sender => SearchOption::SortSender,
            Self::Relevance => SearchOption::SortRelevance,
        }
    }
}

/// A search query: which envelopes, and in which order.
#[derive(Clone, Debug)]
pub struct SearchEmailsQuery {
    /// The query as parsed, `None` for every envelope.
    pub query: Option<PimdirQuery>,
    /// The filter the server translations read, lowered from `query`.
    pub filter: Option<SearchEmailsFilterQuery>,
    /// The order asked, `None` for the backend's own.
    pub order: Option<SearchEmailsOrder>,
    /// The sort chain the local sorts read, lowered from `order`.
    pub sort: Option<Vec<SearchEmailsSorter>>,
    /// The instant relative dates count back from, Unix seconds.
    pub now: i64,
    /// The zone a date with no offset is read in.
    pub zone: TimeZone,
}

impl SearchEmailsQuery {
    /// Lowers a parsed query and an order, refusing a field no mail backend
    /// serves (`search-unsupported`).
    pub fn new(
        query: Option<PimdirQuery>,
        order: Option<SearchEmailsOrder>,
        now: i64,
        zone: TimeZone,
    ) -> Result<Self> {
        let filter = match &query {
            Some(query) => {
                for (term, _) in query.terms() {
                    if SearchOption::of(&term.predicate).is_none() {
                        let name = term.predicate.name();
                        let message = format!("Cannot search `{name}` in mail");
                        return Err(CodedError::new(ErrorCode::SearchUnsupported, message).into());
                    }
                }
                Some(lower(query, now, &zone)?)
            }
            None => None,
        };
        let sort = order.and_then(|order| {
            let (kind, order) = match order {
                SearchEmailsOrder::NewestFirst => (
                    SearchEmailsSorterKind::Date,
                    SearchEmailsSorterOrder::Descending,
                ),
                SearchEmailsOrder::OldestFirst => (
                    SearchEmailsSorterKind::Date,
                    SearchEmailsSorterOrder::Ascending,
                ),
                SearchEmailsOrder::Title => (
                    SearchEmailsSorterKind::Subject,
                    SearchEmailsSorterOrder::Ascending,
                ),
                SearchEmailsOrder::Sender => (
                    SearchEmailsSorterKind::From,
                    SearchEmailsSorterOrder::Ascending,
                ),
                SearchEmailsOrder::Relevance => return None,
            };
            Some(vec![SearchEmailsSorter(kind, order)])
        });

        Ok(Self {
            query,
            filter,
            order,
            sort,
            now,
            zone,
        })
    }

    /// The fields and the order the query uses, each once, as the options
    /// of `envelope.search` a backend declares.
    pub fn options(&self) -> Vec<SearchOption> {
        let mut options = Vec::new();
        for (term, _) in self.query.iter().flat_map(PimdirQuery::terms) {
            if let Some(option) = SearchOption::of(&term.predicate)
                && !options.contains(&option)
            {
                options.push(option);
            }
        }
        if let Some(order) = self.order {
            options.push(order.option());
        }
        options
    }
}

/// Parses the words of a query, joined by spaces, as the pimdir SEARCH
/// query language (§8.3).
///
/// A refused query (§8.4) fails with `code`, drawn over the query in plain
/// output; in JSON its message names the §8.4 code, the span in Unicode
/// scalar values and what was expected there.
pub fn parse(words: &[String], json: bool, code: ErrorCode) -> Result<PimdirQuery> {
    let query = words.join(" ");
    PimdirQuery::parse(&query).map_err(|err| {
        let message = match json {
            true => refusal_line(&err),
            false => refusal_report(&query, &err),
        };
        CodedError::new(code, message).into()
    })
}

/// A refusal as one line: its code, span and message, then what was
/// expected.
fn refusal_line(err: &PimdirQueryError) -> String {
    let mut line = format!(
        "Query refused, {} at {}..{}: {}",
        err.code.as_str(),
        err.start,
        err.end,
        err.message
    );
    if !err.expected.is_empty() {
        line.push_str(&format!("; expected {}", err.expected.join(", ")));
    }
    line
}

/// A refusal drawn over the query with ariadne, without colour, the string
/// landing in a JSON error as often as on a terminal.
fn refusal_report(query: &str, err: &PimdirQueryError) -> String {
    let name = "query";
    let config = Config::default().with_color(false);
    let end = err.end.max(err.start + 1);
    let mut report = Report::build(ReportKind::Error, (name, err.start..end))
        .with_config(config)
        .with_code(err.code.as_str())
        .with_message("Query refused")
        .with_label(Label::new((name, err.start..end)).with_message(&err.message));
    if !err.expected.is_empty() {
        report = report.with_note(format!("expected {}", err.expected.join(", ")));
    }
    if err.code == PimdirQueryErrorCode::UnknownField {
        report = report.with_help(format!("fields: {}", err.fields.join(", ")));
    }

    let mut buf = Vec::new();
    let _ = report.finish().write((name, Source::from(query)), &mut buf);

    String::from_utf8_lossy(&buf).replacen("Error: ", "", 1)
}

/// The filter a query lowers to, its every field one [`SearchOption`]
/// names.
fn lower(query: &PimdirQuery, now: i64, zone: &TimeZone) -> Result<SearchEmailsFilterQuery> {
    use SearchEmailsFilterQuery as Q;

    let fold = |operands: &[PimdirQuery], join: fn(Box<Q>, Box<Q>) -> Q| -> Result<Q> {
        let mut lowered = operands.iter().map(|operand| lower(operand, now, zone));
        let first = lowered
            .next()
            .expect("an operator joins two operands or more")?;
        lowered.try_fold(first, |left, right| {
            Ok(join(Box::new(left), Box::new(right?)))
        })
    };

    let lowered = match query {
        PimdirQuery::And(operands) => fold(operands, Q::And)?,
        PimdirQuery::Or(operands) => fold(operands, Q::Or)?,
        PimdirQuery::Not(operand) => Q::Not(Box::new(lower(operand, now, zone)?)),
        PimdirQuery::Group(operand) => lower(operand, now, zone)?,
        PimdirQuery::Term(term) => match &term.predicate {
            PimdirQueryPredicate::Text { field, text } => match field {
                PimdirQueryTextField::Title => Q::Subject(text.text.clone()),
                PimdirQueryTextField::Body => Q::Body(text.text.clone()),
                _ => Q::Text(text.text.clone()),
            },
            PimdirQueryPredicate::Address { role, text } => {
                let text = text.text.clone();
                match role.map(|role| role.as_str()) {
                    Some("from") => Q::From(text),
                    Some("to") => Q::To(text),
                    Some("cc") => Q::Cc(text),
                    Some("bcc") => Q::Bcc(text),
                    _ => Q::Address(text),
                }
            }
            PimdirQueryPredicate::Flag(flag) => Q::Flag(Flag::from_raw(flag.as_str())),
            PimdirQueryPredicate::HasAttachment => Q::Attachment,
            PimdirQueryPredicate::Mid(id) => Q::MessageId(id.clone()),
            PimdirQueryPredicate::Date(range) => dates(range, now, zone)?,
            predicate => {
                let message = format!("Cannot search `{}` in mail", predicate.name());
                return Err(CodedError::new(ErrorCode::SearchUnsupported, message).into());
            }
        },
    };

    Ok(lowered)
}

/// A date range as the clauses bounding it: a side a calendar point
/// names as a day, the others as an instant.
fn dates(
    range: &PimdirQueryDateRange,
    now: i64,
    zone: &TimeZone,
) -> Result<SearchEmailsFilterQuery> {
    use SearchEmailsFilterQuery as Q;

    let (since, until) = range.bounds(now, zone);
    let calendar = |point: &Option<PimdirQueryDate>| {
        matches!(
            point,
            Some(
                PimdirQueryDate::Year(_)
                    | PimdirQueryDate::Month { .. }
                    | PimdirQueryDate::Day { .. }
                    | PimdirQueryDate::Today
                    | PimdirQueryDate::Yesterday
            )
        )
    };
    let day = |seconds: i64| -> Option<NaiveDate> {
        let Ok(instant) = Timestamp::from_second(seconds) else {
            return DateTime::from_timestamp(seconds, 0).map(|utc| utc.date_naive());
        };
        let date = instant.to_zoned_datetime(zone.clone()).date();
        NaiveDate::from_ymd_opt(
            i32::from(date.year()),
            date.month() as u32,
            date.day() as u32,
        )
    };
    let instant = |seconds: i64| -> Option<DateTime<FixedOffset>> {
        DateTime::from_timestamp(seconds, 0).map(|instant| instant.fixed_offset())
    };

    let first = since.filter(|_| calendar(&range.start)).and_then(day);
    let after = until.filter(|_| calendar(&range.end)).and_then(day);
    if let (Some(first), Some(after)) = (first, after)
        && first.succ_opt() == Some(after)
    {
        return Ok(Q::Date(first));
    }

    let since = since.and_then(|since| match calendar(&range.start) {
        true => day(since).and_then(|day| day.pred_opt()).map(Q::AfterDate),
        false => instant(since - 1).map(Q::AfterInstant),
    });
    let until = until.and_then(|until| match calendar(&range.end) {
        true => day(until).map(Q::BeforeDate),
        false => instant(until).map(Q::BeforeInstant),
    });

    match (since, until) {
        (Some(since), Some(until)) => Ok(Q::And(Box::new(since), Box::new(until))),
        (Some(since), None) => Ok(since),
        (None, Some(until)) => Ok(until),
        (None, None) => {
            let message = "A date range open on both sides bounds no message: drop the term";
            Err(CodedError::new(ErrorCode::SearchInvalid, message).into())
        }
    }
}

#[cfg(any(feature = "maildir", feature = "mbox"))]
impl SearchEmailsQuery {
    /// Whether a message, its raw bytes and its flags, is a hit of the
    /// query, as an index holding it answers (io-pimdir's `search::mail`).
    pub fn matches(&self, raw: &[u8], flags: &std::collections::BTreeSet<Flag>) -> bool {
        let Some(query) = &self.query else {
            return true;
        };
        let flags = flags
            .iter()
            .map(|flag| flag.canonical().to_owned())
            .collect();
        let mail = io_pimdir::search::mail::PimdirMail::read(raw, flags);
        query
            .match_mails(&[mail], self.now, &self.zone)
            .is_ok_and(|hits| hits == [true])
    }
}

/// Reads a query for a test: pimdir SEARCH §8, then an optional
/// ` sort:<order>`, at 2026-11-21T12:00:00Z in UTC.
#[cfg(test)]
impl std::str::FromStr for SearchEmailsQuery {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self> {
        use clap::ValueEnum;

        let (text, order) = match text
            .rsplit_once(" sort:")
            .or(text.strip_prefix("sort:").map(|o| ("", o)))
        {
            Some((text, order)) => (
                text,
                Some(SearchEmailsOrder::from_str(order, false).map_err(anyhow::Error::msg)?),
            ),
            None => (text, None),
        };
        let query = match text.trim().is_empty() {
            true => None,
            false => Some(PimdirQuery::parse(text).map_err(|err| anyhow::anyhow!("{err}"))?),
        };
        Self::new(query, order, 1_795_262_400, TimeZone::UTC)
    }
}
