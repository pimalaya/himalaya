//! # Microsoft Graph search
//!
//! Translation of the shared search filter into a KQL `$search` value, the
//! one Graph query reaching the address fields, subject and body, matched as
//! KQL matches them rather than by substring. Graph refuses `$search`
//! beside `$filter`, and KQL carries no flag property, so a flag clause is
//! refused with `search-unsupported` rather than dropped.
//!
//! <https://learn.microsoft.com/en-us/graph/search-query-parameter>

use anyhow::Result;
use chrono::NaiveDate;

use crate::email::search::{error::unsupported, filter::query::SearchEmailsFilterQuery};

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
        SearchEmailsFilterQuery::BeforeDate(date) => format!("sent<{}", date_kql(*date)),
        SearchEmailsFilterQuery::From(pattern) => format!("from:{}", value_kql(pattern)),
        SearchEmailsFilterQuery::To(pattern) => format!("to:{}", value_kql(pattern)),
        SearchEmailsFilterQuery::Cc(pattern) => format!("cc:{}", value_kql(pattern)),
        SearchEmailsFilterQuery::Bcc(pattern) => format!("bcc:{}", value_kql(pattern)),
        // NOTE: KQL's `participants` reads the From, To, Cc and Bcc fields.
        SearchEmailsFilterQuery::Address(pattern) => {
            format!("participants:{}", value_kql(pattern))
        }
        SearchEmailsFilterQuery::Subject(pattern) => format!("subject:{}", value_kql(pattern)),
        SearchEmailsFilterQuery::Body(pattern) => format!("body:{}", value_kql(pattern)),
        SearchEmailsFilterQuery::Attachment => String::from("hasAttachments:true"),
        SearchEmailsFilterQuery::Flag(_) => {
            return Err(unsupported(
                filter,
                "Microsoft Graph search has no flag property; use `himalaya msgraph messages list --filter`",
            ));
        }
    })
}

/// Renders a pattern as a KQL value: each word a quoted phrase, escaped
/// for the `$search` quotes around the whole query, so a word that is KQL
/// syntax (`OR`, `x:y`, `alpha*`) is matched as a word; several words
/// group in parentheses. A double quote has no way through.
fn value_kql(pattern: &str) -> String {
    let words: Vec<String> = pattern
        .split_whitespace()
        .map(|word| format!("\\\"{}\\\"", word.replace('"', "")))
        .collect();

    match words.as_slice() {
        [word] => word.clone(),
        words => format!("({})", words.join(" ")),
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
            r#""(from:\"alice\") AND (NOT (subject:(\"weekly\" \"report\")))""#
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
    fn copies_addresses_dates_and_attachments() {
        let day = NaiveDate::from_ymd_opt(2026, 8, 14).unwrap();

        assert_eq!(
            filter_to_kql(&SearchEmailsFilterQuery::BeforeDate(day)).unwrap(),
            "sent<2026-08-14"
        );
        assert_eq!(
            filter_to_kql(&SearchEmailsFilterQuery::Cc("carol".into())).unwrap(),
            r#"cc:\"carol\""#
        );
        assert_eq!(
            filter_to_kql(&SearchEmailsFilterQuery::Address("eve".into())).unwrap(),
            r#"participants:\"eve\""#
        );
        assert_eq!(
            filter_to_kql(&SearchEmailsFilterQuery::Attachment).unwrap(),
            "hasAttachments:true"
        );
    }

    #[test]
    fn a_word_that_is_kql_syntax_is_matched_as_a_word() {
        let kql =
            |pattern: &str| filter_to_kql(&SearchEmailsFilterQuery::From(pattern.into())).unwrap();

        assert_eq!(kql("OR"), r#"from:\"OR\""#);
        assert_eq!(kql("x:y"), r#"from:\"x:y\""#);
        assert_eq!(kql("say \"hi\""), r#"from:(\"say\" \"hi\")"#);
    }

    #[test]
    fn a_flag_clause_is_refused_by_name() {
        let filter = SearchEmailsFilterQuery::Flag(Flag::from_iana(IanaFlag::Seen));
        assert!(filter_to_kql(&filter).is_err());
    }
}
