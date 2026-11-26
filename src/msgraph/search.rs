//! # Microsoft Graph search
//!
//! The unified filter becomes a KQL `$search`, the one Graph query reaching
//! addresses, subject and body, matched as KQL does, not by substring.
//! Graph refuses `$search` beside `$filter` and KQL has no flag property, so
//! a flag clause is refused with `search-unsupported` rather than dropped.
//!
//! <https://learn.microsoft.com/en-us/graph/search-query-parameter>

use anyhow::Result;
use chrono::NaiveDate;

use crate::unified::{
    capability::GRAPH_DAYS,
    email::search::{
        error::unsupported,
        filter::query::{SearchEmailsFilterQuery, SearchEmailsFilterText},
    },
};

/// Translates the filter half of a unified query into the quoted `$search`
/// value Graph takes.
pub(super) fn filter_to_search(filter: &SearchEmailsFilterQuery) -> Result<String> {
    Ok(format!("\"{}\"", filter_to_kql(filter)?))
}

/// The `$filter` of a filter that is one `message-id` clause, the one shape
/// Graph answers through `$filter`, quotes doubled as OData escapes them.
pub(super) fn message_id_filter(filter: &SearchEmailsFilterQuery) -> Option<String> {
    match filter {
        SearchEmailsFilterQuery::MessageId(id) => Some(format!(
            "internetMessageId eq '<{}>'",
            id.replace('\'', "''")
        )),
        _ => None,
    }
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
        SearchEmailsFilterQuery::Date(date) => format!("sent:{}", date_kql(*date)),
        SearchEmailsFilterQuery::AfterDate(date) => format!("sent>{}", date_kql(*date)),
        SearchEmailsFilterQuery::BeforeDate(date) => format!("sent<{}", date_kql(*date)),
        SearchEmailsFilterQuery::AfterInstant(_) | SearchEmailsFilterQuery::BeforeInstant(_) => {
            return Err(unsupported(filter, GRAPH_DAYS));
        }
        SearchEmailsFilterQuery::From(pattern) => format!("from:{}", value_kql(pattern)),
        SearchEmailsFilterQuery::To(pattern) => format!("to:{}", value_kql(pattern)),
        SearchEmailsFilterQuery::Cc(pattern) => format!("cc:{}", value_kql(pattern)),
        SearchEmailsFilterQuery::Bcc(pattern) => format!("bcc:{}", value_kql(pattern)),
        SearchEmailsFilterQuery::Address(pattern) => {
            format!("participants:{}", value_kql(pattern))
        }
        SearchEmailsFilterQuery::Subject(pattern) => format!("subject:{}", value_kql(pattern)),
        SearchEmailsFilterQuery::Body(pattern) => format!("body:{}", value_kql(pattern)),
        SearchEmailsFilterQuery::Text(pattern) => value_kql(pattern),
        SearchEmailsFilterQuery::Attachment => String::from("hasAttachments:true"),
        SearchEmailsFilterQuery::Flag(_) => {
            return Err(unsupported(
                filter,
                "Microsoft Graph search has no flag property; use `himalaya msgraph messages list --filter`",
            ));
        }
        SearchEmailsFilterQuery::MessageId(_) => {
            return Err(unsupported(
                filter,
                "Microsoft Graph matches a Message-ID through `$filter` alone, which takes no other clause beside it",
            ));
        }
    })
}

/// Renders a text as KQL, a `"` dropped.
///
/// A phrase is one quoted string, a prefix quoted with its `*`, and any
/// other word an escaped quoted phrase, so KQL syntax (`OR`, `x:y`,
/// `alpha*`) matches as a word.
fn value_kql(text: &SearchEmailsFilterText) -> String {
    let quote = |word: &str, star: &str| format!("\\\"{}{star}\\\"", word.replace('"', ""));

    if text.phrase {
        return quote(&text.text, "");
    }
    if text.prefix {
        return quote(&text.text, "*");
    }

    let words: Vec<String> = text
        .text
        .split_whitespace()
        .map(|word| quote(word, ""))
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
    use crate::unified::email::{
        flag::{Flag, IanaFlag},
        search::query::SearchEmailsQuery,
    };

    #[test]
    fn a_lone_message_id_is_a_filter_and_refused_in_a_search() {
        let id = SearchEmailsFilterQuery::MessageId("o'neil@example.org".into());

        assert_eq!(
            message_id_filter(&id).unwrap(),
            "internetMessageId eq '<o''neil@example.org>'"
        );

        let both = SearchEmailsFilterQuery::And(
            Box::new(id),
            Box::new(SearchEmailsFilterQuery::From("alice".into())),
        );
        assert_eq!(message_id_filter(&both), None);
        assert!(filter_to_search(&both).is_err());
    }

    /// KQL compares days alone: an instant is refused, coded
    /// `search-unsupported`, nothing searched.
    ///
    /// The words are the capability check's for `date.instant`.
    #[test]
    fn an_instant_is_refused() {
        let at = chrono::DateTime::parse_from_rfc3339("2026-11-14T10:00:00Z").unwrap();
        let err = filter_to_search(&SearchEmailsFilterQuery::AfterInstant(at)).unwrap_err();
        let code = pimalaya_cli::error::code_of::<crate::error::ErrorCode>(&err);
        assert_eq!(code, Some(crate::error::ErrorCode::SearchUnsupported));
        assert_eq!(
            err.to_string(),
            "Cannot search `date:2026-11-14T10:00:00+00:00..`: Microsoft Graph search \
             compares days alone; give a day (`date:2026-11-24..`) or `today`"
        );
    }

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

    /// A prefix reaches KQL with its `*`, a phrase as one quoted string,
    /// so neither is answered as other words.
    #[test]
    fn a_prefix_and_a_phrase_keep_their_meaning() {
        let kql = |query: &str| {
            let query: SearchEmailsQuery = query.parse().unwrap();
            filter_to_kql(&query.filter.unwrap()).unwrap()
        };

        assert_eq!(kql("subject:repo*"), r#"subject:\"repo*\""#);
        assert_eq!(kql("quart*"), r#"\"quart*\""#);
        assert_eq!(kql("from:ali*"), r#"from:\"ali*\""#);
        assert_eq!(
            kql("subject:\"report weekly\""),
            r#"subject:\"report weekly\""#
        );
        assert_eq!(kql("\"a OR b\""), r#"\"a OR b\""#);
    }

    #[test]
    fn a_flag_clause_is_refused_by_name() {
        let filter = SearchEmailsFilterQuery::Flag(Flag::from_iana(IanaFlag::Seen));
        assert!(filter_to_kql(&filter).is_err());
    }
}
