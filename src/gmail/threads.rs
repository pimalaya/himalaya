use std::fmt;

use anyhow::Result;
use clap::{Parser, Subcommand};
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use io_gmail::v1::rest::{
    messages::GmailMessageFormat,
    threads::{
        GmailThreadSummary,
        delete::GmailThreadDelete,
        get::GmailThreadGet,
        list::{GmailThreadsList, GmailThreadsListParams},
        modify::GmailThreadModify,
        trash::GmailThreadTrash,
        untrash::GmailThreadUntrash,
    },
};
use pimalaya_cli::printer::{Message, Printer};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::{client::GmailClient, format::FormatArg, messages::GmailMessageHeaderOutput},
    shared::{output::Paginated, table::style_from_preset},
};

/// Manage Gmail threads (users.threads).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailThreadsCommand {
    List(GmailThreadsListCommand),
    Get(GmailThreadGetCommand),
    Modify(GmailThreadModifyCommand),
    Trash(GmailThreadTrashCommand),
    Untrash(GmailThreadUntrashCommand),
    #[command(visible_aliases = ["del", "remove", "rm"])]
    Delete(GmailThreadDeleteCommand),
}

impl GmailThreadsCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, account, client),
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Modify(cmd) => cmd.execute(printer, client),
            Self::Trash(cmd) => cmd.execute(printer, client),
            Self::Untrash(cmd) => cmd.execute(printer, client),
            Self::Delete(cmd) => cmd.execute(printer, client),
        }
    }
}

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

/// Get a single Gmail thread with all its messages
/// (users.threads.get).
#[derive(Debug, Parser)]
pub struct GmailThreadGetCommand {
    /// The id of the thread to get.
    #[arg(value_name = "ID")]
    pub id: String,

    /// The amount of message detail to return.
    #[arg(long, value_enum, default_value_t)]
    pub format: FormatArg,

    /// Header to include when `--format metadata` is used. Can be
    /// repeated.
    #[arg(long = "header", value_name = "NAME")]
    pub headers: Vec<String>,
}

impl GmailThreadGetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let format = GmailMessageFormat::from(self.format);
        let hs: Vec<&str> = self.headers.iter().map(String::as_str).collect();

        let out = {
            let c = GmailThreadGet::new(&client.auth, &client.user_id, &self.id, format, &hs)?;
            client.run(c)?
        };
        let thread = out.response;

        let messages = thread
            .messages
            .into_iter()
            .map(|message| {
                let headers = message
                    .payload
                    .map(|payload| payload.headers)
                    .unwrap_or_default();

                GmailThreadMessageOutput {
                    id: message.id,
                    label_ids: message.label_ids,
                    snippet: message.snippet,
                    headers: headers
                        .into_iter()
                        .map(GmailMessageHeaderOutput::from)
                        .collect(),
                }
            })
            .collect();

        printer.out(GmailThreadGetOutput {
            id: thread.id,
            messages,
        })
    }
}

/// Modify the labels of every message in a Gmail thread
/// (users.threads.modify).
#[derive(Debug, Parser)]
pub struct GmailThreadModifyCommand {
    /// The id of the thread to modify.
    #[arg(value_name = "ID")]
    pub id: String,

    /// Label id to add to the thread. Can be repeated.
    #[arg(long = "add-label", value_name = "ID")]
    pub add: Vec<String>,

    /// Label id to remove from the thread. Can be repeated.
    #[arg(long = "remove-label", value_name = "ID")]
    pub remove: Vec<String>,
}

impl GmailThreadModifyCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailThreadModify::new(
                &client.auth,
                &client.user_id,
                &self.id,
                &self.add,
                &self.remove,
            )?;
            client.run(c)?
        };
        let thread = out.response;

        printer.out(Message::new(format!(
            "Gmail thread `{}` successfully modified",
            thread.id
        )))
    }
}

/// Move a Gmail thread to the trash (users.threads.trash).
#[derive(Debug, Parser)]
pub struct GmailThreadTrashCommand {
    /// The id of the thread to trash.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailThreadTrashCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailThreadTrash::new(&client.auth, &client.user_id, &self.id)?;
            client.run(c)?
        };
        let thread = out.response;

        printer.out(Message::new(format!(
            "Gmail thread `{}` successfully trashed",
            thread.id
        )))
    }
}

/// Remove a Gmail thread from the trash (users.threads.untrash).
#[derive(Debug, Parser)]
pub struct GmailThreadUntrashCommand {
    /// The id of the thread to untrash.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailThreadUntrashCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailThreadUntrash::new(&client.auth, &client.user_id, &self.id)?;
            client.run(c)?
        };
        let thread = out.response;

        printer.out(Message::new(format!(
            "Gmail thread `{}` successfully untrashed",
            thread.id
        )))
    }
}

/// Permanently delete a Gmail thread (users.threads.delete).
#[derive(Debug, Parser)]
pub struct GmailThreadDeleteCommand {
    /// The id of the thread to delete.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailThreadDeleteCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        {
            let c = GmailThreadDelete::new(&client.auth, &client.user_id, &self.id)?;
            client.run(c)?
        };

        printer.out(Message::new(format!(
            "Gmail thread `{}` permanently deleted",
            self.id
        )))
    }
}

/// Gmail thread and its messages, rendered as an indented list or,
/// under `--json`, as a structured object instead of a wrapped human
/// string.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GmailThreadGetOutput {
    id: String,
    messages: Vec<GmailThreadMessageOutput>,
}

impl fmt::Display for GmailThreadGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Thread id: {}", self.id)?;

        for message in &self.messages {
            let snippet = message.snippet.as_deref().unwrap_or("");
            writeln!(f, "- {}: {snippet}", message.id)?;

            for header in &message.headers {
                writeln!(f, "  {}: {}", header.name, header.value)?;
            }
        }

        Ok(())
    }
}

/// One message of a Gmail thread.
///
/// The thread id is left out, since it is the id of the enclosing
/// [`GmailThreadGetOutput`].
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GmailThreadMessageOutput {
    id: String,
    label_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snippet: Option<String>,
    headers: Vec<GmailMessageHeaderOutput>,
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
