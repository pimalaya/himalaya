//! # Microsoft Graph message list
//!
//! The `msgraph messages list` command, listing one page of a mailbox or
//! of one folder.

use std::fmt;

use anyhow::{Context, Result};
use clap::Parser;
use io_msgraph::v1::{
    page::MsgraphPage,
    query::to_query_pairs,
    rest::users::messages::{MsgraphMessage, MsgraphRecipient, list::MsgraphMessagesListParams},
    send::{MSGRAPH_API_BASE, path_segment, user_path},
};
use pimalaya_cli::{
    clap::args::PageTokenArg,
    listing::{Listing, ListingPage},
    printer::Printer,
};
use schemars::JsonSchema;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};
use url::Url;

use crate::{
    account::context::Account,
    msgraph::{backend::envelope_from, client::MsgraphClient},
    unified::{backend::Backend, envelope::list::envelope_cells},
};

/// List Microsoft Graph messages (`GET /me/messages` or, with
/// `--folder`, `GET /me/mailFolders/{id}/messages`).
///
/// `nextPage` is Graph's next link as Graph gives it: `--page-token`
/// follows it as is, carrying the query, so no other option goes beside
/// it.
///
/// JSON output: `{"messages": [<raw Graph message>...], "@odata.count",
/// "nextPage"}`, `@odata.count` only with `--count`.
#[derive(Debug, Parser)]
pub struct MsgraphMessageListCommand {
    /// Restrict the listing to a folder id or well-known name (e.g.
    /// `inbox`). Lists the whole mailbox when omitted.
    #[arg(short = 'f', long, value_name = "ID", conflicts_with = "page_token")]
    pub folder: Option<String>,
    /// Maximum number of messages to return (OData `$top`).
    #[arg(long, value_name = "N", conflicts_with = "page_token")]
    pub top: Option<u32>,
    /// Number of messages to skip (OData `$skip`).
    #[arg(long, value_name = "N", conflicts_with = "page_token")]
    pub skip: Option<u32>,
    #[command(flatten)]
    pub page_token: PageTokenArg,
    /// OData `$filter` expression (e.g. `isRead eq false`).
    #[arg(long, value_name = "EXPR", conflicts_with = "page_token")]
    pub filter: Option<String>,
    /// OData `$search` query, `subject:report` or a bare term, quoted for
    /// Graph unless it opens on a double quote.
    ///
    /// Graph forbids `$search` beside `$orderby` and ignores `$count`, so
    /// both are dropped here and the results come back in relevance
    /// order.
    #[arg(long, value_name = "QUERY", conflicts_with = "page_token")]
    pub search: Option<String>,
    /// OData `$orderby` expression. Defaults to `receivedDateTime desc`.
    #[arg(long, value_name = "EXPR", conflicts_with = "page_token")]
    pub orderby: Option<String>,
    /// OData `$select`: comma-separated fields to return (e.g.
    /// `subject,from,receivedDateTime`).
    #[arg(long, value_name = "FIELDS", conflicts_with = "page_token")]
    pub select: Option<String>,
    /// Request the total count of matching messages (OData `$count`).
    #[arg(long, conflicts_with = "page_token")]
    pub count: bool,
}

impl MsgraphMessageListCommand {
    /// Lists one page of messages and prints it.
    ///
    /// A search drops `$orderby`, which Graph rejects beside `$search`, and
    /// `$count`, which there wants a `ConsistencyLevel` header not sent.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut MsgraphClient,
    ) -> Result<()> {
        let quoted = self.search.as_deref().map(quote_search);
        let search = quoted.as_deref();

        let orderby = match search {
            Some(_) => None,
            None => Some(self.orderby.as_deref().unwrap_or("receivedDateTime desc")),
        };
        let count = match search {
            Some(_) => None,
            None => self.count.then_some(true),
        };

        let params = MsgraphMessagesListParams {
            top: self.top,
            skip: self.skip,
            select: self.select.as_deref(),
            filter: self.filter.as_deref(),
            search,
            orderby,
            count,
            ..Default::default()
        };

        let token = self.page_token.token.as_deref();
        let response = match token {
            Some(link) => resume(client, link)?,
            None => {
                let path = match self.folder.as_deref() {
                    Some(folder) => format!("mailFolders/{}/messages", path_segment(folder)?),
                    None => String::from("messages"),
                };
                raw_page(client, &path, &to_query_pairs(&params))?
            }
        };

        let skip = token.map_or(self.skip, skip_of);
        let next_page = next_page(response.next_link, skip);
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
    /// The messages of the page, each as Graph sent it, every property it
    /// carries kept.
    #[schemars(with = "Vec<Map<String, Value>>")]
    messages: Vec<Value>,
    /// How many messages match, Graph's own key, sent with `--count` and
    /// absent otherwise.
    #[serde(rename = "@odata.count", skip_serializing_if = "Option::is_none")]
    count: Option<u64>,
    /// Graph's next link, `null` once the listing is complete, given back
    /// through `--page-token`.
    next_page: Option<String>,
}

