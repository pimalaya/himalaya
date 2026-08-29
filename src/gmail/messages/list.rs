//! # Gmail message list
//!
//! The `gmail messages list` command, `users.messages.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_gmail::v1::rest::messages::{GmailMessageId, list::GmailMessagesListParams};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::client::GmailClient,
    shared::{output::Paginated, table::style_from_preset},
};

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
    /// Lists one page of message ids and tables it.
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
        let response = client.messages_list(&params)?.response;

        let next_page = response.next_page_token;
        let table = MessageIdsTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            id_color: account.envelopes_list_table_id_color(),
            ids: response.messages,
        };

        printer.out(Paginated::new(table, next_page))
    }
}

/// Renders a list of Gmail message ids as a two-column table.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct MessageIdsTable {
    #[serde(skip)]
    /// The `comfy_table` preset string the table renders with.
    pub preset: String,
    #[serde(skip)]
    /// The column arrangement the table renders with.
    pub arrangement: ContentArrangement,
    #[serde(skip)]
    /// The color of the ID column.
    pub id_color: Color,
    /// The message ids of this page.
    pub ids: Vec<GmailMessageId>,
}

impl fmt::Display for MessageIdsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_style(style_from_preset(&self.preset))
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([Cell::new("ID"), Cell::new("THREAD ID")]))
            .add_rows(self.ids.iter().map(|id| {
                let mut row = Row::new();
                row.max_height(1)
                    .add_cell(Cell::new(&id.id).fg(self.id_color))
                    .add_cell(Cell::new(id.thread_id.as_deref().unwrap_or("")));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
