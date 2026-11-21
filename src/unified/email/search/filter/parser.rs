//! # Search filter parser
//!
//! The chumsky grammar building a [`SearchEmailsFilterQuery`] from a
//! string.

use std::path::Path;

use chrono::{
    DateTime, Days, FixedOffset, Local, Months, NaiveDate, NaiveDateTime, TimeDelta, TimeZone,
    Timelike,
};
use chumsky::prelude::*;
use log::warn;

use crate::unified::email::{
    flag::Flag,
    search::{
        filter::query::SearchEmailsFilterQuery,
        parser::{ParserError, blanks, blanks1, hidden, keyword, mismatch, word_chars},
    },
};

/// Parses a filter of space-separated conditions.
///
/// They join with `not`, `and` and `or`, in that order of precedence, and
/// group with parentheses. A pattern is quoted, or unquoted with its
/// spaces backslash-escaped. `himalaya envelope search --help` is the
/// grammar reference a user reads.
///
/// Its dates are read in the local zone, now taken once, as the parser is
/// built ([`query_at`]).
pub fn query<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    query_at(local_now())
}

/// Parses a filter as [`query`] does, its relative dates counted back
/// from `now` and its dates with no offset read in the zone of `now`.
pub fn query_at<'a, Tz: TimeZone + 'a>(
    now: DateTime<Tz>,
) -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    recursive(move |filter| {
        let filter = choice((
            date(now.clone()),
            after_date(now.clone()),
            before_date(now.clone()),
            from(),
            to(),
            cc(),
            bcc(),
            address(),
            subject(),
            body(),
            flag(),
            attachment(),
            message_id(),
            filter
                .delimited_by(lparen().then_ignore(blanks()), rparen())
                .labelled("(nested filter)"),
        ))
        .then_ignore(blanks());

        let not = not().repeated().foldr(filter, |_, filter| {
            SearchEmailsFilterQuery::Not(Box::new(filter))
        });

        let and = not
            .clone()
            .foldl(and().then(not).repeated(), |left, (_, right)| {
                SearchEmailsFilterQuery::And(Box::new(left), Box::new(right))
            });

        and.clone()
            .foldl(or().then(and).repeated(), |left, (_, right)| {
                SearchEmailsFilterQuery::Or(Box::new(left), Box::new(right))
            })
    })
}

fn not<'a>() -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    keyword("not", "`not`").ignore_then(blanks1("space after `not`"))
}

fn and<'a>() -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    keyword("and", "`and`").ignore_then(blanks1("space after `and`"))
}

fn or<'a>() -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    keyword("or", "`or`").ignore_then(blanks1("space after `or`"))
}

/// `date` and a day, an instant refused.
fn date<'a, Tz: TimeZone + 'a>(
    now: DateTime<Tz>,
) -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    clause("date", "`date`", "space after `date`")
        .ignore_then(dated(now, "date after `date`"))
        .try_map(|value, span| match value {
            DateValue::Day(day) => Ok(SearchEmailsFilterQuery::Date(day)),
            DateValue::Instant(_) => Err(Rich::custom(
                span,
                "`date` takes a day, not an instant: write `after` or `before`",
            )),
        })
}

fn after_date<'a, Tz: TimeZone + 'a>(
    now: DateTime<Tz>,
) -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    clause("after", "`after`", "space after `after`")
        .ignore_then(dated(now, "date after `after`"))
        .map(|value| match value {
            DateValue::Day(day) => SearchEmailsFilterQuery::AfterDate(day),
            DateValue::Instant(instant) => SearchEmailsFilterQuery::AfterInstant(instant),
        })
}

fn before_date<'a, Tz: TimeZone + 'a>(
    now: DateTime<Tz>,
) -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    clause("before", "`before`", "space after `before`")
        .ignore_then(dated(now, "date after `before`"))
        .map(|value| match value {
            DateValue::Day(day) => SearchEmailsFilterQuery::BeforeDate(day),
            DateValue::Instant(instant) => SearchEmailsFilterQuery::BeforeInstant(instant),
        })
}

