//! # Gmail history list
//!
//! The `gmail history list` command, `users.history.list`.

use std::fmt;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use io_gmail::v1::rest::history::{
    GmailHistory, GmailHistoryLabel, GmailHistoryMessage, GmailHistoryType,
    list::{GmailHistoryList, GmailHistoryListParams},
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

/// List the changes applied to the mailbox since a given history id.
///
/// JSON includes `historyId`, `history`, and an optional `nextPage` cursor.
/// Each record retains its message-id arrays and adds `messagesAddedDetails`
/// with each arrival's `id`, nullable `threadId`, and `labelIds` supplied by
/// Gmail. Missing labels become an empty array; text output lists the ids.
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
    /// Lists one page of history records and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
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
            .map(GmailHistoryRecordOutput::from)
            .collect();

        printer.out(GmailHistoryListOutput {
            account: account.name().to_string(),
            since: self.start_history_id,
            label: self.label_id,
            history_id: resp.history_id,
            history,
            next_page: resp.next_page_token,
        })
    }
}

/// The kind of change a history record reports.
#[derive(Clone, Copy, Debug, ValueEnum)]
#[clap(rename_all = "camelCase")]
pub enum HistoryTypeArg {
    /// A message arrived in the mailbox.
    MessageAdded,
    /// A message left the mailbox.
    MessageDeleted,
    /// A label was put on a message.
    LabelAdded,
    /// A label was taken off a message.
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

/// The `gmail history list` output: the new history id, then each record
/// with the ids of the messages it added, deleted, labeled and unlabeled.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailHistoryListOutput {
    #[serde(skip)]
    account: String,
    #[serde(skip)]
    since: String,
    #[serde(skip)]
    label: Option<String>,
    history_id: Option<String>,
    history: Vec<GmailHistoryRecordOutput>,
    next_page: Option<String>,
}

impl fmt::Display for GmailHistoryListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Gmail));
        listing.head("Since", &self.since);

        if let Some(label) = &self.label {
            listing.head("Label", label);
        }

        for record in &self.history {
            let id = sanitize::text(&record.id).into_owned();
            let line = |change: &str, message: &str, labels: &[String]| {
                [
                    id.clone(),
                    String::from(change),
                    sanitize::text(message).into_owned(),
                    sanitize::text(&labels.join(",")).into_owned(),
                ]
            };

            for message in &record.messages_added {
                listing.items.push(line("added", message, &[]));
            }

            for message in &record.messages_deleted {
                listing.items.push(line("deleted", message, &[]));
            }

            for change in &record.labels_added {
                let labels = &change.label_ids;
                listing
                    .items
                    .push(line("labeled", &change.message_id, labels));
            }

            for change in &record.labels_removed {
                let labels = &change.label_ids;
                listing
                    .items
                    .push(line("unlabeled", &change.message_id, labels));
            }
        }

        let count = self.history.len();
        let next = self.next_page.as_deref();
        listing.page(ListingPage::Token, count, ["record", "records"], next);

        let history_id = self.history_id.as_deref().unwrap_or_default();
        let history_id = sanitize::text(history_id).into_owned();
        listing.header.push(("History id", history_id));

        write!(f, "{listing}")
    }
}

/// A single change to the mailbox since the requested history id.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailHistoryRecordOutput {
    id: String,
    messages_added: Vec<String>,
    messages_added_details: Vec<GmailHistoryMessageOutput>,
    messages_deleted: Vec<String>,
    labels_added: Vec<GmailHistoryLabelOutput>,
    labels_removed: Vec<GmailHistoryLabelOutput>,
}

impl From<GmailHistory> for GmailHistoryRecordOutput {
    fn from(record: GmailHistory) -> Self {
        Self {
            id: record.id,
            messages_added_details: record
                .messages_added
                .iter()
                .map(|entry| GmailHistoryMessageOutput {
                    id: entry.message.id.clone(),
                    thread_id: entry.message.thread_id.clone(),
                    label_ids: entry.message.label_ids.clone(),
                })
                .collect(),
            messages_added: message_ids(record.messages_added),
            messages_deleted: message_ids(record.messages_deleted),
            labels_added: label_changes(record.labels_added),
            labels_removed: label_changes(record.labels_removed),
        }
    }
}

/// Identity and labels supplied by Gmail for an added message.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailHistoryMessageOutput {
    id: String,
    thread_id: Option<String>,
    label_ids: Vec<String>,
}

/// Labels added to or removed from one message in a history record.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
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
