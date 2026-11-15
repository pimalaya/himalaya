//! # Gmail label list
//!
//! The `gmail labels list` command, `users.labels.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::labels::{GmailLabel, GmailLabelType};
use pimalaya_cli::{listing::Listing, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{account::context::Account, backend::Backend, gmail::client::GmailClient, listing};

/// List all Gmail labels (users.labels.list).
///
/// JSON output: Gmail's answer as Gmail sent it, `{"labels": [<Gmail
/// label>...]}`, every label in one answer.
#[derive(Debug, Parser)]
pub struct GmailLabelsListCommand;

impl GmailLabelsListCommand {
    /// Lists every label and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let page = client.raw_list_at(&["labels"], "labels", Vec::new())?;

        printer.out(GmailLabelListOutput {
            account: account.name().to_string(),
            labels: page.items,
            rest: page.rest,
        })
    }
}

/// The `gmail labels list` output, the Gmail labels.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailLabelListOutput {
    #[serde(skip)]
    account: String,
    /// The labels, each as Gmail sent it.
    #[schemars(with = "Vec<Map<String, Value>>")]
    labels: Vec<Value>,
    /// The rest of Gmail's answer as Gmail sent it.
    #[serde(flatten)]
    rest: Map<String, Value>,
}

impl fmt::Display for GmailLabelListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Gmail.as_str()));

        for label in &self.labels {
            listing.items.push(label_cells(label));
        }

        write!(f, "{listing}")
    }
}

/// The id, type, counts and name of a label Gmail sent, as its listing
/// and its get render them.
pub fn label_cells(label: &Value) -> [String; 4] {
    let label: GmailLabel = serde_json::from_value(label.clone()).unwrap_or_default();
    let label_type = label.label_type.map(label_type_wire).unwrap_or_default();

    [
        sanitize::text(&label.id).into_owned(),
        label_type.to_string(),
        listing::counts(label.messages_unread, label.messages_total),
        sanitize::text(&label.name).into_owned(),
    ]
}

/// Map a label type to its Gmail wire spelling for display.
fn label_type_wire(label_type: GmailLabelType) -> &'static str {
    match label_type {
        GmailLabelType::System => "system",
        GmailLabelType::User => "user",
        GmailLabelType::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use crate::{
        account::context::Account,
        gmail::{client::GmailClient, labels::list::GmailLabelsListCommand},
    };

    /// The labels print as Gmail sent them, a property io-gmail does not
    /// model kept.
    #[test]
    fn the_labels_print_as_gmail_sent_them() {
        let answer = json!({"labels": [{"id": "L1", "name": "Work", "x-vendor": 1}]});
        let (mut client, _) = GmailClient::scripted(std::slice::from_ref(&answer));
        let mut printer = CapturePrinter::default();

        let command = GmailLabelsListCommand;
        command
            .execute(&mut printer, &mut Account::default(), &mut client)
            .unwrap();

        assert_eq!(printer.into_single().unwrap(), answer);
    }
}
