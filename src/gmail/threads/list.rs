use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_gmail::v1::rest::threads::{
    GmailThreadSummary,
    list::{GmailThreadsList, GmailThreadsListParams},
};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::client::GmailClient,
    shared::{output::Paginated, table::style_from_preset},
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

        let next_page = response.next_page_token;
        let table = ThreadsTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            threads: response.threads,
        };

        printer.out(Paginated::new(table, next_page))
    }
}

/// Renders a list of Gmail thread summaries as a three-column table.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ThreadsTable {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    pub threads: Vec<GmailThreadSummary>,
}

impl fmt::Display for ThreadsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_style(style_from_preset(&self.preset))
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("SNIPPET"),
                Cell::new("HISTORY ID"),
            ]))
            .add_rows(self.threads.iter().map(|t| {
                let mut row = Row::new();
                row.max_height(1)
                    .add_cell(Cell::new(&t.id).fg(Color::Reset))
                    .add_cell(Cell::new(t.snippet.as_deref().unwrap_or("")))
                    .add_cell(Cell::new(t.history_id.as_deref().unwrap_or("")));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
