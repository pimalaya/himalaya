//! # Search emails sort query
//!
//! Exposes [`SearchEmailsSortQuery`] and friends, the AST produced by
//! [`super::parser::query`].

/// The search emails sort query.
///
/// Just a list of [`SearchEmailsSorter`]s, applied left-to-right (the
/// first sorter is the primary sort key).
pub type SearchEmailsSortQuery = Vec<SearchEmailsSorter>;

/// A single sorter: a kind plus an order.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SearchEmailsSorter(
    /// The search emails sorter kind.
    pub SearchEmailsSorterKind,
    /// The search emails sorter order.
    pub SearchEmailsSorterOrder,
);

impl SearchEmailsSorter {
    /// Build a sorter from a kind and an order.
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

/// The property a sorter sorts emails on. `Date` resolves to the
/// `Date:` header (sent-at).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchEmailsSorterKind {
    /// Sort emails by message header `Date`.
    Date,

    /// Sort emails by envelope sender.
    From,

    /// Sort emails by envelope recipient.
    To,

    /// Sort emails by message header `Subject`.
    Subject,
}

/// Sort direction. Defaults to ascending.
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchEmailsSorterOrder {
    /// Sort emails by ascending order.
    #[default]
    Ascending,

    /// Sort emails by descending order.
    Descending,
}
