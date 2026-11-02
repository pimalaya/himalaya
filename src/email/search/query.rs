//! # Search query
//!
//! Exposes [`SearchEmailsQuery`], a filter and a sort parsed from one
//! string by [`super::parser`].

use std::str::FromStr;

use crate::{
    capability::SearchOption,
    email::search::{
        error::Error,
        filter::query::SearchEmailsFilterQuery,
        parser,
        sort::query::{SearchEmailsSortQuery, SearchEmailsSorter, SearchEmailsSorterKind},
    },
};

/// A search query, a valid one carrying at least one of its two halves.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SearchEmailsQuery {
    /// Which emails the query keeps.
    pub filter: Option<SearchEmailsFilterQuery>,
    /// In which order it returns them.
    pub sort: Option<SearchEmailsSortQuery>,
}

impl SearchEmailsQuery {
    /// The clauses and sort keys the query uses, each once, as the
    /// options of `envelope.search` a backend declares.
    pub fn options(&self) -> Vec<SearchOption> {
        let mut options = Vec::new();

        if let Some(filter) = &self.filter {
            filter_options(filter, &mut options);
        }

        for SearchEmailsSorter(kind, _) in self.sort.iter().flatten() {
            let option = match kind {
                SearchEmailsSorterKind::Date => SearchOption::SortDate,
                SearchEmailsSorterKind::From => SearchOption::SortFrom,
                SearchEmailsSorterKind::To => SearchOption::SortTo,
                SearchEmailsSorterKind::Subject => SearchOption::SortSubject,
            };
            push_once(&mut options, option);
        }

        options
    }
}

/// Collects the clauses of a filter into `options`.
fn filter_options(filter: &SearchEmailsFilterQuery, options: &mut Vec<SearchOption>) {
    use SearchEmailsFilterQuery as Q;

    let option = match filter {
        Q::And(left, right) | Q::Or(left, right) => {
            filter_options(left, options);
            filter_options(right, options);
            return;
        }
        Q::Not(inner) => return filter_options(inner, options),
        Q::Date(_) => SearchOption::Date,
        Q::AfterDate(_) => SearchOption::After,
        Q::BeforeDate(_) => SearchOption::Before,
        Q::From(_) => SearchOption::From,
        Q::To(_) => SearchOption::To,
        Q::Cc(_) => SearchOption::Cc,
        Q::Bcc(_) => SearchOption::Bcc,
        Q::Address(_) => SearchOption::Address,
        Q::Subject(_) => SearchOption::Subject,
        Q::Body(_) => SearchOption::Body,
        Q::Flag(_) => SearchOption::Flag,
        Q::Attachment => SearchOption::Attachment,
    };

    push_once(options, option);
}

/// Pushes `option` unless `options` holds it already.
fn push_once(options: &mut Vec<SearchOption>, option: SearchOption) {
    if !options.contains(&option) {
        options.push(option);
    }
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

#[cfg(test)]
mod options_tests {
    use crate::{capability::SearchOption, email::search::query::SearchEmailsQuery};

    #[test]
    fn a_query_names_each_option_it_uses_once() {
        let query: SearchEmailsQuery = "not (from a or attachment) and from b order by date"
            .parse()
            .unwrap();

        assert_eq!(
            query.options(),
            [
                SearchOption::From,
                SearchOption::Attachment,
                SearchOption::SortDate
            ],
        );
    }
}
