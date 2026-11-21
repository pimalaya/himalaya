//! # Gmail send-as get
//!
//! The `gmail settings sendas get` command,
//! `users.settings.sendAs.get`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::send_as::GmailSendAs;
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    gmail::{
        client::GmailClient,
        settings::convert::{verification_status_wire, yes_no},
    },
    unified::message::read::sanitize_body,
};

/// Get one Gmail send-as alias by e-mail address (settings.sendAs.get).
#[derive(Debug, Parser)]
pub struct GmailSettingsSendAsGetCommand {
    /// E-mail address of the send-as alias to get.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl GmailSettingsSendAsGetCommand {
    /// Fetches the send-as alias and prints it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let send_as = client.raw_get(&["settings", "sendAs", &self.email])?;
        printer.out(GmailSettingsSendAsGetOutput(Value::Object(send_as)))
    }
}

/// The `gmail settings sendas get` output.
///
/// The resource is emitted verbatim, so one alias has the very same shape
/// here as in a `list` row.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GmailSettingsSendAsGetOutput(#[schemars(with = "Map<String, Value>")] Value);

impl fmt::Display for GmailSettingsSendAsGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let send_as: GmailSendAs = serde_json::from_value(self.0.clone()).unwrap_or_default();
        let mut fields = Fields::default();
        fields.push("Email", sanitize::text(&send_as.send_as_email));

        if let Some(display_name) = &send_as.display_name {
            fields.push("Name", sanitize::text(display_name));
        }
        if let Some(reply_to_address) = &send_as.reply_to_address {
            fields.push("Reply-To", sanitize::text(reply_to_address));
        }
        if let Some(signature) = &send_as.signature {
            fields.push("Signature", sanitize_body(signature));
        }
        if let Some(is_primary) = send_as.is_primary {
            fields.push("Primary", yes_no(is_primary));
        }
        if let Some(is_default) = send_as.is_default {
            fields.push("Default", yes_no(is_default));
        }
        if let Some(treat_as_alias) = send_as.treat_as_alias {
            fields.push("Treat as alias", yes_no(treat_as_alias));
        }
        if let Some(status) = send_as.verification_status {
            fields.push("Verification", verification_status_wire(status));
        }

        write!(f, "{fields}")
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use crate::gmail::{client::GmailClient, settings::sendas::get::GmailSettingsSendAsGetCommand};

    /// The send-as alias prints as Gmail sent it, its verification status
    /// in Gmail's spelling and a property io-gmail does not model kept.
    #[test]
    fn an_alias_prints_as_gmail_sent_it() {
        let alias = json!({
            "sendAsEmail": "a@x.org",
            "verificationStatus": "accepted",
            "x-vendor": 1,
        });
        let (mut client, requests) = GmailClient::scripted(std::slice::from_ref(&alias));
        let mut printer = CapturePrinter::default();

        let command = GmailSettingsSendAsGetCommand {
            email: "a@x.org".into(),
        };
        command.execute(&mut printer, &mut client).unwrap();

        assert_eq!(printer.into_single().unwrap(), alias);
        let text = requests.lock().unwrap()[0].to_string();
        assert!(
            text.starts_with("GET /gmail/v1/users/me/settings/sendAs/a@x.org "),
            "{text}"
        );
    }
}
