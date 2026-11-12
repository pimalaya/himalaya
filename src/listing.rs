//! # Listing
//!
//! The text of every list: the fields placing it, the account, what it lists
//! and the page it holds, then one item per line through [`Columns`], with
//! no header row.

use std::fmt;

use pimalaya_cli::{columns::Columns, sanitize};

use crate::backend::Backend;

/// A list printed one item per line under the fields placing it.
#[derive(Clone, Debug, Default)]
pub struct Listing {
    /// The fields above the items: the account, what is listed, then the
    /// page and what else places the answer.
    pub header: Vec<(&'static str, String)>,
    /// The items, one per line, with no header row.
    pub items: Columns<'static>,
    /// The line printed in place of the items when there are none, the
    /// header standing alone otherwise.
    pub empty: Option<&'static str>,
}

/// The page a listing holds.
#[cfg(any(backend, feature = "smtp"))]
#[derive(Clone, Copy, Debug)]
pub enum ListingPage {
    /// The whole listing, unpaged.
    All,
    /// A page of a listing paged by number.
    Numbered { page: u64, size: u64 },
    /// A page of a listing reached by an opaque token.
    #[cfg(any(feature = "gmail", feature = "msgraph", feature = "pimdir"))]
    Token,
}

impl Listing {
    /// A listing headed by the account, the backend answering in
    /// parentheses.
    pub fn new(account: &str, backend: Option<Backend>) -> Self {
        let mut value = sanitize::text(account).into_owned();

        if let Some(backend) = backend {
            value.push_str(&format!(" ({backend})"));
        }

        Self {
            header: vec![("Account", value)],
            ..Default::default()
        }
    }

    /// Adds a field above the items, its value sanitized.
    #[cfg(any(backend, feature = "smtp"))]
    pub fn head(&mut self, label: &'static str, value: &str) {
        let value = sanitize::text(value).into_owned();
        self.header.push((label, value));
    }

    /// Adds the page the `count` items fill, named by their singular and
    /// plural `nouns`, then the flag reaching the next page while one
    /// follows.
    #[cfg(any(backend, feature = "smtp"))]
    pub fn page(&mut self, page: ListingPage, count: usize, nouns: [&str; 2], next: Option<&str>) {
        let count = count as u64;
        let [one, many] = nouns;

        let (number, first) = match page {
            ListingPage::All => (Some(String::from("all")), Some(1)),
            ListingPage::Numbered { page, size } => {
                let first = page.saturating_sub(1) * size + 1;
                (Some(page.to_string()), Some(first))
            }
            #[cfg(any(feature = "gmail", feature = "msgraph", feature = "pimdir"))]
            ListingPage::Token => (None, None),
        };

        let held = match (count, first) {
            (0, _) => String::from("empty"),
            (1, Some(first)) => format!("{one} {first}"),
            (n, Some(first)) => format!("{many} {first} to {}", first + n - 1),
            (1, None) => format!("1 {one}"),
            (n, None) => format!("{n} {many}"),
        };

        let last = match (page, next) {
            (ListingPage::All, _) | (_, Some(_)) => "",
            _ if count == 0 => "",
            _ => ", the last",
        };

        let value = match number {
            Some(number) => format!("{number}, {held}{last}"),
            None => format!("{held}{last}"),
        };

        self.header.push(("Page", value));

        if let Some(token) = next {
            let flag = page.flag();
            let token = sanitize::text(token);
            self.header.push(("Next page", format!("{flag} {token}")));
        }
    }
}

#[cfg(any(backend, feature = "smtp"))]
impl ListingPage {
    /// The flag reaching another page.
    fn flag(self) -> &'static str {
        match self {
            Self::All | Self::Numbered { .. } => "--page",
            #[cfg(any(feature = "gmail", feature = "msgraph", feature = "pimdir"))]
            Self::Token => "--page-token",
        }
    }
}

impl fmt::Display for Listing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_fields(f, &self.header)?;

        let body = match (self.items.rows.is_empty(), self.empty) {
            (false, _) => self.items.to_string(),
            (true, Some(empty)) => format!("{empty}\n"),
            (true, None) => return Ok(()),
        };

        if !self.header.is_empty() {
            writeln!(f)?;
        }

        f.write_str(&body)
    }
}

/// The unread and total counts of a mailbox as one cell, `3/120`, a count
/// the backend gave none of as `-`, nothing when it gave neither.
#[cfg(any(backend, feature = "smtp"))]
pub fn counts(unread: Option<u64>, total: Option<u64>) -> String {
    let count = |n: Option<u64>| n.map_or_else(|| String::from("-"), |n| n.to_string());

    match (unread, total) {
        (None, None) => String::new(),
        (unread, total) => format!("{}/{}", count(unread), count(total)),
    }
}

/// Writes `Label:  value` lines, values in one column on every list: two
/// spaces after the longest label, `Next page:`.
fn write_fields(f: &mut fmt::Formatter<'_>, fields: &[(&str, String)]) -> fmt::Result {
    const WIDTH: usize = "Next page:".len() + 2;

    for (label, value) in fields {
        let label = format!("{label}:");
        writeln!(f, "{label:<WIDTH$}{value}")?;
    }

    Ok(())
}

#[cfg(all(test, any(feature = "gmail", feature = "msgraph", feature = "pimdir")))]
mod tests {
    use crate::{
        backend::Backend,
        listing::{Listing, ListingPage},
    };

    fn listing(rows: usize) -> Listing {
        let mut listing = Listing::new("work", Some(Backend::Gmail));

        for i in 0..rows {
            listing.items.push([i.to_string(), String::from("x")]);
        }

        listing
    }

    /// A page reached by a token counts what it holds, and names the flag
    /// reaching the next, both above the items.
    #[test]
    fn a_token_page_counts_its_items() {
        let mut first = listing(2);
        first.page(ListingPage::Token, 2, ["draft", "drafts"], Some("abc"));

        let expected = "\
Account:    work (gmail)
Page:       2 drafts
Next page:  --page-token abc

0  x
1  x
";
        assert_eq!(first.to_string(), expected);

        let mut last = listing(1);
        last.page(ListingPage::Token, 1, ["draft", "drafts"], None);
        assert!(
            last.to_string()
                .contains("\nPage:       1 draft, the last\n\n0  x\n")
        );

        let mut empty = listing(0);
        empty.page(ListingPage::Token, 0, ["draft", "drafts"], None);

        let expected = "\
Account:    work (gmail)
Page:       empty
";
        assert_eq!(empty.to_string(), expected);
    }

    /// An unpaged listing ends on its last item, and one without an
    /// account prints its items alone.
    #[test]
    fn a_listing_without_a_page_ends_on_its_items() {
        let mut listing = listing(1);
        listing.head("Mailbox", "INBOX");

        let expected = "\
Account:    work (gmail)
Mailbox:    INBOX

0  x
";
        assert_eq!(listing.to_string(), expected);

        let mut bare = Listing::default();
        bare.items.push(["a", "b"]);
        assert_eq!(bare.to_string(), "a  b\n");
    }
}