impl fmt::Display for MsgraphMessageListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Msgraph.as_str()));

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
            let message: MsgraphMessage =
                serde_json::from_value(message.clone()).unwrap_or_default();
            let envelope = envelope_from(message);
            let fmt = &self.datetime_fmt;
            let cells = envelope_cells(&envelope, fmt, self.datetime_local_tz);
            listing.items.push(cells);
        }

        let count = self.messages.len();
        let next = self.next_page.as_deref();
        listing.page(ListingPage::Token, count, ["message", "messages"], next);

        write!(f, "{listing}")
    }
}

/// One page of the collection at `path` under the mailbox owner, queried
/// by the OData `params` (`to_query_pairs`), its items kept as Graph sent
/// them.
pub(crate) fn raw_page(
    client: &mut MsgraphClient,
    path: &str,
    params: &[(String, String)],
) -> Result<MsgraphPage<Value>> {
    let user = user_path(&client.user_id)?;
    let mut url = Url::parse(MSGRAPH_API_BASE)?.join(&format!("{user}/{path}"))?;
    if !params.is_empty() {
        url.query_pairs_mut().extend_pairs(params);
    }

    Ok(client.page_get(url.as_str(), None)?.response)
}

/// The page behind `link`, the next link a Graph listing printed, sent as
/// it is: it carries the query of the listing it resumes. A link off
/// Graph's origin is refused before any request, the bearer token never
/// leaving it.
pub(crate) fn resume<T: DeserializeOwned>(
    client: &mut MsgraphClient,
    link: &str,
) -> Result<MsgraphPage<T>> {
    let page = client.page_get(link, None).with_context(|| {
        format!("Cannot resume a Microsoft Graph listing from page token {link}")
    })?;

    Ok(page.response)
}

