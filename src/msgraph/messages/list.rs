//! # Microsoft Graph message list
//!
//! The `msgraph messages list` command, listing one page of a mailbox or
//! of one folder.

use std::fmt;

use anyhow::{Result, anyhow};
use clap::Parser;
use io_msgraph::v1::rest::users::messages::{
    MsgraphMessage, MsgraphRecipient, list::MsgraphMessagesListParams,
};
use pimalaya_cli::{clap::args::PageTokenArg, printer::Printer};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    listing::{Listing, ListingPage},
    msgraph::{backend::envelope_from, client::MsgraphClient},
    shared::envelope::list::envelope_cells,
};

/// List Microsoft Graph messages (`GET /me/messages` or, with
/// `--folder`, `GET /me/mailFolders/{id}/messages`).
#[derive(Debug, Parser)]
pub struct MsgraphMessageListCommand {
    /// Restrict the listing to a folder id or well-known name (e.g.
    /// `inbox`). Lists the whole mailbox when omitted.
    #[arg(short = 'f', long, value_name = "ID")]
    pub folder: Option<String>,
    /// Maximum number of messages to return (OData `$top`).
    #[arg(long, value_name = "N")]
    pub top: Option<u32>,
    /// Number of messages to skip (OData `$skip`).
    #[arg(long, value_name = "N")]
    pub skip: Option<u32>,
    #[command(flatten)]
    pub page_token: PageTokenArg,
    /// OData `$filter` expression (e.g. `isRead eq false`).
    #[arg(long, value_name = "EXPR")]
    pub filter: Option<String>,
    /// OData `$search` query, `subject:report` or a bare term, quoted for
    /// Graph unless it opens on a double quote.
    ///
    /// Graph forbids `$search` beside `$orderby` and ignores `$count`, so
    /// both are dropped here and the results come back in relevance
    /// order.
    #[arg(long, value_name = "QUERY")]
    pub search: Option<String>,
    /// OData `$orderby` expression. Defaults to `receivedDateTime desc`.
    #[arg(long, value_name = "EXPR")]
    pub orderby: Option<String>,
    /// OData `$select`: comma-separated fields to return (e.g.
    /// `subject,from,receivedDateTime`).
    #[arg(long, value_name = "FIELDS")]
    pub select: Option<String>,
    /// Request the total count of matching messages (OData `$count`).
    #[arg(long)]
    pub count: bool,
}

impl MsgraphMessageListCommand {
    /// Lists one page of messages and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        let quoted = self.search.as_deref().map(quote_search);
        let search = quoted.as_deref();

        // NOTE: Graph rejects `$search` beside `$orderby`, and pairing it
        // with `$count` wants a `ConsistencyLevel` header the client does
        // not send, so both are dropped when searching.
        let orderby = match search {
            Some(_) => None,
            None => Some(self.orderby.as_deref().unwrap_or("receivedDateTime desc")),
        };
        let count = match search {
            Some(_) => None,
            None => self.count.then_some(true),
        };

        let skip = page_skip(self.page_token.token.as_deref(), self.skip)?;

        let params = MsgraphMessagesListParams {
            top: self.top,
            skip,
            select: self.select.as_deref(),
            filter: self.filter.as_deref(),
            search,
            orderby,
            count,
            ..Default::default()
        };
        let response = client
            .messages_list(self.folder.as_deref(), &params)?
            .response;

        let next_page = next_page_token(response.next_link, skip);
        printer.out(MsgraphMessageListOutput {
            account: account.name().to_string(),
            datetime_fmt: account.datetime_fmt().to_string(),
            datetime_local_tz: account.datetime_local_tz(),
            folder: self.folder,
            search: self.search,
            filter: self.filter,
            messages: response.value,
            count: response.count,
            next_page,
        })
    }
}

/// Renders the display form of a recipient: `Name <addr>`, or just the
/// address when there is no display name.
pub(crate) fn recipient(recipient: &MsgraphRecipient) -> String {
    let address = recipient.email_address.address.clone().unwrap_or_default();
    match &recipient.email_address.name {
        Some(name) if !name.is_empty() => format!("{name} <{address}>"),
        _ => address,
    }
}

/// The `msgraph messages list` output, the messages of one page.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MsgraphMessageListOutput {
    #[serde(skip)]
    account: String,
    #[serde(skip)]
    datetime_fmt: String,
    #[serde(skip)]
    datetime_local_tz: bool,
    #[serde(skip)]
    folder: Option<String>,
    #[serde(skip)]
    search: Option<String>,
    #[serde(skip)]
    filter: Option<String>,
    messages: Vec<MsgraphMessage>,
    /// How many messages match, with `--count`, `null` otherwise.
    count: Option<u64>,
    /// The cursor of the next page, `null` once the listing is complete,
    /// given back through `--page-token`.
    next_page: Option<String>,
}

