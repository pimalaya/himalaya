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
use pimalaya_cli::{columns::Columns, output::Paginated, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, gmail::client::GmailClient};

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
        _account: &mut Account,
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

        let next_page = response.next_page_token;
        let drafts = GmailDraftListOutput {
            drafts: response.drafts,
        };

        printer.out(Paginated {
            inner: drafts,
            next_page,
        })
    }
}

/// The `gmail drafts list` output, the drafts of one page.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailDraftListOutput {
    /// The drafts of this page.
    pub drafts: Vec<GmailDraft>,
}

impl fmt::Display for GmailDraftListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut columns = Columns::new(["DRAFT ID", "MESSAGE ID", "THREAD ID"]);

        for draft in &self.drafts {
            let message = draft.message.as_ref();
            let message_id = message.map(|m| m.id.as_str()).unwrap_or_default();
            let thread_id = message
                .and_then(|m| m.thread_id.as_deref())
                .unwrap_or_default();

            columns.push([
                sanitize::text(&draft.id),
                sanitize::text(message_id),
                sanitize::text(thread_id),
            ]);
        }

        write!(f, "{columns}")
    }
}
