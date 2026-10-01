//! # Microsoft Graph search
//!
//! Translation of the shared search filter into a KQL `$search` value,
//! the one Graph query that matches sender, recipient, subject and body by
//! substring, as the other backends do. Graph refuses `$search` beside
//! `$filter`, and KQL carries no flag property, so a flag clause is refused
//! rather than dropped.
//!
//! <https://learn.microsoft.com/en-us/graph/search-query-parameter>

use anyhow::{Result, bail};
use chrono::NaiveDate;

use crate::email::search::filter::query::SearchEmailsFilterQuery;

/// Translates the filter half of a shared query into the quoted `$search`
/// value Graph takes.
pub(super) fn filter_to_search(filter: &SearchEmailsFilterQuery) -> Result<String> {
    Ok(format!("\"{}\"", filter_to_kql(filter)?))
}

/// Translates a filter into KQL, every combination parenthesized.
fn filter_to_kql(filter: &SearchEmailsFilterQuery) -> Result<String> {
    Ok(match filter {
        SearchEmailsFilterQuery::And(lhs, rhs) => {
            format!("({}) AND ({})", filter_to_kql(lhs)?, filter_to_kql(rhs)?)
        }
        SearchEmailsFilterQuery::Or(lhs, rhs) => {
            format!("({}) OR ({})", filter_to_kql(lhs)?, filter_to_kql(rhs)?)
        }
        SearchEmailsFilterQuery::Not(inner) => format!("NOT ({})", filter_to_kql(inner)?),
        // NOTE: the shared date clauses read the `Date:` header, which is
        // what Graph indexes as `sent`.
        SearchEmailsFilterQuery::Date(date) => format!("sent:{}", date_kql(*date)),
        SearchEmailsFilterQuery::AfterDate(date) => format!("sent>{}", date_kql(*date)),
        SearchEmailsFilterQuery::From(pattern) => format!("from:{}", value_kql(pattern)),
        SearchEmailsFilterQuery::To(pattern) => format!("to:{}", value_kql(pattern)),
        SearchEmailsFilterQuery::Subject(pattern) => format!("subject:{}", value_kql(pattern)),
        SearchEmailsFilterQuery::Body(pattern) => format!("body:{}", value_kql(pattern)),
        SearchEmailsFilterQuery::Flag(flag) => bail!(
            "Microsoft Graph search cannot filter on the {} flag; use `himalaya msgraph messages list --filter`",
            flag.raw()
        ),
    })
}

/// Renders a pattern as a KQL value: several words group in parentheses,
/// a double quote having no way through Graph's own quoting.
fn value_kql(pattern: &str) -> String {
    let pattern = pattern.replace('"', "");
    if pattern.contains(char::is_whitespace) {
        format!("({pattern})")
    } else {
        pattern
    }
}

/// Renders a date the way KQL reads it.
fn date_kql(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::email::flag::{Flag, IanaFlag};

    #[test]
    fn a_shared_filter_becomes_a_kql_search() {
        let filter = SearchEmailsFilterQuery::And(
            Box::new(SearchEmailsFilterQuery::From("alice".into())),
            Box::new(SearchEmailsFilterQuery::Not(Box::new(
                SearchEmailsFilterQuery::Subject("weekly report".into()),
            ))),
        );

        assert_eq!(
            filter_to_search(&filter).unwrap(),
            "\"(from:alice) AND (NOT (subject:(weekly report)))\""
        );
    }

    #[test]
    fn dates_read_the_sent_date() {
        let day = NaiveDate::from_ymd_opt(2026, 8, 14).unwrap();

        assert_eq!(
            filter_to_kql(&SearchEmailsFilterQuery::Date(day)).unwrap(),
            "sent:2026-08-14"
        );
        assert_eq!(
            filter_to_kql(&SearchEmailsFilterQuery::AfterDate(day)).unwrap(),
            "sent>2026-08-14"
        );
    }

    #[test]
    fn a_flag_clause_is_refused_by_name() {
        let filter = SearchEmailsFilterQuery::Flag(Flag::from_iana(IanaFlag::Seen));
        assert!(filter_to_kql(&filter).is_err());
    }
}
