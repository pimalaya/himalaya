use std::fmt;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use io_gmail::v1::rest::history::{
    GmailHistoryLabel, GmailHistoryMessage, GmailHistoryType,
    list::{GmailHistoryList, GmailHistoryListParams},
};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, gmail::client::GmailClient, shared::output::Paginated};

/// Manage the Gmail mailbox history (users.history).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailHistoryCommand {
    List(GmailHistoryListCommand),
}

impl GmailHistoryCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, client),
        }
    }
}

/// List the changes applied to the mailbox since a given history id.
#[derive(Debug, Parser)]
pub struct GmailHistoryListCommand {
    /// History id to start listing changes from.
    #[arg(long = "start-history-id", value_name = "ID")]
    pub start_history_id: String,

    /// Restrict the listing to changes affecting this label id.
    #[arg(long = "label-id", value_name = "ID")]
    pub label_id: Option<String>,

    /// History change types to include (repeatable).
    #[arg(long = "history-type", value_name = "TYPE")]
    pub history_types: Vec<HistoryTypeArg>,

    /// Maximum number of history records to return.
    #[arg(short = 's', long, value_name = "N")]
    pub max_results: Option<u32>,

    /// Page token from a previous listing, for pagination.
    #[arg(long, value_name = "TOKEN")]
    pub page_token: Option<String>,
}

impl GmailHistoryListCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let types: Vec<GmailHistoryType> =
            self.history_types.iter().copied().map(Into::into).collect();

        let out = {
            let params = GmailHistoryListParams {
                start_history_id: &self.start_history_id,
                label_id: self.label_id.as_deref(),
                history_types: &types,
                max_results: self.max_results,
                page_token: self.page_token.as_deref(),
            };
            let c = GmailHistoryList::new(&client.auth, &client.user_id, &params)?;
            client.run(c)?
        };

        let resp = out.response;

        let history = resp
            .history
            .into_iter()
            .map(|record| GmailHistoryRecordOutput {
                id: record.id,
                messages_added: message_ids(record.messages_added),
                messages_deleted: message_ids(record.messages_deleted),
                labels_added: label_changes(record.labels_added),
                labels_removed: label_changes(record.labels_removed),
            })
            .collect();

        let output = GmailHistoryListOutput {
            history_id: resp.history_id,
            history,
        };

        printer.out(Paginated::new(output, resp.next_page_token))
    }
}

/// Gmail mailbox history delta, rendered as one summary line per record
/// or, under `--json`, as structured records instead of a wrapped human
/// string.
///
/// The JSON carries the affected message ids rather than the counts the
/// text summary shows, since driving an incremental sync is what the
/// history listing is for.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GmailHistoryListOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    history_id: Option<String>,
    history: Vec<GmailHistoryRecordOutput>,
}

impl fmt::Display for GmailHistoryListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let history_id = self.history_id.as_deref().unwrap_or("(none)");
        writeln!(f, "New history id: {history_id}")?;

        if self.history.is_empty() {
            return writeln!(f, "No history changes since the given history id.");
        }

        for record in &self.history {
            writeln!(
                f,
                "{}: +{}msg -{}msg +{}lbl -{}lbl",
                record.id,
                record.messages_added.len(),
                record.messages_deleted.len(),
                record.labels_added.len(),
                record.labels_removed.len(),
            )?;
        }

        Ok(())
    }
}

/// A single change to the mailbox since the requested history id.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GmailHistoryRecordOutput {
    id: String,
    messages_added: Vec<String>,
    messages_deleted: Vec<String>,
    labels_added: Vec<GmailHistoryLabelOutput>,
    labels_removed: Vec<GmailHistoryLabelOutput>,
}

/// Labels added to or removed from one message in a history record.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GmailHistoryLabelOutput {
    message_id: String,
    label_ids: Vec<String>,
}

/// Collects the ids of the messages a history change applies to.
fn message_ids(messages: Vec<GmailHistoryMessage>) -> Vec<String> {
    messages
        .into_iter()
        .map(|message| message.message.id)
        .collect()
}

/// Collects the label changes of a history record, keyed by message.
fn label_changes(labels: Vec<GmailHistoryLabel>) -> Vec<GmailHistoryLabelOutput> {
    labels
        .into_iter()
        .map(|label| GmailHistoryLabelOutput {
            message_id: label.message.id,
            label_ids: label.label_ids,
        })
        .collect()
}

/// Gmail history change type accepted on the CLI.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[clap(rename_all = "camelCase")]
pub enum HistoryTypeArg {
    MessageAdded,
    MessageDeleted,
    LabelAdded,
    LabelRemoved,
}

impl From<HistoryTypeArg> for GmailHistoryType {
    fn from(arg: HistoryTypeArg) -> Self {
        match arg {
            HistoryTypeArg::MessageAdded => GmailHistoryType::MessageAdded,
            HistoryTypeArg::MessageDeleted => GmailHistoryType::MessageDeleted,
            HistoryTypeArg::LabelAdded => GmailHistoryType::LabelAdded,
            HistoryTypeArg::LabelRemoved => GmailHistoryType::LabelRemoved,
        }
    }
}
