//! # Gmail message list
//!
//! The `gmail messages list` command, `users.messages.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::messages::list::GmailMessagesListParams;
use pimalaya_cli::{
    listing::{Listing, ListingPage},
    printer::Printer,
    sanitize,
};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    account::context::Account,
    gmail::{client::GmailClient, format::str_of},
    unified::backend::Backend,
};

/// List Gmail message ids matching the given query and labels
/// (users.messages.list).
///
/// JSON output: Gmail's page as Gmail sent it, `{"messages": [{"id",
/// "threadId"}...], "resultSizeEstimate", "nextPage"}`, `nextPage`
/// Gmail's `nextPageToken`, `null` on the last page.
#[derive(Debug, Parser)]
pub struct GmailMessagesListCommand {
    /// Gmail search query, using the same syntax as the Gmail search
    /// box (e.g. `from:alice is:unread`).
    #[arg(short = 'q', long)]
    pub query: Option<String>,
    /// Only return messages carrying the given label id. Can be
    /// repeated to require multiple labels.
    #[arg(short = 'l', long = "label", value_name = "ID")]
    pub labels: Vec<String>,
    /// Maximum number of message ids to return.
    #[arg(short = 's', long, value_name = "N")]
    pub max_results: Option<u32>,
    /// Page token returned by a previous listing, to fetch the next
    /// page.
    #[arg(long, value_name = "TOKEN")]
    pub page_token: Option<String>,
    /// Also include messages from SPAM and TRASH.
    #[arg(long)]
    pub include_spam_trash: bool,
}

impl GmailMessagesListCommand {
    /// Lists one page of message ids and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let params = GmailMessagesListParams {
            q: self.query.as_deref(),
            label_ids: &self.labels,
            max_results: self.max_results,
            page_token: self.page_token.as_deref(),
            include_spam_trash: self.include_spam_trash,
        };
        let page = client.raw_list("messages", &params)?;

        printer.out(GmailMessageListOutput {
            account: account.name().to_string(),
            query: self.query,
            labels: self.labels,
            messages: page.items,
            rest: page.rest,
            next_page: page.next_page,
        })
    }
}

/// The `gmail messages list` output, the message ids of one page.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailMessageListOutput {
    /// The name of the account listed.
    #[serde(skip)]
    pub account: String,
    /// The search query, if any.
    #[serde(skip)]
    pub query: Option<String>,
    /// The label ids every message carries.
    #[serde(skip)]
    pub labels: Vec<String>,
    /// The messages of this page, each as Gmail sent it, `{id, threadId}`.
    #[schemars(with = "Vec<Map<String, Value>>")]
    pub messages: Vec<Value>,
    /// The rest of the page as Gmail sent it, such as
    /// `resultSizeEstimate`.
    #[serde(flatten)]
    pub rest: Map<String, Value>,
    /// The cursor of the next page, `null` once the listing is complete,
    /// given back through `--page-token`.
    pub next_page: Option<String>,
}

impl fmt::Display for GmailMessageListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Gmail.as_str()));

        if !self.labels.is_empty() {
            listing.head("Labels", &self.labels.join(", "));
        }

        if let Some(query) = &self.query {
            listing.head("Query", query);
        }

        for message in &self.messages {
            listing.items.push([
                sanitize::text(str_of(message, "id")),
                sanitize::text(str_of(message, "threadId")),
            ]);
        }

        let count = self.messages.len();
        let next = self.next_page.as_deref();
        listing.page(ListingPage::Token, count, ["message", "messages"], next);

        write!(f, "{listing}")
    }
}

#[cfg(test)]
mod tests {
    use io_gmail::v1::client::GmailClientStd;
    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use crate::{
        account::context::Account,
        gmail::{client::GmailClient, messages::list::GmailMessagesListCommand},
        http_script::{Script, ok},
    };

    /// The page prints as sent, `nextPageToken` lifted into `nextPage`; an
    /// empty page, sent without `messages`, prints `[]`.
    #[test]
    fn a_page_prints_as_gmail_sent_it() {
        let list = |page: serde_json::Value| {
            let (script, _) = Script::new(vec![ok("application/json", &page.to_string())]);
            let inner = GmailClientStd::new(script, "token", Default::default());
            let command = GmailMessagesListCommand {
                query: None,
                labels: Vec::new(),
                max_results: Some(1),
                page_token: None,
                include_spam_trash: false,
            };
            let mut printer = CapturePrinter::default();
            let mut client = GmailClient::over(inner);
            command
                .execute(&mut printer, &mut Account::default(), &mut client)
                .unwrap();
            printer.into_single().unwrap()
        };

        let message = json!({"id": "m1", "threadId": "t1"});
        let page = json!({"messages": [message], "nextPageToken": "p2", "resultSizeEstimate": 3});
        let expected = json!({"messages": [message], "resultSizeEstimate": 3, "nextPage": "p2"});
        assert_eq!(list(page), expected);

        let empty = json!({"messages": [], "resultSizeEstimate": 0, "nextPage": null});
        assert_eq!(list(json!({"resultSizeEstimate": 0})), empty);
    }
}
