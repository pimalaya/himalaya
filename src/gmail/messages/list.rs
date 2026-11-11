//! # Gmail message list
//!
//! The `gmail messages list` command, `users.messages.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::messages::{GmailMessageId, list::GmailMessagesListParams};
use pimalaya_cli::{columns::Columns, output::Paginated, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, gmail::client::GmailClient};

/// List Gmail message ids matching the given query and labels
/// (users.messages.list).
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
        _account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let params = GmailMessagesListParams {
            q: self.query.as_deref(),
            label_ids: &self.labels,
            max_results: self.max_results,
            page_token: self.page_token.as_deref(),
            include_spam_trash: self.include_spam_trash,
        };
        let response = client.messages_list(&params)?.response;

        let next_page = response.next_page_token;
        let messages = GmailMessageListOutput {
            messages: response.messages,
        };

        printer.out(Paginated {
            inner: messages,
            next_page,
        })
    }
}

/// The `gmail messages list` output, the message ids of one page.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailMessageListOutput {
    /// The messages of this page, each named by its id and thread id.
    pub messages: Vec<GmailMessageId>,
}

impl fmt::Display for GmailMessageListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut columns = Columns::new(["ID", "THREAD ID"]);

        for message in &self.messages {
            columns.push([
                sanitize::text(&message.id),
                sanitize::text(message.thread_id.as_deref().unwrap_or_default()),
            ]);
        }

        write!(f, "{columns}")
    }
}
