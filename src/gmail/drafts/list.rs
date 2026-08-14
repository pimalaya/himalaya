use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_gmail::v1::rest::drafts::{
    GmailDraft,
    list::{GmailDraftsList, GmailDraftsListParams},
};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::client::GmailClient,
    shared::{output::Paginated, table::style_from_preset},
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

        let next_page = response.next_page_token;
        let table = DraftsTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            drafts: response.drafts,
        };

        printer.out(Paginated::new(table, next_page))
    }
}

/// Renders a list of Gmail drafts as a three-column table.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct DraftsTable {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    pub drafts: Vec<GmailDraft>,
}

impl fmt::Display for DraftsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_style(style_from_preset(&self.preset))
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("DRAFT ID"),
                Cell::new("MESSAGE ID"),
                Cell::new("THREAD ID"),
            ]))
            .add_rows(self.drafts.iter().map(|d| {
                let message_id = d.message.as_ref().map(|m| m.id.as_str()).unwrap_or("");
                let thread_id = d
                    .message
                    .as_ref()
                    .and_then(|m| m.thread_id.as_deref())
                    .unwrap_or("");

                let mut row = Row::new();
                row.max_height(1)
                    .add_cell(Cell::new(&d.id).fg(Color::Reset))
                    .add_cell(Cell::new(message_id).fg(Color::Reset))
                    .add_cell(Cell::new(thread_id).fg(Color::Reset));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
