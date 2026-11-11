//! # Gmail label list
//!
//! The `gmail labels list` command, `users.labels.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::labels::{GmailLabel, GmailLabelType};
use pimalaya_cli::{columns::Columns, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{account::context::Account, gmail::client::GmailClient};

/// List all Gmail labels (users.labels.list).
#[derive(Debug, Parser)]
pub struct GmailLabelsListCommand;

impl GmailLabelsListCommand {
    /// Lists every label and prints it.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        let labels = client.labels_list()?.response.labels;

        printer.out(GmailLabelListOutput::new(labels))
    }
}

/// The Gmail labels, as `list` and `get` print them.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailLabelListOutput {
    labels: Vec<GmailLabel>,
}

impl GmailLabelListOutput {
    /// Wraps the labels, shared by the `list` and `get` commands.
    pub fn new(labels: Vec<GmailLabel>) -> Self {
        Self { labels }
    }
}

impl fmt::Display for GmailLabelListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: a name is free text, so it reads last.
        let mut columns = Columns::new(["ID", "TYPE", "TOTAL", "UNREAD", "NAME"]);

        for label in &self.labels {
            let label_type = label.label_type.map(label_type_wire).unwrap_or_default();

            columns.push([
                sanitize::text(&label.id).into_owned(),
                label_type.to_string(),
                count(label.messages_total),
                count(label.messages_unread),
                sanitize::text(&label.name).into_owned(),
            ]);
        }

        write!(f, "{columns}")
    }
}

/// Renders a count, or an empty cell when Gmail gave none.
fn count(value: Option<impl ToString>) -> String {
    value.map(|n| n.to_string()).unwrap_or_default()
}

/// Map a label type to its Gmail wire spelling for display.
fn label_type_wire(label_type: GmailLabelType) -> &'static str {
    match label_type {
        GmailLabelType::System => "system",
        GmailLabelType::User => "user",
    }
}
