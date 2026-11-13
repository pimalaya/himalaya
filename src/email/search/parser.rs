//! # Search query parser
//!
//! Entry point parsing a whole search query from a string, deferring the
//! grammar to [`super::filter::parser`] and [`super::sort::parser`], and
//! the lexical pieces both halves share.

use chumsky::{DefaultExpected, error::LabelError, prelude::*};

use crate::email::search::{
    error::Error, filter, query::SearchEmailsQuery, sort::parser::order_by,
};

/// A rich chumsky error, which is what the diagnostics are drawn from.
pub type ParserError<'a> = extra::Err<Rich<'a, char>>;

/// Parses a string into a [`SearchEmailsQuery`], which may carry a
/// filter, a sort, or both.
///
/// The filter comes first and the sort last, introduced by the words
/// `order by`, so a pattern holding them, quoted or escaped, stays a
/// pattern. The diagnostics are raised over the whole query.
pub fn parse(input: impl AsRef<str>) -> Result<SearchEmailsQuery, Error> {
    let input = input.as_ref().trim();

    query().parse(input).into_result().map_err(|errs| {
        let errs = errs.into_iter().map(Rich::into_owned).collect();
        Error::ParseError(errs, String::from(input))
    })
}

/// The whole query: a sort alone, or a filter followed by an optional
/// sort.
fn query<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
    let sort_only = order_by().map(|sort| SearchEmailsQuery {
        filter: None,
        sort: Some(sort),
    });

    let filtered = filter::parser::query()
        .then(order_by().or_not())
        .map(|(filter, sort)| SearchEmailsQuery {
            filter: Some(filter),
            sort,
        });

    choice((sort_only, filtered)).then_ignore(end())
}

/// A whole word of the grammar, refused when the word read differs, so a
/// longer word holding it (`orders`) or a prefix of it (`ord`) is no
/// match and the diagnostic names `label` over the whole word.
pub(crate) fn keyword<'a>(
    word: &'static str,
    label: &'static str,
) -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    word_chars()
        .try_map(move |found: &str, span| {
            if found == word {
                Ok(())
            } else {
                Err(mismatch(span))
            }
        })
        .labelled(label)
}

/// A run of ASCII letters, digits and underscores, the characters ending it
/// left unnamed.
fn word_chars<'a>() -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone {
    any()
        .filter(|c: &char| is_word_char(*c))
        .labelled(hidden())
        .repeated()
        .at_least(1)
        .to_slice()
}

/// Whether `c` belongs to a word of the grammar.
fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// An error over `span` expecting nothing yet, for a `labelled` parser
/// to name what it expected there.
pub(crate) fn mismatch<'a>(span: SimpleSpan) -> Rich<'a, char> {
    let expected: [DefaultExpected<'a, char>; 0] = [];
    LabelError::<&'a str, _>::expected_found(expected, None, span)
}

/// A label naming nothing: what it labels is left out of the expected
/// list of a diagnostic.
pub(crate) fn hidden<'a>() -> DefaultExpected<'a, char> {
    DefaultExpected::NothingElse
}

/// Any number of spaces, none of them named by a diagnostic.
pub(crate) fn blanks<'a>() -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    just(' ').labelled(hidden()).repeated()
}

/// At least one space, the first one named `label` when it is missing.
pub(crate) fn blanks1<'a>(
    label: &'static str,
) -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    just(' ').labelled(label).ignore_then(blanks())
}

#[cfg(test)]
mod tests {
    use crate::email::search::{
        error::Error,
        filter::query::SearchEmailsFilterQuery::*,
        parser::parse,
        query::SearchEmailsQuery,
        sort::query::{SearchEmailsSorterKind, SearchEmailsSorterOrder},
    };

    fn refused(query: &str) -> (String, std::ops::Range<usize>) {
        let Err(Error::ParseError(errs, src)) = parse(query) else {
            panic!("{query} should be refused");
        };
        (src, errs[0].span().into_range())
    }

    #[test]
    fn order_by_inside_a_quoted_pattern_stays_in_the_pattern() {
        for (query, pattern) in [
            ("subject \"border by the sea\"", "border by the sea"),
            ("subject \"in order by date\"", "in order by date"),
            ("subject in\\ order\\ by\\ date", "in order by date"),
        ] {
            assert_eq!(
                parse(query).unwrap(),
                SearchEmailsQuery {
                    filter: Some(Subject(pattern.into())),
                    sort: None,
                },
                "{query}",
            );
        }
    }

    #[test]
    fn order_by_inside_a_word_is_no_sort() {
        let (src, span) = refused("subject reorder by");
        assert_eq!(src, "subject reorder by");
        assert_eq!(span.start, 16);
    }

    #[test]
    fn a_quoted_pattern_before_the_sort_keeps_both() {
        assert_eq!(
            parse("subject \"law and order by friday\" order by date desc").unwrap(),
            SearchEmailsQuery {
                filter: Some(Subject("law and order by friday".into())),
                sort: Some(vec![
                    (
                        SearchEmailsSorterKind::Date,
                        SearchEmailsSorterOrder::Descending
                    )
                        .into()
                ]),
            },
        );
    }

    #[test]
    fn an_empty_sort_is_refused() {
        let (src, span) = refused("from alice order by");
        assert_eq!(span, src.len()..src.len());

        refused("order by");
    }

    #[test]
    fn a_bad_sort_key_is_located_in_the_whole_query() {
        let (src, span) = refused("from alice order by size");
        assert_eq!(src, "from alice order by size");
        assert_eq!(span.start, 20);
    }

    #[test]
    fn an_unterminated_quote_is_refused() {
        let (src, span) = refused("subject \"unterminated");
        assert_eq!(span, src.len()..src.len());
    }

    #[test]
    fn blanks_inside_parentheses_are_insignificant() {
        let expected = parse("not (from bob or from alice)").unwrap();

        for query in [
            "not ( from bob or from alice )",
            "not (from bob or from alice )",
            "not (  from bob or from alice)",
        ] {
            assert_eq!(parse(query).unwrap(), expected, "{query}");
        }
    }
}
