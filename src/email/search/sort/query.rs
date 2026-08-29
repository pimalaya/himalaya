//! # Search sort query
//!
//! Exposes [`SearchEmailsSortQuery`] and friends, the AST
//! [`super::parser::query`] produces.

/// The sort half of a search query, applied left to right, so the first
/// sorter is the primary key.
pub type SearchEmailsSortQuery = Vec<SearchEmailsSorter>;

/// One sorter: a key and a direction.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SearchEmailsSorter(
    /// The property to sort on.
    pub SearchEmailsSorterKind,
    /// The direction to sort in.
    pub SearchEmailsSorterOrder,
);

impl SearchEmailsSorter {
    /// Builds a sorter from a key and a direction.
    pub fn new(kind: SearchEmailsSorterKind, order: SearchEmailsSorterOrder) -> Self {
        Self(kind, order)
    }
}

impl From<(SearchEmailsSorterKind, SearchEmailsSorterOrder)> for SearchEmailsSorter {
    fn from((kind, order): (SearchEmailsSorterKind, SearchEmailsSorterOrder)) -> Self {
        SearchEmailsSorter::new(kind, order)
    }
}

impl From<(SearchEmailsSorterKind, Option<SearchEmailsSorterOrder>)> for SearchEmailsSorter {
    fn from((kind, order): (SearchEmailsSorterKind, Option<SearchEmailsSorterOrder>)) -> Self {
        (kind, order.unwrap_or_default()).into()
    }
}

impl From<SearchEmailsSorterKind> for SearchEmailsSorter {
    fn from(kind: SearchEmailsSorterKind) -> Self {
        (kind, None).into()
    }
}

/// The property a sorter sorts on.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchEmailsSorterKind {
    /// The `Date:` header, the sent-at.
    Date,
    /// The envelope sender.
    From,
    /// The envelope recipient.
    To,
    /// The `Subject:` header.
    Subject,
}

/// The direction a sorter sorts in.
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchEmailsSorterOrder {
    /// Smallest first.
    #[default]
    Ascending,
    /// Greatest first.
    Descending,
}
