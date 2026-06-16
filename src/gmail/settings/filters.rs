use std::fmt;

use anyhow::Result;
use clap::{Parser, Subcommand};
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_gmail::v1::rest::settings::filters::{
    GmailFilter, GmailFilterAction, GmailFilterCriteria, create::GmailFilterCreate,
    delete::GmailFilterDelete, get::GmailFilterGet, list::GmailFiltersList,
    list::GmailFiltersListResponse,
};
use pimalaya_cli::printer::{Message, Printer};
use serde::Serialize;

use crate::{account::context::Account, gmail::client::GmailClient};

/// Manage Gmail filters (users.settings.filters).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsFiltersCommand {
    List(List),
    Get(Get),
    Create(Create),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(Delete),
}

impl GmailSettingsFiltersCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Create(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}

/// List all Gmail filters (users.settings.filters.list).
#[derive(Debug, Parser)]
pub struct List;

impl List {
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

/// Get a Gmail filter by identifier (users.settings.filters.get).
#[derive(Debug, Parser)]
pub struct Get {
    /// Identifier of the filter to get.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl Get {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailFilterGet::new(&client.auth, &client.user_id, &self.id)?;
            client.run(c)?
        };
        let filter = out.response;

        let mut text = format!("Id: {}\n", filter.id);
        if let Some(criteria) = &filter.criteria {
            let summary = criteria_summary(criteria);
            if !summary.is_empty() {
                text.push_str(&format!("Criteria: {summary}\n"));
            }
        }
        if let Some(action) = &filter.action {
            let summary = action_summary(action);
            if !summary.is_empty() {
                text.push_str(&format!("Action: {summary}\n"));
            }
        }

        printer.out(Message::new(text))
    }
}

/// Create a Gmail filter (users.settings.filters.create).
#[derive(Debug, Parser)]
pub struct Create {
    /// Match messages whose sender matches this value.
    #[arg(long, value_name = "ADDR")]
    pub from: Option<String>,

    /// Match messages whose recipient matches this value.
    #[arg(long, value_name = "ADDR")]
    pub to: Option<String>,

    /// Match messages whose subject matches this value.
    #[arg(long, value_name = "TEXT")]
    pub subject: Option<String>,

    /// Match messages with this Gmail search query.
    #[arg(long, value_name = "QUERY")]
    pub query: Option<String>,

    /// Exclude messages matching this Gmail search query.
    #[arg(long, value_name = "QUERY")]
    pub negated_query: Option<String>,

    /// Match only messages that have an attachment.
    #[arg(long)]
    pub has_attachment: bool,

    /// Label identifier to add to matching messages (repeatable).
    #[arg(long = "add-label", value_name = "ID")]
    pub add_label: Vec<String>,

    /// Label identifier to remove from matching messages (repeatable).
    #[arg(long = "remove-label", value_name = "ID")]
    pub remove_label: Vec<String>,

    /// Forward matching messages to this address.
    #[arg(long, value_name = "ADDR")]
    pub forward: Option<String>,
}

impl Create {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let filter = GmailFilter {
            id: String::new(),
            criteria: Some(GmailFilterCriteria {
                from: self.from,
                to: self.to,
                subject: self.subject,
                query: self.query,
                negated_query: self.negated_query,
                has_attachment: self.has_attachment.then_some(true),
                exclude_chats: None,
                size: None,
                size_comparison: None,
            }),
            action: Some(GmailFilterAction {
                add_label_ids: (!self.add_label.is_empty()).then(|| self.add_label.clone()),
                remove_label_ids: (!self.remove_label.is_empty())
                    .then(|| self.remove_label.clone()),
                forward: self.forward,
            }),
        };

        let out = {
            let c = GmailFilterCreate::new(&client.auth, &client.user_id, &filter)?;
            client.run(c)?
        };
        let created = out.response;

        printer.out(Message::new(format!(
            "Gmail filter `{}` successfully created",
            created.id
        )))
    }
}

/// Delete a Gmail filter (users.settings.filters.delete).
#[derive(Debug, Parser)]
pub struct Delete {
    /// Identifier of the filter to delete.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl Delete {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        {
            let c = GmailFilterDelete::new(&client.auth, &client.user_id, &self.id)?;
            client.run(c)?;
        }

        printer.out(Message::new(format!(
            "Gmail filter `{}` successfully deleted",
            self.id
        )))
    }
}

/// Best-effort one-line summary of a filter's match criteria.
fn criteria_summary(criteria: &GmailFilterCriteria) -> String {
    let mut parts = Vec::new();
    if let Some(from) = &criteria.from {
        parts.push(format!("from={from}"));
    }
    if let Some(to) = &criteria.to {
        parts.push(format!("to={to}"));
    }
    if let Some(subject) = &criteria.subject {
        parts.push(format!("subject={subject}"));
    }
    if let Some(query) = &criteria.query {
        parts.push(format!("query={query}"));
    }
    if let Some(negated_query) = &criteria.negated_query {
        parts.push(format!("negated_query={negated_query}"));
    }
    if criteria.has_attachment == Some(true) {
        parts.push("has_attachment".to_string());
    }
    parts.join(" ")
}

/// Best-effort one-line summary of a filter's action.
fn action_summary(action: &GmailFilterAction) -> String {
    let mut parts = Vec::new();
    if let Some(add) = &action.add_label_ids {
        parts.push(format!("+labels={}", add.len()));
    }
    if let Some(remove) = &action.remove_label_ids {
        parts.push(format!("-labels={}", remove.len()));
    }
    if let Some(forward) = &action.forward {
        parts.push(format!("forward={forward}"));
    }
    parts.join(" ")
}

/// Renderable table of Gmail filters.
#[derive(Clone, Debug, Serialize)]
pub struct FiltersTable {
    #[serde(skip)]
    preset: String,
    #[serde(skip)]
    arrangement: ContentArrangement,
    response: GmailFiltersListResponse,
}

impl fmt::Display for FiltersTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
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
