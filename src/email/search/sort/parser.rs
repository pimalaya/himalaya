//! # Search emails sort query string parser
//!
//! Parsers needed to build a [`SearchEmailsSorter`] chain from a string
//! slice. Based on [`chumsky`].
//!
//! [`SearchEmailsSorter`]: crate::email::search::sort::query::SearchEmailsSorter

use chumsky::prelude::*;

use crate::email::search::{
    parser::ParserError,
    sort::query::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
};

/// The emails search sort query string parser.
///
/// A sort query string is a list of sorters separated by spaces. Each
/// sorter is a kind (`date`, `from`, `to`, `subject`) followed by an
/// optional order (`asc`, `desc`; ascending when omitted).
pub fn query<'a>() -> impl Parser<'a, &'a str, Vec<SearchEmailsSorter>, ParserError<'a>> + Clone {
    choice((date(), from(), to(), subject()))
        .separated_by(
            just(' ')
                .labelled("space between sorters")
                .repeated()
                .at_least(1),
        )
        .collect()
}

fn date<'a>() -> impl Parser<'a, &'a str, SearchEmailsSorter, ParserError<'a>> + Clone {
    choice((
        date_kind()
            .then(
                just(' ')
                    .labelled("space after `date`")
                    .repeated()
                    .at_least(1)
                    .ignore_then(choice((ascending(), descending()))),
            )
            .map(SearchEmailsSorter::from),
        date_kind().map(SearchEmailsSorter::from),
    ))
}

fn date_kind<'a>() -> impl Parser<'a, &'a str, SearchEmailsSorterKind, ParserError<'a>> + Clone {
    just('d')
        .labelled("`date`")
        .ignored()
        .then_ignore(just('a').labelled("`date`"))
        .then_ignore(just('t').labelled("`date`"))
        .then_ignore(just('e').labelled("`date`"))
        .to(SearchEmailsSorterKind::Date)
}

fn from<'a>() -> impl Parser<'a, &'a str, SearchEmailsSorter, ParserError<'a>> + Clone {
    choice((
        from_kind()
            .then(
                just(' ')
                    .labelled("space after `from`")
                    .repeated()
                    .at_least(1)
                    .ignore_then(choice((ascending(), descending()))),
            )
            .map(SearchEmailsSorter::from),
        from_kind().map(SearchEmailsSorter::from),
    ))
}

fn from_kind<'a>() -> impl Parser<'a, &'a str, SearchEmailsSorterKind, ParserError<'a>> + Clone {
    just('f')
        .labelled("`from`")
        .ignored()
        .then_ignore(just('r').labelled("`from`"))
        .then_ignore(just('o').labelled("`from`"))
        .then_ignore(just('m').labelled("`from`"))
        .to(SearchEmailsSorterKind::From)
}

fn to<'a>() -> impl Parser<'a, &'a str, SearchEmailsSorter, ParserError<'a>> + Clone {
    choice((
        to_kind()
            .then(
                just(' ')
                    .labelled("space after `to`")
                    .repeated()
                    .at_least(1)
                    .ignore_then(choice((ascending(), descending()))),
            )
            .map(SearchEmailsSorter::from),
        to_kind().map(SearchEmailsSorter::from),
    ))
}

fn to_kind<'a>() -> impl Parser<'a, &'a str, SearchEmailsSorterKind, ParserError<'a>> + Clone {
    just('t')
        .labelled("`to`")
        .ignored()
        .then_ignore(just('o').labelled("`to`"))
        .to(SearchEmailsSorterKind::To)
}

fn subject<'a>() -> impl Parser<'a, &'a str, SearchEmailsSorter, ParserError<'a>> + Clone {
    choice((
        subject_kind()
            .then(
                just(' ')
                    .labelled("space after `subject`")
                    .repeated()
                    .at_least(1)
                    .ignore_then(choice((ascending(), descending()))),
            )
            .map(SearchEmailsSorter::from),
        subject_kind().map(SearchEmailsSorter::from),
    ))
}

fn subject_kind<'a>() -> impl Parser<'a, &'a str, SearchEmailsSorterKind, ParserError<'a>> + Clone {
    just('s')
        .labelled("`subject`")
        .ignored()
        .then_ignore(just('u').labelled("`subject`"))
        .then_ignore(just('b').labelled("`subject`"))
        .then_ignore(just('j').labelled("`subject`"))
        .then_ignore(just('e').labelled("`subject`"))
        .then_ignore(just('c').labelled("`subject`"))
        .then_ignore(just('t').labelled("`subject`"))
        .to(SearchEmailsSorterKind::Subject)
}

fn ascending<'a>() -> impl Parser<'a, &'a str, SearchEmailsSorterOrder, ParserError<'a>> + Clone {
    just('a')
        .labelled("`asc`")
        .ignore_then(just('s').labelled("`asc`"))
        .ignore_then(just('c').labelled("`asc`"))
        .to(SearchEmailsSorterOrder::Ascending)
}

fn descending<'a>() -> impl Parser<'a, &'a str, SearchEmailsSorterOrder, ParserError<'a>> + Clone {
    just('d')
        .labelled("`desc`")
        .ignore_then(just('e').labelled("`desc`"))
        .ignore_then(just('s').labelled("`desc`"))
        .ignore_then(just('c').labelled("`desc`"))
        .to(SearchEmailsSorterOrder::Descending)
}

#[cfg(test)]
mod tests {
    use chumsky::prelude::*;

    use super::{SearchEmailsSorter, SearchEmailsSorterKind::*, SearchEmailsSorterOrder::*};

    #[test]
    fn empty_sorter() {
        assert_eq!(super::query().parse("").into_result(), Ok(vec![]));
    }

    #[test]
    fn simple_sorters() {
        assert_eq!(
            super::query().parse("date from to").into_result(),
            Ok(vec![
                SearchEmailsSorter(Date, Ascending),
                SearchEmailsSorter(From, Ascending),
                SearchEmailsSorter(To, Ascending)
            ])
        );
    }

    #[test]
    fn mixed_sorters() {
        assert_eq!(
            super::query()
                .parse("date asc from subject desc")
                .into_result(),
            Ok(vec![
                SearchEmailsSorter(Date, Ascending),
                SearchEmailsSorter(From, Ascending),
                SearchEmailsSorter(Subject, Descending)
            ])
        );
    }
}