/// The cursor of the next page, Graph's next link as Graph gives it,
/// `None` without one or when it leads back to the page at `skip` (a
/// `$top` of 0).
pub(crate) fn next_page(link: Option<String>, skip: Option<u32>) -> Option<String> {
    let link = link?;

    match skip_of(&link) {
        Some(next) if next <= skip.unwrap_or(0) => None,
        _ => Some(link),
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
pub(crate) fn skip_of(link: &str) -> Option<u32> {
    let (_, query) = link.split_once('?')?;

    query.split('&').find_map(|param| {
        let (key, value) = param.split_once('=')?;
        matches!(key, "$skip" | "%24skip").then(|| value.parse().ok())?
    })
}

#[cfg(test)]
mod tests {
    use io_msgraph::v1::client::MsgraphClientStd;
    use pimalaya_cli::{clap::args::PageTokenArg, printer::CapturePrinter};
    use serde_json::{Value, json};

    use crate::{
        account::context::Account,
        http_script::{Script, ok},
        msgraph::{
            client::MsgraphClient,
            messages::list::{MsgraphMessageListCommand, next_page, quote_search, skip_of},
        },
    };

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
        assert_eq!(next_page(link(0), None), None);
        assert_eq!(next_page(link(20), Some(10)), link(20));

        let search = String::from("https://graph.microsoft.com/v1.0/me/messages?%24skiptoken=x");
        assert_eq!(next_page(Some(search.clone()), None), Some(search));
    }

    /// The `msgraph messages list` of `token`, against a Graph server
    /// answering `pages` in order: the JSON it printed, or its failure,
    /// and the requests the server read.
    fn list(token: Option<&str>, pages: &[Value]) -> (anyhow::Result<Value>, Vec<String>) {
        let responses = pages
            .iter()
            .map(|page| ok("application/json", &page.to_string()));
        let (script, requests) = Script::new(responses.collect());
        let mut client =
            MsgraphClient::over(MsgraphClientStd::new(script, "token", Default::default()));

        let command = MsgraphMessageListCommand {
            folder: None,
            top: Some(1),
            skip: None,
            page_token: PageTokenArg {
                token: token.map(String::from),
            },
            filter: None,
            search: Some(String::from("report")),
            orderby: None,
            select: None,
            count: false,
        };
        let mut printer = CapturePrinter::default();
        let result = command
            .execute(&mut printer, &mut Account::default(), &mut client)
            .and_then(|()| printer.into_single());

        let requests = requests.lock().unwrap().clone();
        (result, requests)
    }

    /// A listing prints Graph's next link as `nextPage`, a `$search` one
    /// carrying no `$skip` included, and `--page-token` follows it as is,
    /// the query it carries kept; a link off Graph's origin is refused
    /// before any request.
    #[test]
    fn a_listing_resumes_from_the_next_link_it_printed() {
        let first = "https://graph.microsoft.com/v1.0/me/messages?%24search=%22report%22&%24top=1&%24skiptoken=a";
        let second = "https://graph.microsoft.com/v1.0/me/messages?%24search=%22report%22&%24top=1&%24skiptoken=b";
        let page = |id: &str, next: Option<&str>| json!({"value": [{"id": id, "importance": "high"}], "@odata.nextLink": next});

        let (printed, requests) = list(None, &[page("m1", Some(first))]);
        let printed = printed.unwrap();
        assert_eq!(printed["nextPage"], first);
        assert_eq!(printed["messages"][0]["importance"], "high");
        assert!(printed.get("@odata.nextLink").is_none(), "{printed}");
        assert!(requests[0].contains("%24search"), "{}", requests[0]);

        let (printed, requests) = list(Some(first), &[page("m2", Some(second))]);
        let printed = printed.unwrap();
        assert_eq!(printed["messages"][0]["id"], "m2");
        assert_eq!(printed["nextPage"], second);
        let line = format!(
            "GET {} ",
            first.trim_start_matches("https://graph.microsoft.com")
        );
        assert!(requests[0].starts_with(&line), "{}", requests[0]);

        let (printed, _) = list(Some(second), &[page("m3", None)]);
        assert_eq!(printed.unwrap()["nextPage"], Value::Null);

        let (refused, requests) = list(Some("https://example.org/v1.0/me/messages"), &[]);
        let err = format!("{:#}", refused.unwrap_err());
        assert!(err.contains("page token"), "{err}");
        assert!(requests.is_empty(), "{requests:#?}");
    }

    /// The query a next link carries is not given again beside it.
    #[test]
    fn a_query_flag_beside_a_page_token_is_refused() {
        use clap::{Parser, error::ErrorKind};

        use crate::cli::Cli;

        let list = [
            "himalaya",
            "msgraph",
            "messages",
            "list",
            "--page-token",
            "x",
        ];
        for flag in [
            &["--top", "1"][..],
            &["--select", "id"],
            &["--filter", "isRead eq false"],
            &["-f", "inbox"],
            &["--count"],
        ] {
            let err = Cli::try_parse_from([&list[..], flag].concat()).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::ArgumentConflict, "{flag:?}");
        }

        assert!(Cli::try_parse_from(list).is_ok());

        let folders = ["himalaya", "msgraph", "mail-folders"];
        let token = ["--page-token", "x"];
        let refused = [
            &["list", "--top", "1"][..],
            &["list", "--select", "id"],
            &["child-folders", "inbox"],
            &["child-folders", "--hidden"],
        ];
        for args in refused {
            let args = [&folders[..], args, &token].concat();
            let err = Cli::try_parse_from(&args).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::ArgumentConflict, "{args:?}");
        }

        let child = [&folders[..], &["child-folders"], &token].concat();
        assert!(Cli::try_parse_from(child).is_ok());
        let bare = [&folders[..], &["child-folders"]].concat();
        let err = Cli::try_parse_from(bare).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn the_skip_of_a_next_link_is_read_encoded_or_not() {
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
