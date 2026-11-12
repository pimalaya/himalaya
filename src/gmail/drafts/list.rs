//! # Gmail draft list
//!
//! The `gmail drafts list` command, `users.drafts.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::drafts::{
    GmailDraft,
    list::{GmailDraftsList, GmailDraftsListParams},
};
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    gmail::client::GmailClient,
    listing::{Listing, ListingPage},
};

/// List Gmail drafts (users.drafts.list).
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
        let out = {
            let params = GmailDraftsListParams {
                q: self.query.as_deref(),
                max_results: self.max_results,
                page_token: self.page_token.as_deref(),
                include_spam_trash: self.include_spam_trash,
            };
            let c = GmailDraftsList::new(&client.auth, &client.user_id, &params)?;
            client.run(c)?
        };
        let response = out.response;

        printer.out(GmailDraftListOutput {
            account: account.name().to_string(),
            query: self.query,
            drafts: response.drafts,
            next_page: response.next_page_token,
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
    /// The drafts of this page.
    pub drafts: Vec<GmailDraft>,
    /// The cursor of the next page, `null` once the listing is complete,
    /// given back through `--page-token`.
    pub next_page: Option<String>,
}

impl fmt::Display for GmailDraftListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Gmail));

        if let Some(query) = &self.query {
            listing.head("Query", query);
        }

        for draft in &self.drafts {
            let message = draft.message.as_ref();
            let message_id = message.map(|m| m.id.as_str()).unwrap_or_default();
            let thread_id = message
                .and_then(|m| m.thread_id.as_deref())
                .unwrap_or_default();

            listing.items.push([
                sanitize::text(&draft.id),
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