impl fmt::Display for MsgraphMessageListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Msgraph));

        let heads = [
            ("Folder", &self.folder),
            ("Search", &self.search),
            ("Filter", &self.filter),
        ];

        for (label, value) in heads {
            if let Some(value) = value {
                listing.head(label, value);
            }
        }

        if let Some(count) = self.count {
            listing.head("Count", &count.to_string());
        }

        for message in &self.messages {
            let envelope = envelope_from(message.clone());
            let fmt = &self.datetime_fmt;
            let cells = envelope_cells(&envelope, fmt, self.datetime_local_tz);
            listing.items.push(cells);
        }

        let count = self.messages.len();
        let next = self.next_page.as_deref();
        listing.page(ListingPage::Token, count, ["message", "messages"], next);

        // NOTE: a whole next link is no page token, so it replaces the
        // `Next page:` line `--page-token` would refuse.
        if let Some(link) = next.filter(|next| next.parse::<u32>().is_err()) {
            listing.header.pop();
            listing.head("Next link", link);
        }

        write!(f, "{listing}")
    }
}

/// The `$skip` a listing resumes from: the page token, the `$skip` of the
/// next link Graph returned, so a listing resumes with the same
/// parameters, else `skip`.
pub(crate) fn page_skip(token: Option<&str>, skip: Option<u32>) -> Result<Option<u32>> {
    let Some(token) = token else {
        return Ok(skip);
    };

    let skip = token
        .parse::<u32>()
        .map_err(|_| anyhow!("Cannot resume a Microsoft Graph listing from page token {token}"))?;

    Ok(Some(skip))
}

/// The page token of a Graph next link, its `$skip`, `None` without a
/// link or when it leads back to the page at `skip` (a `$top` of 0).
///
/// A next link without one (a `$search` page, resumed by `$skiptoken`)
/// comes back whole, which `--page-token` refuses.
pub(crate) fn next_page_token(link: Option<String>, skip: Option<u32>) -> Option<String> {
    let link = link?;

    match skip_of(&link) {
        Some(next) if next <= skip.unwrap_or(0) => None,
        Some(next) => Some(next.to_string()),
        None => Some(link),
    }
}

/// Quotes a `$search` value as Graph wants it, unless already quoted:
/// unquoted, a `:` or `-` is a syntax error.
fn quote_search(search: &str) -> String {
    if search.starts_with('"') {
        return search.to_string();
    }

    format!("\"{}\"", search.replace('"', "\\\""))
}

/// The `$skip` query parameter of a Graph next link, percent-encoded or
/// not.
fn skip_of(link: &str) -> Option<u32> {
    let (_, query) = link.split_once('?')?;

    query.split('&').find_map(|param| {
        let (key, value) = param.split_once('=')?;
        matches!(key, "$skip" | "%24skip").then(|| value.parse().ok())?
    })
}

#[cfg(test)]
mod tests {
    use crate::msgraph::messages::list::{next_page_token, quote_search, skip_of};

    #[test]
    fn a_raw_search_is_quoted_once() {
        assert_eq!(quote_search("subject:report"), r#""subject:report""#);
        assert_eq!(quote_search(r#"a "b""#), r#""a \"b\"""#);
        assert_eq!(quote_search(r#""subject:x""#), r#""subject:x""#);
    }

    #[test]
    fn a_next_link_leading_back_is_no_next_page() {
        let link = |skip: u32| {
            Some(format!(
                "https://graph.microsoft.com/v1.0/me/messages?%24top=0&%24skip={skip}"
            ))
        };
        assert_eq!(next_page_token(link(0), None), None);
        assert_eq!(next_page_token(link(20), Some(10)).as_deref(), Some("20"));

        let search = String::from("https://graph.microsoft.com/v1.0/me/messages?%24skiptoken=x");
        assert_eq!(next_page_token(Some(search.clone()), None), Some(search));
    }

    #[test]
    fn the_skip_of_a_next_link_is_its_page_token() {
        let link = "https://graph.microsoft.com/v1.0/me/messages?%24top=10&%24skip=20";
        assert_eq!(skip_of(link), Some(20));
        assert_eq!(
            skip_of("https://graph.microsoft.com/v1.0/me/messages?$skip=5"),
            Some(5)
        );
        assert_eq!(
            skip_of("https://graph.microsoft.com/v1.0/me/messages?$skiptoken=x"),
            None
        );
    }
}
