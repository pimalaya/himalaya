//! # Gmail filter list
//!
//! The `gmail settings filters list` command,
//! `users.settings.filters.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_gmail::v1::rest::settings::filters::list::{GmailFiltersList, GmailFiltersListResponse};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::filters::summary::{action_summary, criteria_summary},
    },
    shared::table::style_from_preset,
};

/// List all Gmail filters (users.settings.filters.list).
#[derive(Debug, Parser)]
pub struct GmailSettingsFiltersListCommand;

impl GmailSettingsFiltersListCommand {
    /// Lists every filter and tables it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let out = {
            let c = GmailFiltersList::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };
        let resp = out.response;

        let table = FiltersTable {
            preset: account.table_preset().to_string(),
            arrangement: account.table_arrangement(),
            response: resp,
        };

        printer.out(table)
    }
}

/// Renderable table of Gmail filters.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct FiltersTable {
    #[serde(skip)]
    #[schemars(skip)]
    preset: String,
    #[serde(skip)]
    #[schemars(skip)]
    arrangement: ContentArrangement,
    response: GmailFiltersListResponse,
}

impl fmt::Display for FiltersTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_style(style_from_preset(&self.preset))
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from([
                Cell::new("ID"),
                Cell::new("CRITERIA"),
                Cell::new("ACTION"),
            ]))
            .add_rows(self.response.filter.iter().map(|filter| {
                let criteria = filter
                    .criteria
                    .as_ref()
                    .map(criteria_summary)
                    .unwrap_or_default();
                let action = filter
                    .action
                    .as_ref()
                    .map(action_summary)
                    .unwrap_or_default();

                let mut row = Row::new();
                row.max_height(1)
                    .add_cell(Cell::new(&filter.id).fg(Color::Reset))
                    .add_cell(Cell::new(criteria).fg(Color::Reset))
                    .add_cell(Cell::new(action).fg(Color::Reset));
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
