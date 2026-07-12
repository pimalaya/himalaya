//! # Search emails filter query
//!
//! Exposes [`SearchEmailsFilterQuery`], the recursive AST produced by
//! [`super::parser::query`].

use chrono::NaiveDate;

use crate::email::flag::Flag;

/// The search emails filter query.
///
/// Composed of 3 operators (and, or, not) and 7 conditions (date,
/// after date, from, to, subject, body, flag). All date-related
/// conditions are anchored to the `Date:` header (sent-at), never to
/// the server-side received-at timestamp.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchEmailsFilterQuery {
    /// Filter emails that match both given conditions.
    And(Box<SearchEmailsFilterQuery>, Box<SearchEmailsFilterQuery>),

    /// Filter emails that match one of the two given conditions.
    Or(Box<SearchEmailsFilterQuery>, Box<SearchEmailsFilterQuery>),

    /// Filter emails that do not match the given condition.
    Not(Box<SearchEmailsFilterQuery>),

    /// Filter emails where the `Date:` header of the message matches
    /// the given date. Only the year, month and day are considered.
    Date(NaiveDate),

    /// Filter emails where the `Date:` header of the message is
    /// strictly greater than the given date. Only the year, month and
    /// day are considered.
    AfterDate(NaiveDate),

    /// Filter emails where the `From:` header contains the pattern.
    From(String),

    /// Filter emails where the `To:` header contains the pattern.
    To(String),

    /// Filter emails where the `Subject:` header contains the pattern.
    Subject(String),

    /// Filter emails where one of the text bodies contains the pattern.
    Body(String),

    /// Filter emails where the given flag is set on the envelope.
    Flag(Flag),
}
