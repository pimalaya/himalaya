//! # Search filter query
//!
//! Exposes [`SearchEmailsFilterQuery`], the recursive AST
//! [`super::parser::query`] produces.

use std::fmt;

use chrono::NaiveDate;

use crate::email::flag::Flag;

/// The filter half of a search query: three operators over eleven
/// conditions.
///
/// The date conditions read the `Date:` header, the sent-at, rather than
/// the server's received-at.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
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
    /// Emails whose `From:` header contains the pattern.
    From(String),
    /// Emails whose `To:` header contains the pattern.
    To(String),
    /// Emails whose `Cc:` header contains the pattern.
    Cc(String),
    /// Emails whose `Bcc:` header contains the pattern.
    Bcc(String),
    /// Emails whose `From:`, `To:`, `Cc:` or `Bcc:` header contains the
    /// pattern.
    Address(String),
    /// Emails whose `Subject:` header contains the pattern.
    Subject(String),
    /// Emails one of whose text bodies contains the pattern.
    Body(String),
    /// Emails carrying the flag.
    Flag(Flag),
    /// Emails holding at least one attachment.
    Attachment,
}

/// Renders the filter in the query language, so an error can quote the
/// clause it is about as the user would write it.
impl fmt::Display for SearchEmailsFilterQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use SearchEmailsFilterQuery as Q;

        match self {
            Q::And(left, right) => write!(f, "({left} and {right})"),
            Q::Or(left, right) => write!(f, "({left} or {right})"),
            Q::Not(inner) => write!(f, "not {inner}"),
            Q::Date(date) => write!(f, "date {date}"),
            Q::AfterDate(date) => write!(f, "after {date}"),
            Q::BeforeDate(date) => write!(f, "before {date}"),
            Q::From(pattern) => write!(f, "from {}", Pattern(pattern)),
            Q::To(pattern) => write!(f, "to {}", Pattern(pattern)),
            Q::Cc(pattern) => write!(f, "cc {}", Pattern(pattern)),
            Q::Bcc(pattern) => write!(f, "bcc {}", Pattern(pattern)),
            Q::Address(pattern) => write!(f, "address {}", Pattern(pattern)),
            Q::Subject(pattern) => write!(f, "subject {}", Pattern(pattern)),
            Q::Body(pattern) => write!(f, "body {}", Pattern(pattern)),
            Q::Flag(flag) => write!(f, "flag {}", flag.raw()),
            Q::Attachment => write!(f, "attachment"),
        }
    }
}

/// A pattern as the grammar reads it back: quoted when it holds a space,
/// a parenthesis or a double quote.
struct Pattern<'a>(&'a str);

impl fmt::Display for Pattern<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.contains([' ', '(', ')', '"', '\\']) {
            let escaped = self.0.replace('\\', "\\\\").replace('"', "\\\"");
            write!(f, "\"{escaped}\"")
        } else {
            write!(f, "{}", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use chumsky::Parser;

    use crate::email::search::filter::{parser, query::SearchEmailsFilterQuery};

    #[test]
    fn a_filter_renders_back_into_the_language_it_parses_from() {
        let queries = [
            "from alice",
            "not subject \"weekly report\"",
            "(cc bob or attachment)",
            "(before 2026-01-01 and address \"say \\\"hi\\\"\")",
        ];

        for query in queries {
            let parsed: SearchEmailsFilterQuery =
                parser::query().parse(query).into_result().unwrap();
            let rendered = parsed.to_string();
            let reparsed: SearchEmailsFilterQuery = parser::query()
                .parse(rendered.as_str())
                .into_result()
                .unwrap();

            assert_eq!(parsed, reparsed, "{query} rendered as {rendered}");
        }
    }
}
