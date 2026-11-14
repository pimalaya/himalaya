//! # Search filter parser
//!
//! The chumsky grammar building a [`SearchEmailsFilterQuery`] from a
//! string.

use chrono::NaiveDate;
use chumsky::prelude::*;

use crate::email::{
    flag::Flag,
    search::{
        filter::query::SearchEmailsFilterQuery,
        parser::{ParserError, blanks, blanks1, hidden, keyword, mismatch},
    },
};

/// Parses a filter of space-separated conditions.
///
/// They join with `not`, `and` and `or`, in that order of precedence, and
/// group with parentheses. A pattern is quoted, or unquoted with its
/// spaces backslash-escaped. `himalaya envelope search --help` is the
/// grammar reference a user reads.
pub fn query<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    recursive(|filter| {
        let filter = choice((
            date(),
            after_date(),
            before_date(),
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

fn date<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    clause("date", "`date`", "space after `date`")
        .ignore_then(naive_date().labelled("date after `date`"))
        .map(SearchEmailsFilterQuery::Date)
}

fn after_date<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    clause("after", "`after`", "space after `after`")
        .ignore_then(naive_date().labelled("date after `after`"))
        .map(SearchEmailsFilterQuery::AfterDate)
}

fn before_date<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    clause("before", "`before`", "space after `before`")
        .ignore_then(naive_date().labelled("date after `before`"))
        .map(SearchEmailsFilterQuery::BeforeDate)
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

/// A date in one of the four accepted formats, refused as a whole word.
fn naive_date<'a>() -> impl Parser<'a, &'a str, NaiveDate, ParserError<'a>> + Clone {
    const FORMATS: [&str; 4] = ["%Y-%m-%d", "%Y/%m/%d", "%d-%m-%Y", "%d/%m/%Y"];

    pattern().try_map(|s, span| {
        FORMATS
            .iter()
            .find_map(|fmt| NaiveDate::parse_from_str(&s, fmt).ok())
            .ok_or_else(|| mismatch(span))
    })
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
            super::after_date().parse("after 2024-01-01").into_result(),
            Ok(AfterDate(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()))
        );
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
