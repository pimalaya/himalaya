//! # Microsoft Graph profile get
//!
//! The `msgraph profile get` command, fetching the signed-in user.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::msgraph::client::MsgraphClient;

/// Get the Microsoft Graph user profile (`GET /me`).
///
/// JSON output: the Graph user object as Graph sent it.
#[derive(Debug, Parser)]
pub struct MsgraphProfileGetCommand;

impl MsgraphProfileGetCommand {
    /// Fetches the signed-in user and prints the profile.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let user = client.raw_get(&[])?;
        printer.out(MsgraphProfileOutput(user))
    }
}

/// The signed-in user as Graph sent it, every property kept, rendered as
/// aligned text or, under `--json`, verbatim.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct MsgraphProfileOutput(#[schemars(with = "Map<String, Value>")] pub Value);

impl fmt::Display for MsgraphProfileOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = Fields::default();

        for (label, key) in [
            ("Id", "id"),
            ("Display name", "displayName"),
            ("Mail", "mail"),
            ("User principal name", "userPrincipalName"),
        ] {
            if let Some(value) = self.0.get(key).and_then(Value::as_str) {
                fields.push(label, sanitize::text(value));
            }
        }

        write!(f, "{fields}")
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use crate::msgraph::{client::MsgraphClient, profile::get::MsgraphProfileGetCommand};

    /// The signed-in user prints as Graph sent it, every property kept.
    #[test]
    fn the_user_prints_as_graph_sent_it() {
        let user = json!({
            "id": "u1",
            "displayName": "Alice",
            "mail": "a@x.org",
            "userPrincipalName": "a@x.org",
            "jobTitle": "Engineer",
        });
        let (mut client, requests) = MsgraphClient::scripted(std::slice::from_ref(&user));
        let mut printer = CapturePrinter::default();

        MsgraphProfileGetCommand
            .execute(&mut printer, &mut client)
            .unwrap();

        assert_eq!(printer.into_single().unwrap(), user);
        let requests = requests.lock().unwrap();
        assert!(requests[0].starts_with("GET /v1.0/me "));
    }
}
