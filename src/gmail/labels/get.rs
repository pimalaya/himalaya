//! # Gmail label get
//!
//! The `gmail labels get` command, `users.labels.get`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{fields::Fields, printer::Printer};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::gmail::{client::GmailClient, labels::list::label_cells};

/// Get a Gmail label by identifier (users.labels.get).
///
/// JSON output: the Gmail label as Gmail sent it.
#[derive(Debug, Parser)]
pub struct GmailLabelGetCommand {
    /// Identifier of the label to get.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl GmailLabelGetCommand {
    /// Fetches the label and prints it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let label = client.raw_get(&["labels", &self.id])?;
        printer.out(GmailLabelGetOutput(Value::Object(label)))
    }
}

/// One Gmail label as Gmail sent it, every property kept.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct GmailLabelGetOutput(#[schemars(with = "Map<String, Value>")] pub Value);

impl fmt::Display for GmailLabelGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [id, label_type, counts, name] = label_cells(&self.0);

        let mut fields = Fields::default();
        fields.push("Id", id);
        fields.push("Type", label_type);
        fields.push("Messages", counts);
        fields.push("Name", name);
        write!(f, "{fields}")
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use crate::gmail::{client::GmailClient, labels::get::GmailLabelGetCommand};

    /// The label prints as Gmail sent it, a property io-gmail does not
    /// model kept, not wrapped in a list.
    #[test]
    fn a_label_prints_as_gmail_sent_it() {
        let label = json!({"id": "L1", "name": "Work", "type": "user", "x-vendor": 1});
        let (mut client, requests) = GmailClient::scripted(std::slice::from_ref(&label));
        let mut printer = CapturePrinter::default();

        let command = GmailLabelGetCommand { id: "L1".into() };
        command.execute(&mut printer, &mut client).unwrap();

        assert_eq!(printer.into_single().unwrap(), label);
        let requests = requests.lock().unwrap();
        assert!(requests[0].starts_with("GET /gmail/v1/users/me/labels/L1 "));
    }
}