fn from<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    clause("from", "`from`", "space after `from`")
        .ignore_then(pattern().labelled("pattern after `from`"))
        .map(SearchEmailsFilterQuery::From)
}

fn to<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    clause("to", "`to`", "space after `to`")
        .ignore_then(pattern().labelled("pattern after `to`"))
        .map(SearchEmailsFilterQuery::To)
}

fn cc<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    clause("cc", "`cc`", "space after `cc`")
        .ignore_then(pattern().labelled("pattern after `cc`"))
        .map(SearchEmailsFilterQuery::Cc)
}

fn bcc<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    clause("bcc", "`bcc`", "space after `bcc`")
        .ignore_then(pattern().labelled("pattern after `bcc`"))
        .map(SearchEmailsFilterQuery::Bcc)
}

fn address<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    clause("address", "`address`", "space after `address`")
        .ignore_then(pattern().labelled("pattern after `address`"))
        .map(SearchEmailsFilterQuery::Address)
}

fn subject<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    clause("subject", "`subject`", "space after `subject`")
        .ignore_then(pattern().labelled("pattern after `subject`"))
        .map(SearchEmailsFilterQuery::Subject)
}

fn body<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    clause("body", "`body`", "space after `body`")
        .ignore_then(pattern().labelled("pattern after `body`"))
        .map(SearchEmailsFilterQuery::Body)
}

fn flag<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    clause("flag", "`flag`", "space after `flag`")
        .ignore_then(
            unquoted_pattern()
                .map(Flag::from_raw)
                .labelled("flag name after `flag`"),
        )
        .map(SearchEmailsFilterQuery::Flag)
}

/// The valueless attachment clause, which ends where its word does.
fn attachment<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    keyword("attachment", "`attachment`")
        .then_ignore(
            choice((space(), rparen()))
                .rewind()
                .ignored()
                .or(end())
                .labelled("space or end after `attachment`"),
        )
        .to(SearchEmailsFilterQuery::Attachment)
}

/// `message-id` and the id, held bare, its angle brackets optional. The
/// grammar's words stop at a hyphen, so the keyword reads in three parts.
fn message_id<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    keyword("message", "`message-id`")
        .ignore_then(just('-').labelled("`message-id`"))
        .ignore_then(keyword("id", "`message-id`"))
        .ignore_then(blanks1("space after `message-id`"))
        .ignore_then(pattern().labelled("id after `message-id`"))
        .try_map(|raw, span| {
            let trimmed = raw.trim();
            let bare = trimmed
                .strip_prefix('<')
                .and_then(|id| id.strip_suffix('>'))
                .unwrap_or(trimmed)
                .trim();

            if bare.is_empty() {
                Err(mismatch(span))
            } else {
                Ok(SearchEmailsFilterQuery::MessageId(bare.to_owned()))
            }
        })
}

/// A clause keyword followed by the spaces separating it from its value.
fn clause<'a>(
    word: &'static str,
    label: &'static str,
    space_label: &'static str,
) -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    keyword(word, label).ignore_then(blanks1(space_label))
}

/// What a date clause takes: a calendar day, or an instant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DateValue {
    /// A day, compared with the day a message is dated.
    Day(NaiveDate),
    /// An instant, compared to the second.
    Instant(DateTime<FixedOffset>),
}

/// The local zone's now, `TZ` then the system's zone, warning when
/// neither names one and the dates read in UTC.
fn local_now() -> DateTime<Local> {
    let tz = std::env::var_os("TZ").filter(|tz| !tz.is_empty());
    if cfg!(unix) && tz.is_none() && !Path::new("/etc/localtime").exists() {
        warn!("local time zone unknown, reading search dates in UTC; set TZ to one");
    }

    Local::now()
}

/// A [`date_value`] labelled `label`, a day spelled with a short year
/// refused by name: the label would hide a refusal of its own.
fn dated<'a, Tz: TimeZone + 'a>(
    now: DateTime<Tz>,
    label: &'static str,
) -> impl Parser<'a, &'a str, DateValue, ParserError<'a>> + Clone {
    let short = pattern().try_map(|value, span| {
        Err(match short_year(&value) {
            true => Rich::custom(span, "a year takes four digits, as in `2026-11-15`"),
            false => mismatch(span),
        })
    });

    choice((short, date_value(now).labelled(label)))
}

