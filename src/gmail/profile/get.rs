//! # Gmail profile get
//!
//! The `gmail profile get` command, `users.getProfile`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::gmail::client::GmailClient;

/// Get the Gmail profile: email address, message/thread totals and the
/// current history id.
///
/// JSON output: the profile as Gmail sent it.
#[derive(Debug, Parser)]
pub struct GmailProfileGetCommand;

impl GmailProfileGetCommand {
    /// Fetches the profile and prints it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let profile = client.raw_get(&["profile"])?;
        printer.out(GmailProfileOutput(Value::Object(profile)))
    }
}

/// The Gmail profile as Gmail sent it, every property kept.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GmailProfileOutput(#[schemars(with = "Map<String, Value>")] Value);

impl fmt::Display for GmailProfileOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = Fields::default();
        let rows = [
            ("Email", "emailAddress"),
            ("Messages", "messagesTotal"),
            ("Threads", "threadsTotal"),
            ("History id", "historyId"),
        ];

        for (label, key) in rows {
            match self.0.get(key) {
                Some(Value::String(value)) => fields.push(label, sanitize::text(value)),
                Some(Value::Number(value)) => fields.push(label, value),
                _ => {}
            }
        }

        write!(f, "{fields}")
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use crate::gmail::{client::GmailClient, profile::get::GmailProfileGetCommand};

    /// The profile prints as Gmail sent it, its keys in Gmail's spelling
    /// and a property io-gmail does not model kept.
    #[test]
    fn the_profile_prints_as_gmail_sent_it() {
        let profile = json!({
            "emailAddress": "a@gmail.com",
            "messagesTotal": 3,
            "threadsTotal": 2,
            "historyId": "9",
            "x-vendor": 1,
        });
        let (mut client, requests) = GmailClient::scripted(std::slice::from_ref(&profile));
        let mut printer = CapturePrinter::default();

        GmailProfileGetCommand
            .execute(&mut printer, &mut client)
            .unwrap();

        assert_eq!(printer.into_single().unwrap(), profile);
        let requests = requests.lock().unwrap();
        assert!(requests[0].starts_with("GET /gmail/v1/users/me/profile "));
    }
}
