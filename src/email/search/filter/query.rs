//! # Search filter query
//!
//! Exposes [`SearchEmailsFilterQuery`], the recursive AST
//! [`super::parser::query`] produces.

use chrono::NaiveDate;

use crate::email::flag::Flag;

/// The filter half of a search query: three operators over seven
/// conditions.
///
/// Both date conditions read the `Date:` header, the sent-at, rather than
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
    /// Emails whose `From:` header contains the pattern.
    From(String),
    /// Emails whose `To:` header contains the pattern.
    To(String),
    /// Emails whose `Subject:` header contains the pattern.
    Subject(String),
    /// Emails one of whose text bodies contains the pattern.
    Body(String),
    /// Emails carrying the flag.
    Flag(Flag),
}
