//! # Gmail label update
//!
//! The `gmail labels update` command, `users.labels.patch`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::labels::GmailLabel;
use pimalaya_cli::printer::Printer;

use crate::{gmail::client::GmailClient, receipt::IdReceipt};

/// Rename a Gmail label (users.labels.patch).
///
/// Only the name is sent, so the label keeps its colour and visibility.
#[derive(Debug, Parser)]
pub struct GmailLabelUpdateCommand {
    /// Identifier of the label to update.
    #[arg(value_name = "ID")]
    pub id: String,
    /// New display name to set on the label.
    #[arg(value_name = "NAME")]
    pub name: String,
}

impl GmailLabelUpdateCommand {
    /// Renames the label.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let label = GmailLabel {
            id: self.id.clone(),
            name: self.name.clone(),
            ..Default::default()
        };
        client.label_patch(&label)?;

        printer.out(IdReceipt {
            id: self.id,
            done: "Gmail label successfully updated",
        })
    }
}

#[cfg(test)]
mod tests {
    use io_gmail::v1::client::GmailClientStd;
    use pimalaya_cli::printer::CapturePrinter;

    use crate::{
        gmail::{client::GmailClient, labels::update::GmailLabelUpdateCommand},
        http_script::{Script, ok},
    };

    /// A rename patches the name alone, keeping the colour and visibility a
    /// `labels.update` would clear.
    #[test]
    fn a_rename_patches_the_name_alone() {
        let label = r#"{"id":"Label_1","name":"Renamed","type":"user"}"#;
        let (script, requests) = Script::new(vec![ok("application/json", label)]);
        let mut client =
            GmailClient::over(GmailClientStd::new(script, "token", Default::default()));
        let mut printer = CapturePrinter::default();

        let command = GmailLabelUpdateCommand {
            id: String::from("Label_1"),
            name: String::from("Renamed"),
        };
        command.execute(&mut printer, &mut client).unwrap();

        let requests = requests.lock().unwrap();
        let [request] = requests.as_slice() else {
            panic!("{requests:?}");
        };
        assert!(request.starts_with("PATCH "), "{request}");
        let body = request.split("\r\n\r\n").nth(1).unwrap();
        let body: serde_json::Value = serde_json::from_str(body).unwrap();
        assert_eq!(
            body,
            serde_json::json!({"id": "Label_1", "name": "Renamed"})
        );
    }
}
