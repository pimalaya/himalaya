//! # Search query parser
//!
//! Entry point parsing a whole search query from a string, deferring the
//! grammar to [`super::filter::parser`] and [`super::sort::parser`], and
//! the lexical pieces both halves share.

use chumsky::{DefaultExpected, error::LabelError, prelude::*};

use crate::email::search::{
    error::Error, filter, query::SearchEmailsQuery, sort::parser::order_by,
};

/// The deepest parentheses may nest in a query.
pub const MAX_NESTING: usize = 32;

/// The most `and`, `or` and `not` a query may hold.
pub const MAX_OPERATORS: usize = 256;

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

    if let Some(err) = past_bounds(input) {
        return Err(Error::ParseError(vec![err], String::from(input)));
    }

    query().parse(input).into_result().map_err(|errs| {
        let errs = errs.into_iter().map(Rich::into_owned).collect();
        Error::ParseError(errs, String::from(input))
    })
}

/// The error over the first parenthesis nesting deeper than
/// [`MAX_NESTING`] or the first operator past [`MAX_OPERATORS`], read
/// before the grammar recurses into the query or builds a tree the
/// backends walk.
///
/// Quoted text and backslash-escaped characters are skipped, as the
/// grammar reads them as patterns; a bare `and`, `or` or `not` counts
/// wherever it stands.
fn past_bounds(input: &str) -> Option<Rich<'static, char>> {
    let mut depth = 0usize;
    let mut operators = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    let mut word = 0..0;

    for (index, c) in input.char_indices().chain([(input.len(), ' ')]) {
        if is_word_char(c) && !quoted && !escaped {
            if word.is_empty() {
                word = index..index;
            }
            word.end = index + c.len_utf8();
            continue;
        }

        if matches!(&input[word.clone()], "and" | "or" | "not") {
            operators += 1;
            if operators > MAX_OPERATORS {
                let reason = format!("more than {MAX_OPERATORS} `and`, `or` and `not`");
                return Some(Rich::custom(word.into(), reason));
            }
        }
        word = index..index;

        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '"' => quoted = !quoted,
            '(' if !quoted => {
                depth += 1;
                if depth > MAX_NESTING {
                    let reason = format!("parentheses nested deeper than {MAX_NESTING} levels");
                    return Some(Rich::custom((index..index + 1).into(), reason));
                }
            }
            ')' if !quoted => depth = depth.saturating_sub(1),
            _ => (),
        }
    }

    None
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
pub(crate) fn word_chars<'a>() -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone {
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
        parser::{MAX_NESTING, MAX_OPERATORS, parse},
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

    /// `levels` parentheses around one condition.
    fn nested(levels: usize) -> String {
        format!("{}from a{}", "(".repeat(levels), ")".repeat(levels))
    }

    #[test]
    fn nesting_at_the_bound_parses() {
        assert!(parse(nested(MAX_NESTING)).is_ok());
    }

    /// Deep nesting is refused before the grammar recurses into it, where
    /// 3,000 levels overflowed the stack.
    #[test]
    fn nesting_past_the_bound_is_refused() {
        refused(&nested(3_000));
        let (_, span) = refused(&nested(MAX_NESTING + 1));
        assert_eq!(span, MAX_NESTING..MAX_NESTING + 1);
    }

    /// A chain of operators is a tree as deep as it is long, which the
    /// backends then walk: it is bounded too.
    #[test]
    fn operators_past_the_bound_are_refused() {
        let chain =
            |op: &str, count: usize| format!("{}from a", format!("from a {op} ").repeat(count));
        refused(&chain("or", 100_000));
        refused(&format!("{}from a", "not ".repeat(100_000)));
        assert!(parse(chain("and", MAX_OPERATORS)).is_ok());
        refused(&chain("and", MAX_OPERATORS + 1));
        assert!(parse(format!("subject \"{}\"", "a and ".repeat(1_000))).is_ok());
    }
}