/// A date value: `today`, `yesterday`, `<n> <unit> ago`, a day in one of
/// its four spellings or an RFC 3339 date-time, its seconds and offset
/// optional, one with no offset read in the zone of `now`.
fn date_value<'a, Tz: TimeZone + 'a>(
    now: DateTime<Tz>,
) -> impl Parser<'a, &'a str, DateValue, ParserError<'a>> + Clone {
    let today = now.date_naive();
    let yesterday = today.pred_opt().unwrap_or(today);
    let zone = now.timezone();
    let now = now.fixed_offset();

    let count = unquoted_pattern().try_map(|count: String, span| {
        match count.bytes().all(|byte| byte.is_ascii_digit()) {
            true => Ok(count),
            false => Err(mismatch(span)),
        }
    });
    let ago = count
        .then_ignore(blanks1("space after the number"))
        .then(word_chars().labelled("unit, `day` to `year` or `minute` and `hour`"))
        .then_ignore(blanks1("space before `ago`"))
        .then_ignore(keyword("ago", "`ago`"))
        .try_map(move |(count, unit), span| ago(now, &count, unit).ok_or_else(|| mismatch(span)));

    choice((
        ago,
        keyword("today", "`today`").to(DateValue::Day(today)),
        keyword("yesterday", "`yesterday`").to(DateValue::Day(yesterday)),
        pattern().try_map(move |s, span| absolute(&s, &zone).ok_or_else(|| mismatch(span))),
    ))
}

/// The value `<count> <unit> ago` names from `now`: a day, counted back
/// in calendar terms, for `day` to `year`, an instant for `minute` and
/// `hour`, each singular or plural.
fn ago(now: DateTime<FixedOffset>, count: &str, unit: &str) -> Option<DateValue> {
    let count: u32 = count.parse().ok()?;
    let today = now.date_naive();
    let instant = |delta: Option<TimeDelta>| now.checked_sub_signed(delta?).map(to_the_second);

    match unit {
        "day" | "days" => today
            .checked_sub_days(Days::new(count.into()))
            .map(DateValue::Day),
        "week" | "weeks" => {
            let days = u64::from(count) * 7;
            today.checked_sub_days(Days::new(days)).map(DateValue::Day)
        }
        "month" | "months" => today
            .checked_sub_months(Months::new(count))
            .map(DateValue::Day),
        "year" | "years" => {
            let months = count.checked_mul(12)?;
            today
                .checked_sub_months(Months::new(months))
                .map(DateValue::Day)
        }
        "minute" | "minutes" => {
            instant(TimeDelta::try_minutes(count.into())).map(DateValue::Instant)
        }
        "hour" | "hours" => instant(TimeDelta::try_hours(count.into())).map(DateValue::Instant),
        _ => None,
    }
}

/// A day or an RFC 3339 date-time, its seconds and offset optional, one
/// with no offset read in `zone`.
fn absolute<Tz: TimeZone>(value: &str, zone: &Tz) -> Option<DateValue> {
    if let Some(day) = day(value) {
        return Some(DateValue::Day(day));
    }

    let (date, _) = value.split_once(['T', 't'])?;
    day(date).filter(|_| date.len() == 10 && date.as_bytes()[4] == b'-')?;

    let value = match value.strip_suffix(['Z', 'z']) {
        Some(utc) => format!("{utc}+00:00"),
        None => value.replace('t', "T"),
    };

    for format in ["%Y-%m-%dT%H:%M:%S%.f%:z", "%Y-%m-%dT%H:%M%:z"] {
        if let Ok(instant) = DateTime::parse_from_str(&value, format) {
            return Some(DateValue::Instant(to_the_second(instant)));
        }
    }

    for format in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%dT%H:%M"] {
        if let Ok(local) = NaiveDateTime::parse_from_str(&value, format) {
            let instant = zone.from_local_datetime(&local).earliest()?;
            return Some(DateValue::Instant(to_the_second(instant.fixed_offset())));
        }
    }

    None
}

