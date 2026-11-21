//! # Search filter query
//!
//! Exposes [`SearchEmailsFilterQuery`], the tree a pimdir SEARCH §8 query
//! lowers to ([`super::super::query::SearchEmailsQuery::new`]).

use std::fmt;

use chrono::{DateTime, FixedOffset, NaiveDate};

use crate::unified::email::flag::Flag;

/// The filter half of a search query: three operators over the conditions
/// a mail server searches.
///
/// The date conditions read the `Date:` header, the sent-at, rather than
/// the server's received-at.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SearchEmailsFilterQuery {
    /// Emails matching both conditions.
    And(Box<SearchEmailsFilterQuery>, Box<SearchEmailsFilterQuery>),
    /// Emails matching either condition.
    Or(Box<SearchEmailsFilterQuery>, Box<SearchEmailsFilterQuery>),
    /// Emails matching the condition not at all.
    Not(Box<SearchEmailsFilterQuery>),
    /// Emails sent on that day, the time of day being ignored.
    Date(NaiveDate),
    /// Emails sent strictly after that day.
    AfterDate(NaiveDate),
    /// Emails sent strictly before that day.
    BeforeDate(NaiveDate),
    /// Emails sent strictly after that instant, to the second.
    AfterInstant(DateTime<FixedOffset>),
    /// Emails sent strictly before that instant, to the second.
    BeforeInstant(DateTime<FixedOffset>),
    /// Emails holding the text anywhere: a header or a text body.
    Text(String),
    /// Emails whose `From:` header holds the text.
    From(String),
    /// Emails whose `To:` header holds the text.
    To(String),
    /// Emails whose `Cc:` header holds the text.
    Cc(String),
    /// Emails whose `Bcc:` header holds the text.
    Bcc(String),
    /// Emails whose `From:`, `To:`, `Cc:` or `Bcc:` header holds the text.
    Address(String),
    /// Emails whose `Subject:` header holds the text.
    Subject(String),
    /// Emails one of whose text bodies holds the text.
    Body(String),
    /// Emails carrying the flag.
    Flag(Flag),
    /// Emails holding at least one attachment.
    Attachment,
    /// The email whose `Message-ID:` is the id, held bare, without its
    /// angle brackets.
    MessageId(String),
}

/// Renders the filter in the pimdir SEARCH language, so an error can quote
/// the clause it is about as the user would write it.
impl fmt::Display for SearchEmailsFilterQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use SearchEmailsFilterQuery as Q;

        match self {
            Q::And(left, right) => write!(f, "({left} {right})"),
            Q::Or(left, right) => write!(f, "({left} or {right})"),
            Q::Not(inner) => write!(f, "not {inner}"),
            Q::Date(date) => write!(f, "date:{date}"),
            Q::AfterDate(date) => write!(f, "date:{}..", date.succ_opt().unwrap_or(*date)),
            Q::AfterInstant(instant) => write!(f, "date:{}..", instant.to_rfc3339()),
            Q::BeforeDate(date) => write!(f, "date:..{}", date.pred_opt().unwrap_or(*date)),
            Q::BeforeInstant(instant) => write!(f, "date:..{}", instant.to_rfc3339()),
            Q::Text(text) => write!(f, "{}", Text(text)),
            Q::From(text) => write!(f, "from:{}", Text(text)),
            Q::To(text) => write!(f, "to:{}", Text(text)),
            Q::Cc(text) => write!(f, "cc:{}", Text(text)),
            Q::Bcc(text) => write!(f, "bcc:{}", Text(text)),
            Q::Address(text) => write!(f, "with:{}", Text(text)),
            Q::Subject(text) => write!(f, "subject:{}", Text(text)),
            Q::Body(text) => write!(f, "body:{}", Text(text)),
            Q::Flag(flag) => match flag.raw() {
                "\\Seen" => write!(f, "is:read"),
                "\\Flagged" => write!(f, "is:flagged"),
                "\\Answered" => write!(f, "is:answered"),
                "\\Draft" => write!(f, "is:draft"),
                raw => write!(f, "tag:{raw}"),
            },
            Q::Attachment => write!(f, "has:attachment"),
            Q::MessageId(id) => write!(f, "mid:{}", Text(id)),
        }
    }
}

/// A text as the language reads it back: quoted when it holds a space, a
/// parenthesis or a double quote.
struct Text<'a>(&'a str);

impl fmt::Display for Text<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.contains([' ', '(', ')', '"', ':']) {
            true => write!(f, "\"{}\"", self.0.replace('"', " ")),
            false => write!(f, "{}", self.0),
        }
    }
}
