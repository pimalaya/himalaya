//! # JMAP vacation get
//!
//! The `jmap vacation-response get` command, RFC 8621
//! `VacationResponse/get`.

use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use io_jmap::rfc8621::vacation_response::JMAP_VACATION_RESPONSE_CAPABILITY;
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::{
    account::context::Account,
    jmap::{client::JmapClient, raw},
    shared::message::read::sanitize_body,
};

/// Get the JMAP vacation response (VacationResponse/get).
///
/// JSON output: the `VacationResponse` object as the server sent it.
#[derive(Debug, Parser)]
pub struct JmapVacationGetCommand;

impl JmapVacationGetCommand {
    /// Fetches the vacation response and prints it, a server holding none
    /// reading as a disabled one, RFC 8621 making the singleton always
    /// exist.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut JmapClient,
    ) -> Result<()> {
        let has_vacation = client
            .session()
            .map(|s| {
                s.capabilities
                    .contains_key(JMAP_VACATION_RESPONSE_CAPABILITY)
            })
            .unwrap_or(false);

        if !has_vacation {
            bail!("Vacation response is not supported by the server");
        }

        let list = raw::get(client, "VacationResponse", None)?.list;

        let vacation = list
            .into_iter()
            .next()
            .unwrap_or_else(|| json!({"id": "singleton", "isEnabled": false}));

        printer.out(JmapVacationResponseGetOutput { vacation })
    }
}

/// The `jmap vacation get` output, the vacation response the server
/// holds, as it sent it.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct JmapVacationResponseGetOutput {
    /// The vacation response the server holds, every property kept.
    #[schemars(with = "Map<String, Value>")]
    pub vacation: Value,
}

impl fmt::Display for JmapVacationResponseGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = |key: &str| self.vacation.get(key).and_then(Value::as_str);
        let enabled = self.vacation.get("isEnabled").and_then(Value::as_bool);
        let mut fields = Fields::default();

        fields.push("Enabled", enabled.unwrap_or_default());

        if let Some(d) = text("fromDate") {
            fields.push("From", sanitize::text(d));
        }
        if let Some(d) = text("toDate") {
            fields.push("To", sanitize::text(d));
        }
        if let Some(s) = text("subject") {
            fields.push("Subject", sanitize::text(s));
        }
        if let Some(b) = text("textBody") {
            fields.push("Body (plain)", sanitize_body(b));
        }
        if let Some(b) = text("htmlBody") {
            fields.push("Body (HTML)", sanitize_body(b));
        }

        write!(f, "{fields}")
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::jmap::vacation::get::JmapVacationResponseGetOutput;

    /// The vacation response prints as the server sent it, a property
    /// io-jmap does not model kept, and its text reads the known ones.
    #[test]
    fn the_vacation_response_prints_as_sent() {
        let vacation = json!({
            "id": "singleton",
            "isEnabled": true,
            "subject": "Away",
            "x-vendor": 1,
        });
        let output = JmapVacationResponseGetOutput {
            vacation: vacation.clone(),
        };

        assert_eq!(serde_json::to_value(&output).unwrap(), vacation);
        let text = output.to_string();
        assert!(text.contains("Enabled") && text.contains("true"), "{text}");
        assert!(text.contains("Away"), "{text}");
    }
}