/// `instant` without its fraction of a second, which no message date
/// carries.
fn to_the_second(instant: DateTime<FixedOffset>) -> DateTime<FixedOffset> {
    instant.with_nanosecond(0).unwrap_or(instant)
}

/// A day as `2026-11-14`, `2026/11/14`, `14-11-2026` or `14/11/2026`, its
/// year written with four digits, its month and day with one or two.
fn day(value: &str) -> Option<NaiveDate> {
    if value.contains('-') && value.contains('/') {
        return None;
    }

    let parts: Vec<&str> = value.split(['-', '/']).collect();
    let [first, month, last] = parts.as_slice() else {
        return None;
    };
    let digits = |part: &str, lens: &[usize]| {
        lens.contains(&part.len()) && part.bytes().all(|byte| byte.is_ascii_digit())
    };

    let (year, day) = match (first.len(), last.len()) {
        (4, _) => (first, last),
        (_, 4) => (last, first),
        _ => return None,
    };
    if !digits(year, &[4]) || !digits(month, &[1, 2]) || !digits(day, &[1, 2]) {
        return None;
    }

    NaiveDate::from_ymd_opt(year.parse().ok()?, month.parse().ok()?, day.parse().ok()?)
}

/// Whether `value` is spelled as a day whose year lacks digits: three
/// runs of digits joined by `-` or `/`, none of four.
fn short_year(value: &str) -> bool {
    let parts: Vec<&str> = value.split(['-', '/']).collect();
    let numeric = |part: &&str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());

    parts.len() == 3 && parts.iter().all(numeric) && parts.iter().all(|part| part.len() < 4)
}

fn pattern<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
    choice((quoted_pattern(), unquoted_pattern()))
}

fn quoted_pattern<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
    let escapable_chars = ['\\', '"'];

    dquote()
        .ignore_then(
            choice((
                bslash().ignore_then(one_of(escapable_chars)),
                none_of(escapable_chars),
            ))
            .labelled(hidden())
            .repeated()
            .collect(),
        )
        .then_ignore(just('"').labelled("closing double quote"))
}

/// A pattern up to the next unescaped space or parenthesis.
///
/// It cannot open with a bare double quote, which opens a quoted pattern,
/// so an unterminated one is refused rather than read as a literal.
fn unquoted_pattern<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
    let escapable_chars = ['\\', ' ', '(', ')', '"'];
    let escaped = bslash().ignore_then(one_of(escapable_chars));

    choice((escaped.clone(), none_of(escapable_chars)))
        .then(
            choice((escaped, none_of(['\\', ' ', '(', ')'])))
                .labelled(hidden())
                .repeated()
                .collect::<Vec<char>>(),
        )
        .map(|(first, rest)| std::iter::once(first).chain(rest).collect())
}

fn space<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
    just(' ')
}

fn lparen<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
    just('(').labelled("nested filter opening '('")
}

fn rparen<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
    just(')').labelled("nested filter closing ')'")
}

fn bslash<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
    just('\\').labelled("backslash")
}

