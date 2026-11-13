//! # Search sort parser
//!
//! The chumsky grammar building a [`SearchEmailsSorter`] chain from a
//! string.
//!
//! [`SearchEmailsSorter`]: crate::email::search::sort::query::SearchEmailsSorter

use chumsky::prelude::*;

use crate::email::search::{
    parser::{ParserError, blanks1, hidden, keyword},
    sort::query::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
};

/// Parses the sort half of a query: the words `order by`, then at least
/// one sorter.
pub fn order_by<'a>() -> impl Parser<'a, &'a str, Vec<SearchEmailsSorter>, ParserError<'a>> + Clone
{
    keyword("order", "`order by`")
        .ignore_then(blanks1("space after `order`"))
        .ignore_then(keyword("by", "`by` after `order`"))
        .ignore_then(blanks1("sort key after `order by`"))
        .ignore_then(query())
}

/// Parses a sort: space-separated sorters, each a key followed by an
/// optional direction, ascending when omitted.
pub fn query<'a>() -> impl Parser<'a, &'a str, Vec<SearchEmailsSorter>, ParserError<'a>> + Clone {
    let kind = choice((
        keyword("date", "`date`").to(SearchEmailsSorterKind::Date),
        keyword("from", "`from`").to(SearchEmailsSorterKind::From),
        keyword("to", "`to`").to(SearchEmailsSorterKind::To),
        keyword("subject", "`subject`").to(SearchEmailsSorterKind::Subject),
    ));

    // The spaces before a direction, unnamed: a key may end the sort.
    let blank = just(' ').labelled(hidden()).repeated().at_least(1);

    let order = choice((
        keyword("asc", "`asc`").to(SearchEmailsSorterOrder::Ascending),
        keyword("desc", "`desc`").to(SearchEmailsSorterOrder::Descending),
    ));

    kind.then(blank.ignore_then(order).or_not())
        .map(SearchEmailsSorter::from)
        .separated_by(blanks1("space between sort keys"))
        .at_least(1)
        .collect()
}

#[cfg(test)]
mod tests {
    use chumsky::prelude::*;

    use super::{SearchEmailsSorter, SearchEmailsSorterKind::*, SearchEmailsSorterOrder::*};

    #[test]
    fn empty_sorter_is_refused() {
        assert!(super::query().parse("").into_result().is_err());
        assert!(super::order_by().parse("order by").into_result().is_err());
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

    #[test]
    fn a_key_is_a_whole_word() {
        assert!(super::query().parse("dates").into_result().is_err());
        assert!(
            super::query()
                .parse("date ascending")
                .into_result()
                .is_err()
        );
    }
}
