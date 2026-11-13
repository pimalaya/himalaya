//! # Gmail thread list
//!
//! The `gmail threads list` command, `users.threads.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::threads::{
    GmailThreadSummary,
    list::{GmailThreadsList, GmailThreadsListParams},
};
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    gmail::{client::GmailClient, format::snippet_text},
    listing::{Listing, ListingPage},
};

/// List Gmail threads matching the given query and labels
/// (users.threads.list).
#[derive(Debug, Parser)]
pub struct GmailThreadsListCommand {
    /// Gmail search query, using the same syntax as the Gmail search
    /// box (e.g. `from:alice is:unread`).
    #[arg(short = 'q', long)]
    pub query: Option<String>,
    /// Only return threads carrying the given label id. Can be repeated
    /// to require multiple labels.
    #[arg(short = 'l', long = "label", value_name = "ID")]
    pub labels: Vec<String>,
    /// Maximum number of threads to return.
    #[arg(short = 's', long, value_name = "N")]
    pub max_results: Option<u32>,
    /// Page token returned by a previous listing, to fetch the next
    /// page.
    #[arg(long, value_name = "TOKEN")]
    pub page_token: Option<String>,
    /// Also include threads from SPAM and TRASH.
    #[arg(long)]
    pub include_spam_trash: bool,
}

impl GmailThreadsListCommand {
    /// Lists one page of threads and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let out = {
            let params = GmailThreadsListParams {
                q: self.query.as_deref(),
                label_ids: &self.labels,
                max_results: self.max_results,
                page_token: self.page_token.as_deref(),
                include_spam_trash: self.include_spam_trash,
            };
            let c = GmailThreadsList::new(&client.auth, &client.user_id, &params)?;
            client.run(c)?
        };
        let response = out.response;

        printer.out(GmailThreadListOutput {
            account: account.name().to_string(),
            query: self.query,
            labels: self.labels,
            threads: response.threads,
            next_page: response.next_page_token,
        })
    }
}

/// The `gmail threads list` output, the thread summaries of one page.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailThreadListOutput {
    /// The name of the account listed.
    #[serde(skip)]
    pub account: String,
    /// The search query, if any.
    #[serde(skip)]
    pub query: Option<String>,
    /// The label ids every thread carries.
    #[serde(skip)]
    pub labels: Vec<String>,
    /// The threads of this page.
    pub threads: Vec<GmailThreadSummary>,
    /// The cursor of the next page, `null` once the listing is complete,
    /// given back through `--page-token`.
    pub next_page: Option<String>,
}

impl fmt::Display for GmailThreadListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Gmail));

        if !self.labels.is_empty() {
            listing.head("Labels", &self.labels.join(", "));
        }

        if let Some(query) = &self.query {
            listing.head("Query", query);
        }

        for thread in &self.threads {
            listing.items.push([
                sanitize::text(&thread.id),
                sanitize::text(thread.history_id.as_deref().unwrap_or_default()),
                sanitize::text(&snippet_text(thread.snippet.as_deref().unwrap_or_default()))
                    .into_owned()
                    .into(),
            ]);
        }

        let count = self.threads.len();
        let next = self.next_page.as_deref();
        listing.page(ListingPage::Token, count, ["thread", "threads"], next);

        write!(f, "{listing}")
    }
}
