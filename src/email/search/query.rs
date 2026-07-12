//! # Search emails query
//!
//! Exposes [`SearchEmailsQuery`], the top-level structure wrapping a
//! [`SearchEmailsFilterQuery`] (filter) and a [`SearchEmailsSortQuery`]
//! (sort). The query parses from a string slice via [`FromStr`]; see
//! [`super::parser`] for the parser entry point and the grammar.

use std::str::FromStr;

use crate::email::search::{
    error::Error, filter::query::SearchEmailsFilterQuery, parser,
    sort::query::SearchEmailsSortQuery,
};

/// The search emails query structure.
///
/// Composed of an optional recursive [`SearchEmailsFilterQuery`] and an
/// optional [`SearchEmailsSortQuery`]. At least one of the two must be
/// present in a valid query string.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SearchEmailsQuery {
    /// The recursive emails search filter query.
    pub filter: Option<SearchEmailsFilterQuery>,

    /// The emails search sort query.
    pub sort: Option<SearchEmailsSortQuery>,
}

impl FromStr for SearchEmailsQuery {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parser::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use crate::email::search::{
        filter::query::SearchEmailsFilterQuery,
        query::SearchEmailsQuery,
        sort::query::{SearchEmailsSorterKind::*, SearchEmailsSorterOrder::*},
    };

    #[test]
    fn filters_only() {
        assert_eq!(
            "from f and to t".parse::<SearchEmailsQuery>().unwrap(),
            SearchEmailsQuery {
                filter: Some(SearchEmailsFilterQuery::And(
                    Box::new(SearchEmailsFilterQuery::From("f".into())),
                    Box::new(SearchEmailsFilterQuery::To("t".into())),
                )),
                sort: None,
            },
        );
    }

    #[test]
    fn sorters_only() {
        assert_eq!(
            "order by from".parse::<SearchEmailsQuery>().unwrap(),
            SearchEmailsQuery {
                filter: None,
                sort: Some(vec![From.into()]),
            },
        );

        assert_eq!(
            "order by from asc subject desc"
                .parse::<SearchEmailsQuery>()
                .unwrap(),
            SearchEmailsQuery {
                filter: None,
                sort: Some(vec![From.into(), (Subject, Descending).into()]),
            },
        );
    }

    #[test]
    fn full() {
        assert_eq!(
            "from f and to t order by from to desc"
                .parse::<SearchEmailsQuery>()
                .unwrap(),
            SearchEmailsQuery {
                filter: Some(SearchEmailsFilterQuery::And(
                    Box::new(SearchEmailsFilterQuery::From("f".into())),
                    Box::new(SearchEmailsFilterQuery::To("t".into())),
                )),
                sort: Some(vec![From.into(), (To, Descending).into()]),
            },
        );
    }
}
