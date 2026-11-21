//! # Gmail history list
//!
//! The `gmail history list` command, `users.history.list`.

use std::fmt;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use io_gmail::v1::rest::history::{
    GmailHistory, GmailHistoryLabel, GmailHistoryMessage, GmailHistoryType,
    list::GmailHistoryListParams,
};
use pimalaya_cli::{
    listing::{Listing, ListingPage},
    printer::Printer,
    sanitize,
};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{account::context::Account, gmail::client::GmailClient, unified::backend::Backend};

/// List the changes applied to the mailbox since a given history id.
///
/// JSON output: Gmail's page as Gmail sent it, `{"history": [<raw Gmail
/// history record>...], "historyId", "nextPage"}`, `nextPage` Gmail's
/// `nextPageToken`, `null` on the last page. The text lists one line per
/// message change.
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

        let params = GmailHistoryListParams {
            start_history_id: &self.start_history_id,
            label_id: self.label_id.as_deref(),
            history_types: &types,
            max_results: self.max_results,
            page_token: self.page_token.as_deref(),
        };
        let page = client.raw_list("history", &params)?;

        printer.out(GmailHistoryListOutput {
            account: account.name().to_string(),
            since: self.start_history_id,
            label: self.label_id,
            history: page.items,
            rest: page.rest,
            next_page: page.next_page,
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

/// The `gmail history list` output, Gmail's page as Gmail sent it.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GmailHistoryListOutput {
    #[serde(skip)]
    account: String,
    #[serde(skip)]
    since: String,
    #[serde(skip)]
    label: Option<String>,
    /// The history records of this page, each as Gmail sent it.
    #[schemars(with = "Vec<Map<String, Value>>")]
    history: Vec<Value>,
    /// The rest of the page as Gmail sent it: `historyId`, the mailbox's
    /// current history id.
    #[serde(flatten)]
    rest: Map<String, Value>,
    /// Gmail's `nextPageToken`, `null` on the last page, given back
    /// through `--page-token`.
    next_page: Option<String>,
}

impl fmt::Display for GmailHistoryListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Gmail.as_str()));
        listing.head("Since", &self.since);

        if let Some(label) = &self.label {
            listing.head("Label", label);
        }

        for record in &self.history {
            // NOTE: a record Gmail sent in a shape io-gmail does not read
            // prints no line; the JSON keeps it whole.
            let Ok(record) = serde_json::from_value::<GmailHistory>(record.clone()) else {
                continue;
            };
            let record = GmailHistoryRecordOutput::from(record);
            let id = sanitize::text(&record.id).into_owned();
            let line = |change: &str, message: &str, labels: &[String]| {
                [
                    id.clone(),
                    String::from(change),
                    sanitize::text(message).into_owned(),
                    sanitize::text(&labels.join(",")).into_owned(),
                ]
            };

            for message in &record.messages_added_details {
                let labels = &message.label_ids;
                listing.items.push(line("added", &message.id, labels));
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

        let history_id = self.rest.get("historyId").and_then(Value::as_str);
        let history_id = history_id.unwrap_or_default();
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

#[cfg(test)]
mod tests {
    use serde_json::{Map, json};

    use crate::gmail::history::list::GmailHistoryListOutput;

    /// The page prints as Gmail sent it, its cursor lifted into `nextPage`,
    /// and the text shows each added message with its labels.
    #[test]
    fn a_page_prints_as_gmail_sent_it() {
        let record = json!({
            "id": "42",
            "messages": [{"id": "m1", "threadId": "t1"}],
            "messagesAdded": [{"message": {"id": "m1", "threadId": "t1", "labelIds": ["Label_1", "INBOX"]}}],
        });
        let mut rest = Map::new();
        rest.insert(String::from("historyId"), json!("43"));
        let output = GmailHistoryListOutput {
            account: "a".into(),
            since: "1".into(),
            label: None,
            history: vec![record.clone()],
            rest,
            next_page: Some(String::from("p2")),
        };

        let json = serde_json::to_value(&output).unwrap();
        let page = json!({"history": [record], "historyId": "43", "nextPage": "p2"});
        assert_eq!(json, page);

        let text = output.to_string();
        let line = text.lines().find(|line| line.contains("m1")).unwrap();
        assert!(line.contains("Label_1,INBOX"), "{text}");
        assert!(text.contains("History id:  43"), "{text}");
    }
}
