//! # Gmail draft list
//!
//! The `gmail drafts list` command, `users.drafts.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::drafts::list::GmailDraftsListParams;
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
    backend::Backend,
    gmail::{client::GmailClient, format::str_of},
};

/// List Gmail drafts (users.drafts.list).
///
/// JSON output: Gmail's page as Gmail sent it, `{"drafts": [{"id",
/// "message": {"id", "threadId"}}...], "resultSizeEstimate", "nextPage"}`,
/// `nextPage` Gmail's `nextPageToken`, `null` on the last page.
#[derive(Debug, Parser)]
pub struct GmailDraftsListCommand {
    /// Gmail search query, using the same syntax as the Gmail search
    /// box (e.g. `from:alice is:unread`).
    #[arg(short = 'q', long)]
    pub query: Option<String>,
    /// Maximum number of drafts to return.
    #[arg(short = 's', long, value_name = "N")]
    pub max_results: Option<u32>,
    /// Page token returned by a previous listing, to fetch the next
    /// page.
    #[arg(long, value_name = "TOKEN")]
    pub page_token: Option<String>,
    /// Also include drafts from SPAM and TRASH.
    #[arg(long)]
    pub include_spam_trash: bool,
}

impl GmailDraftsListCommand {
    /// Lists one page of drafts and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let params = GmailDraftsListParams {
            q: self.query.as_deref(),
            max_results: self.max_results,
            page_token: self.page_token.as_deref(),
            include_spam_trash: self.include_spam_trash,
        };
        let page = client.raw_list("drafts", &params)?;

        printer.out(GmailDraftListOutput {
            account: account.name().to_string(),
            query: self.query,
            drafts: page.items,
            rest: page.rest,
            next_page: page.next_page,
        })
    }
}

/// The `gmail drafts list` output, the drafts of one page.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailDraftListOutput {
    /// The name of the account listed.
    #[serde(skip)]
    pub account: String,
    /// The search query, if any.
    #[serde(skip)]
    pub query: Option<String>,
    /// The drafts of this page, each as Gmail sent it, `{id, message: {id,
    /// threadId}}`.
    #[schemars(with = "Vec<Map<String, Value>>")]
    pub drafts: Vec<Value>,
    /// The rest of the page as Gmail sent it, such as
    /// `resultSizeEstimate`.
    #[serde(flatten)]
    pub rest: Map<String, Value>,
    /// The cursor of the next page, `null` once the listing is complete,
    /// given back through `--page-token`.
    pub next_page: Option<String>,
}

impl fmt::Display for GmailDraftListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Gmail.as_str()));

        if let Some(query) = &self.query {
            listing.head("Query", query);
        }

        for draft in &self.drafts {
            let message = draft.get("message").unwrap_or(&Value::Null);
            let message_id = str_of(message, "id");
            let thread_id = str_of(message, "threadId");

            listing.items.push([
                sanitize::text(str_of(draft, "id")),
                sanitize::text(message_id),
                sanitize::text(thread_id),
            ]);
        }

        let count = self.drafts.len();
        let next = self.next_page.as_deref();
        listing.page(ListingPage::Token, count, ["draft", "drafts"], next);

        write!(f, "{listing}")
    }
}
