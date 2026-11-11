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
use pimalaya_cli::{columns::Columns, output::Paginated, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, gmail::client::GmailClient};

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
        _account: &mut Account,
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

        let next_page = response.next_page_token;
        let threads = GmailThreadListOutput {
            threads: response.threads,
        };

        printer.out(Paginated {
            inner: threads,
            next_page,
        })
    }
}

/// The `gmail threads list` output, the thread summaries of one page.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailThreadListOutput {
    /// The threads of this page.
    pub threads: Vec<GmailThreadSummary>,
}

impl fmt::Display for GmailThreadListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: a snippet is free text, so it reads last.
        let mut columns = Columns::new(["ID", "HISTORY ID", "SNIPPET"]);

        for thread in &self.threads {
            columns.push([
                sanitize::text(&thread.id),
                sanitize::text(thread.history_id.as_deref().unwrap_or_default()),
                sanitize::text(thread.snippet.as_deref().unwrap_or_default()),
            ]);
        }

        write!(f, "{columns}")
    }
}
