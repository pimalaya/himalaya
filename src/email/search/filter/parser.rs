//! # Search emails filter query string parser
//!
//! Parsers needed to build a [`SearchEmailsFilterQuery`] from a string
//! slice. Based on [`chumsky`].

use chrono::NaiveDate;
use chumsky::prelude::*;

use crate::email::{
    flag::Flag,
    search::{filter::query::SearchEmailsFilterQuery, parser::ParserError},
};

/// The emails search filter query string parser.
///
/// A filter query string is composed of operators and conditions
/// separated by spaces. Operators and conditions can be wrapped in
/// parentheses `(…)` to override precedence.
///
/// # Operators
///
/// Three operators are supported, ordered by precedence (highest
/// first):
///
/// - `not <condition>`
/// - `<condition> and <condition>`
/// - `<condition> or <condition>`
///
/// `not` has the highest priority, then `and`, then `or`. `a and b or
/// c` is the same as `(a and b) or c`, but different from `a and (b or
/// c)`.
///
/// # Conditions
///
/// Seven conditions are supported:
///
/// - `date <yyyy-mm-dd>`
/// - `after <yyyy-mm-dd>`
/// - `from <pattern>`
/// - `to <pattern>`
/// - `subject <pattern>`
/// - `body <pattern>`
/// - `flag <flag>`
///
/// `<pattern>` can be quoted with `"` (`subject "foo bar"`) or
/// unquoted (spaces must be escaped with a backslash: `subject foo\
/// bar`).
pub fn query<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    recursive(|filter| {
        let filter = choice((
            date(),
            after_date(),
            from(),
            to(),
            subject(),
            body(),
            flag(),
            filter
                .delimited_by(lparen(), rparen())
                .labelled("(nested filter)"),
        ))
        .then_ignore(space().labelled("space between filters").repeated());

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
    just('n')
        .labelled("`not`")
        .ignore_then(just('o').labelled("`not`"))
        .ignore_then(just('t').labelled("`not`"))
        .ignore_then(space().labelled("space after `not`").repeated().at_least(1))
}

fn and<'a>() -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    just('a')
        .labelled("`and`")
        .ignore_then(just('n').labelled("`and`"))
        .ignore_then(just('d').labelled("`and`"))
        .ignore_then(space().labelled("space after `and`").repeated().at_least(1))
}

fn or<'a>() -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    just('o')
        .labelled("`or`")
        .ignore_then(just('r').labelled("`or`"))
        .ignore_then(space().labelled("space after `or`").repeated().at_least(1))
}

fn date<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    just('d')
        .labelled("`date`")
        .ignore_then(just('a').labelled("`date`"))
        .ignore_then(just('t').labelled("`date`"))
        .ignore_then(just('e').labelled("`date`"))
        .ignore_then(
            space()
                .labelled("space after `date`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(naive_date().labelled("date format after `date`"))
        .map(SearchEmailsFilterQuery::Date)
}

fn after_date<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    just('a')
        .labelled("`after`")
        .ignore_then(just('f').labelled("`after`"))
        .ignore_then(just('t').labelled("`after`"))
        .ignore_then(just('e').labelled("`after`"))
        .ignore_then(just('r').labelled("`after`"))
        .ignore_then(
            space()
                .labelled("space after `after`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(naive_date().labelled("pattern after `after`"))
        .map(SearchEmailsFilterQuery::AfterDate)
}

fn from<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    just('f')
        .labelled("`from`")
        .ignore_then(just('r').labelled("`from`"))
        .ignore_then(just('o').labelled("`from`"))
        .ignore_then(just('m').labelled("`from`"))
        .ignore_then(
            space()
                .labelled("space after `from`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(pattern().labelled("pattern after `from`"))
        .map(SearchEmailsFilterQuery::From)
}

fn to<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    just('t')
        .labelled("`to`")
        .ignore_then(just('o').labelled("`to`"))
        .ignore_then(space().labelled("space after `to`").repeated().at_least(1))
        .ignore_then(pattern().labelled("pattern after `to`"))
        .map(SearchEmailsFilterQuery::To)
}

fn subject<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    just('s')
        .labelled("`subject`")
        .ignore_then(just('u').labelled("`subject`"))
        .ignore_then(just('b').labelled("`subject`"))
        .ignore_then(just('j').labelled("`subject`"))
        .ignore_then(just('e').labelled("`subject`"))
        .ignore_then(just('c').labelled("`subject`"))
        .ignore_then(just('t').labelled("`subject`"))
        .ignore_then(
            space()
                .labelled("space after `subject`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(pattern().labelled("pattern after `subject`"))
        .map(SearchEmailsFilterQuery::Subject)
}

fn body<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    just('b')
        .labelled("`body`")
        .ignore_then(just('o').labelled("`body`"))
        .ignore_then(just('d').labelled("`body`"))
        .ignore_then(just('y').labelled("`body`"))
        .ignore_then(
            space()
                .labelled("space after `body`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(pattern().labelled("pattern after `body`"))
        .map(SearchEmailsFilterQuery::Body)
}

fn flag<'a>() -> impl Parser<'a, &'a str, SearchEmailsFilterQuery, ParserError<'a>> + Clone {
    just('f')
        .labelled("`flag`")
        .ignore_then(just('l').labelled("`flag`"))
        .ignore_then(just('a').labelled("`flag`"))
        .ignore_then(just('g').labelled("`flag`"))
        .ignore_then(
            space()
                .labelled("space after `flag`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(
            unquoted_pattern()
                .map(Flag::from_raw)
                .labelled("flag name after `flag`"),
        )
        .map(SearchEmailsFilterQuery::Flag)
}

fn naive_date<'a>() -> impl Parser<'a, &'a str, NaiveDate, ParserError<'a>> + Clone {
    choice((
        naive_date_with_fmt("%Y-%m-%d"),
        naive_date_with_fmt("%Y/%m/%d"),
        naive_date_with_fmt("%d-%m-%Y"),
        naive_date_with_fmt("%d/%m/%Y"),
    ))
}

fn naive_date_with_fmt(fmt: &str) -> impl Parser<'_, &str, NaiveDate, ParserError<'_>> + Clone {
    pattern().try_map(move |ref s, span| {
        NaiveDate::parse_from_str(s, fmt).map_err(|err| Rich::custom(span, err.to_string()))
    })
}

fn pattern<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
    choice((quoted_pattern(), unquoted_pattern()))
}

fn quoted_pattern<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
    let escapable_chars = ['\\', '"'];

    dquote()
        .then(
            choice((
                bslash().ignore_then(one_of(escapable_chars)),
                none_of(escapable_chars),
            ))
            .repeated(),
        )
        .then(dquote())
        .to_slice()
        .map(String::from)
}

fn unquoted_pattern<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
    let escapable_chars = ['\\', ' ', '(', ')'];

    choice((
        bslash().ignore_then(one_of(escapable_chars)),
        none_of(escapable_chars),
    ))
    .repeated()
    .at_least(1)
    .collect()
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
            Ok("\"\"".into())
        );

        assert_eq!(
            super::quoted_pattern()
                .parse("\"quoted pattern\"")
                .into_result(),
            Ok("\"quoted pattern\"".into()),
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
            Ok(From("\"quoted val\"".into())),
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
                    Box::new(Subject("\"s with parens )\"".into()))
                )),
            )),
        );
    }
}
