//! # Microsoft Graph mail folder get
//!
//! The `msgraph mail-folders get` command, fetching one folder by id.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{listing, msgraph::client::MsgraphClient};

/// Get a Microsoft Graph mail folder by id (`GET /me/mailFolders/{id}`).
///
/// JSON output: the Graph mail folder as Graph sent it.
#[derive(Debug, Parser)]
pub struct MsgraphMailFolderGetCommand {
    /// The id of the mail folder to get.
    #[arg(value_name = "ID")]
    pub id: String,
}

impl MsgraphMailFolderGetCommand {
    /// Fetches the folder and prints it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let folder = client.raw_get(&["mailFolders", &self.id])?;
        printer.out(MsgraphMailFolderGetOutput(folder))
    }
}

/// One Graph mail folder as Graph sent it, every property kept.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct MsgraphMailFolderGetOutput(#[schemars(with = "Map<String, Value>")] pub Value);

impl fmt::Display for MsgraphMailFolderGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text =
            |key| sanitize::text(self.0.get(key).and_then(Value::as_str).unwrap_or_default());
        let number = |key| self.0.get(key).and_then(Value::as_u64);
        let unread = number("unreadItemCount");
        let total = number("totalItemCount");
        let children = number("childFolderCount").map(|n| n.to_string());

        let mut fields = Fields::default();
        fields.push("Id", text("id"));
        fields.push("Name", text("displayName"));
        fields.push("Parent", text("parentFolderId"));
        fields.push("Messages", listing::counts(unread, total));
        fields.push("Child folders", children.unwrap_or_default());
        write!(f, "{fields}")
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use crate::msgraph::{client::MsgraphClient, mail_folders::get::MsgraphMailFolderGetCommand};

    /// The folder prints as Graph sent it, a property io-msgraph does not
    /// model kept, not wrapped in a list.
    #[test]
    fn a_folder_prints_as_graph_sent_it() {
        let folder = json!({"id": "F1", "displayName": "Work", "isHidden": false, "x": 1});
        let (mut client, requests) = MsgraphClient::scripted(std::slice::from_ref(&folder));
        let mut printer = CapturePrinter::default();

        let command = MsgraphMailFolderGetCommand { id: "F1".into() };
        command.execute(&mut printer, &mut client).unwrap();

        assert_eq!(printer.into_single().unwrap(), folder);
        let requests = requests.lock().unwrap();
        assert!(requests[0].starts_with("GET /v1.0/me/mailFolders/F1 "));
    }
}
