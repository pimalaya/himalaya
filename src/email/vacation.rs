//! # Vacation
//!
//! An account's auto-reply, as the backends keeping one (JMAP, Gmail)
//! agree on it: whether it is on, the window it answers in, and what it
//! answers.

// NOTE: a build with neither JMAP nor Gmail keeps the shape for the
// commands it parses and refuses, nothing building one.
#![cfg_attr(not(any(feature = "jmap", feature = "gmail")), allow(dead_code))]

use anyhow::{Result, anyhow};
use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use schemars::JsonSchema;
use serde::Serialize;

/// An account's auto-reply.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Vacation {
    /// Whether the auto-reply is sent.
    pub enabled: bool,
    /// When it starts answering, RFC 3339 in UTC, `null` for no start.
    pub from: Option<String>,
    /// When it stops answering, RFC 3339 in UTC, `null` for no end.
    pub to: Option<String>,
    /// The subject of the reply, `null` for the backend's default.
    pub subject: Option<String>,
    /// The plain text body of the reply.
    pub text: Option<String>,
    /// The HTML body of the reply.
    pub html: Option<String>,
}

/// What `vacation set` changes, the rest left as it is: a field `None`
/// stays, `Some(None)` is cleared.
#[derive(Clone, Debug, Default)]
pub struct VacationPatch {
    pub enabled: Option<bool>,
    pub from: Option<Option<DateTime<Utc>>>,
    pub to: Option<Option<DateTime<Utc>>>,
    pub subject: Option<Option<String>>,
    pub text: Option<Option<String>>,
    pub html: Option<Option<String>>,
}

/// Parses a `--from` or `--to` value: an RFC 3339 date-time, or a bare
/// `YYYY-MM-DD` read as midnight UTC.
pub fn parse_date(value: &str) -> Result<DateTime<Utc>> {
    if let Ok(date) = DateTime::parse_from_rfc3339(value) {
        return Ok(date.with_timezone(&Utc));
    }

    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .ok()
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|date| date.and_utc())
        .ok_or_else(|| anyhow!("Invalid date `{value}`: expected RFC 3339 or YYYY-MM-DD"))
}

/// Writes a date as the shared shape carries it, RFC 3339 in UTC.
pub fn format_date(date: DateTime<Utc>) -> String {
    date.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Brings a date a backend wrote as RFC 3339 into UTC, keeping it as
/// written when it does not parse, so nothing it said is lost.
#[cfg(feature = "jmap")]
pub fn normalize_date(value: String) -> String {
    match DateTime::parse_from_rfc3339(&value) {
        Ok(date) => format_date(date.with_timezone(&Utc)),
        Err(_) => value,
    }
}

/// Reads a date a backend wrote as epoch milliseconds, keeping it as
/// written when it does not parse.
#[cfg(feature = "gmail")]
pub fn date_from_millis(value: String) -> String {
    value
        .parse::<i64>()
        .ok()
        .and_then(DateTime::from_timestamp_millis)
        .map(format_date)
        .unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_date_is_rfc_3339_or_a_bare_day_in_utc() {
        let offset = parse_date("2026-12-24T10:00:00+02:00").unwrap();
        assert_eq!(format_date(offset), "2026-12-24T08:00:00Z");

        let day = parse_date("2026-12-24").unwrap();
        assert_eq!(format_date(day), "2026-12-24T00:00:00Z");

        assert!(parse_date("24/12/2026").is_err());
    }

    #[cfg(feature = "gmail")]
    #[test]
    fn gmail_milliseconds_read_as_utc() {
        assert_eq!(
            date_from_millis("1798070400000".into()),
            "2026-12-24T00:00:00Z"
        );
        assert_eq!(date_from_millis("soon".into()), "soon");
    }

    #[cfg(feature = "jmap")]
    #[test]
    fn a_jmap_date_is_brought_to_utc() {
        assert_eq!(
            normalize_date("2026-12-24T10:00:00+02:00".into()),
            "2026-12-24T08:00:00Z"
        );
    }
}
