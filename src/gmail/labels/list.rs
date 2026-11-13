//! # Gmail label list
//!
//! The `gmail labels list` command, `users.labels.list`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::labels::{GmailLabel, GmailLabelType};
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    backend::Backend,
    gmail::client::GmailClient,
    listing::{self, Listing},
};

/// List all Gmail labels (users.labels.list).
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
        let labels = client.labels_list()?.response.labels;

        printer.out(GmailLabelListOutput::new(account, labels))
    }
}

/// The Gmail labels, as `list` and `get` print them.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GmailLabelListOutput {
    #[serde(skip)]
    account: String,
    labels: Vec<GmailLabel>,
}

impl GmailLabelListOutput {
    /// Wraps the labels of `account`, shared by the `list` and `get`
    /// commands.
    pub fn new(account: &Account, labels: Vec<GmailLabel>) -> Self {
        Self {
            account: account.name().to_string(),
            labels,
        }
    }
}

impl fmt::Display for GmailLabelListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut listing = Listing::new(&self.account, Some(Backend::Gmail));

        for label in &self.labels {
            let label_type = label.label_type.map(label_type_wire).unwrap_or_default();

            listing.items.push([
                sanitize::text(&label.id).into_owned(),
                label_type.to_string(),
                listing::counts(label.messages_unread, label.messages_total),
                sanitize::text(&label.name).into_owned(),
            ]);
        }

        write!(f, "{listing}")
    }
}

/// Map a label type to its Gmail wire spelling for display.
fn label_type_wire(label_type: GmailLabelType) -> &'static str {
    match label_type {
        GmailLabelType::System => "system",
        GmailLabelType::User => "user",
        GmailLabelType::Unknown => "unknown",
    }
}