fn dquote<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
    just('"').labelled("double quote")
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use chumsky::prelude::*;

    use super::SearchEmailsFilterQuery::*;

    #[test]
    fn pattern() {
        assert_eq!(
            super::unquoted_pattern().parse("pattern").into_result(),
            Ok("pattern".into())
        );

        assert_eq!(
            super::unquoted_pattern()
                .parse("escaped\\ chars\\)")
                .into_result(),
            Ok("escaped chars)".into()),
        );

        assert_eq!(
            super::quoted_pattern().parse("\"\"").into_result(),
            Ok("".into())
        );

        assert_eq!(
            super::quoted_pattern()
                .parse("\"quoted pattern\"")
                .into_result(),
            Ok("quoted pattern".into()),
        );

        assert_eq!(
            super::quoted_pattern()
                .parse("\"escaped \\\" quote\"")
                .into_result(),
            Ok("escaped \" quote".into()),
        );
    }

    #[test]
    fn after_date() {
        assert_eq!(
            super::after_date(now())
                .parse("after 2024-01-01")
                .into_result(),
            Ok(AfterDate(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()))
        );
    }

    /// A year is written with its four digits: a two-digit one is refused,
    /// where `15-11-26` read as the year 15.
    #[test]
    fn a_two_digit_year_is_refused() {
        for query in ["after 15-11-26", "before 26/11/15", "date 1-1-1"] {
            let errs = super::query().parse(query).into_result().unwrap_err();
            let reason = errs[0].reason().to_string();
            assert!(
                reason.contains("a year takes four digits"),
                "{query}: {reason}"
            );
        }
        for query in ["after 2026-13-45", "after foo", "after 3 dayz ago"] {
            let errs = super::query().parse(query).into_result().unwrap_err();
            let reason = errs[0].reason().to_string();
            assert!(!reason.contains("four digits"), "{query}: {reason}");
        }
        assert!(
            super::query()
                .parse("after 01-02-2026")
                .into_result()
                .is_ok()
        );
    }

    /// 2026-11-14T15:00 at UTC+01:00, the now of the date value tests.
    fn now() -> chrono::DateTime<chrono::FixedOffset> {
        chrono::DateTime::parse_from_rfc3339("2026-11-14T15:00:30.5+01:00").unwrap()
    }

    fn parsed(query: &str) -> Result<super::SearchEmailsFilterQuery, String> {
        super::query_at(now())
            .parse(query)
            .into_result()
            .map_err(|errs| format!("{errs:?}"))
    }

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn instant(rfc3339: &str) -> chrono::DateTime<chrono::FixedOffset> {
        chrono::DateTime::parse_from_rfc3339(rfc3339).unwrap()
    }

    /// The relative days count back in calendar terms from today, the
    /// relative hours and minutes from now, to the second.
    #[test]
    fn a_relative_date_counts_back_from_now() {
        assert_eq!(parsed("after today"), Ok(AfterDate(day(2026, 11, 14))));
        assert_eq!(parsed("date yesterday"), Ok(Date(day(2026, 11, 13))));
        assert_eq!(parsed("after 2 days ago"), Ok(AfterDate(day(2026, 11, 12))));
        assert_eq!(parsed("after 1 day ago"), Ok(AfterDate(day(2026, 11, 13))));
        assert_eq!(
            parsed("before 2 weeks ago"),
            Ok(BeforeDate(day(2026, 10, 31)))
        );
        assert_eq!(parsed("after 1 year ago"), Ok(AfterDate(day(2025, 11, 14))));
        assert_eq!(
            parsed("after 5 hours ago"),
            Ok(AfterInstant(instant("2026-11-14T10:00:30+01:00")))
        );
        assert_eq!(
            parsed("before 90 minutes ago and from a"),
            Ok(And(
                Box::new(BeforeInstant(instant("2026-11-14T13:30:30+01:00"))),
                Box::new(From("a".into()))
            ))
        );

        let end_of_march = chrono::DateTime::parse_from_rfc3339("2026-03-31T12:00:00Z").unwrap();
        let month = super::query_at(end_of_march).parse("after 1 month ago");
        assert_eq!(month.into_result(), Ok(AfterDate(day(2026, 2, 28))));
    }

    /// An RFC 3339 date-time is an instant, its seconds and offset
    /// optional, one with no offset read in the zone of now.
    #[test]
    fn a_date_time_is_an_instant() {
        let at = |rfc3339| Ok(AfterInstant(instant(rfc3339)));
        assert_eq!(
            parsed("after 2026-11-14T09:30"),
            at("2026-11-14T09:30:00+01:00")
        );
        assert_eq!(
            parsed("after 2026-11-14T09:30:15"),
            at("2026-11-14T09:30:15+01:00")
        );
        assert_eq!(
            parsed("after 2026-11-14T09:30Z"),
            at("2026-11-14T09:30:00Z")
        );
        assert_eq!(
            parsed("after 2026-11-14T09:30:15.9-05:00"),
            at("2026-11-14T09:30:15-05:00")
        );
    }

    /// `date` takes a day alone, and a value of another shape is refused.
    #[test]
    fn a_date_value_of_no_known_shape_is_refused() {
        for query in [
            "date 5 hours ago",
            "date 2026-11-14T09:30",
            "after 2 fortnights ago",
            "after 2 days",
            "after 15-11-26T10:00",
            "after tomorrow",
        ] {
            assert!(parsed(query).is_err(), "{query}");
        }
    }

    #[test]
    fn from() {
        assert_eq!(
            super::from().parse("from unquoted-val").into_result(),
            Ok(From("unquoted-val".into())),
        );

        assert_eq!(
            super::from().parse("from \"quoted val\"").into_result(),
            Ok(From("quoted val".into())),
        );
    }

    #[test]
    fn filter() {
        assert_eq!(
            super::query()
                .parse("from f and to t and subject s")
                .into_result(),
            Ok(And(
                Box::new(And(Box::new(From("f".into())), Box::new(To("t".into())))),
                Box::new(Subject("s".into()))
            )),
        );

        assert_eq!(
            super::query()
                .parse("subject or or subject and")
                .into_result(),
            Ok(Or(
                Box::new(Subject("or".into())),
                Box::new(Subject("and".into()))
            )),
        );

        assert_eq!(
            super::query()
                .parse("from f and (to t and subject s)")
                .into_result(),
            Ok(And(
                Box::new(From("f".into())),
                Box::new(And(Box::new(To("t".into())), Box::new(Subject("s".into())))),
            )),
        );

        assert_eq!(
            super::query()
                .parse("from f and to t or subject s")
                .into_result(),
            Ok(Or(
                Box::new(And(Box::new(From("f".into())), Box::new(To("t".into())))),
                Box::new(Subject("s".into()))
            )),
        );

        assert_eq!(
            super::query()
                .parse("from f or to t and not subject s")
                .into_result(),
            Ok(Or(
                Box::new(From("f".into())),
                Box::new(And(
                    Box::new(To("t".into())),
                    Box::new(Not(Box::new(Subject("s".into()))))
                )),
            )),
        );

        assert_eq!(
            super::query()
                .parse("from f and (to t or subject \"s with parens )\")")
                .into_result(),
            Ok(And(
                Box::new(From("f".into())),
                Box::new(Or(
                    Box::new(To("t".into())),
                    Box::new(Subject("s with parens )".into()))
                )),
            )),
        );
    }

    #[test]
    fn copies_addresses_dates_and_attachments() {
        let day = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();

        assert_eq!(
            super::query().parse("before 2026-01-01").into_result(),
            Ok(BeforeDate(day)),
        );
        assert_eq!(
            super::query().parse("cc carol and bcc dave").into_result(),
            Ok(And(
                Box::new(Cc("carol".into())),
                Box::new(Bcc("dave".into()))
            )),
        );
        assert_eq!(
            super::query().parse("address \"eve smith\"").into_result(),
            Ok(Address("eve smith".into())),
        );
        assert_eq!(
            super::query()
                .parse("not attachment and from a")
                .into_result(),
            Ok(And(
                Box::new(Not(Box::new(Attachment))),
                Box::new(From("a".into()))
            )),
        );
        assert_eq!(
            super::query().parse("(attachment)").into_result(),
            Ok(Attachment),
        );
    }

    #[test]
    fn attachment_ends_where_its_word_does() {
        assert!(super::query().parse("attachments").into_result().is_err());
        assert!(super::query().parse("attachment x").into_result().is_err());
    }

    #[test]
    fn message_id_is_held_bare() {
        let id = || Ok(MessageId("id@example.com".into()));

        assert_eq!(
            super::query()
                .parse("message-id id@example.com")
                .into_result(),
            id()
        );
        assert_eq!(
            super::query()
                .parse("message-id <id@example.com>")
                .into_result(),
            id()
        );
        assert_eq!(
            super::query()
                .parse("message-id \" <id@example.com> \"")
                .into_result(),
            id()
        );
    }

    #[test]
    fn message_id_needs_its_whole_word_and_an_id() {
        assert!(super::query().parse("message id").into_result().is_err());
        assert!(super::query().parse("message-ids x").into_result().is_err());
        assert!(super::query().parse("message-id <>").into_result().is_err());
        assert!(super::query().parse("message-id").into_result().is_err());
    }
}
