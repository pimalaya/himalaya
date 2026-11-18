//! # Gmail vacation get
//!
//! The `gmail settings vacation get` command,
//! `users.settings.getVacation`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::GmailVacationSettings;
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    gmail::{client::GmailClient, settings::convert::yes_no},
    shared::message::read::sanitize_body,
};

/// Get the Gmail vacation responder settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsVacationGetCommand;

impl GmailSettingsVacationGetCommand {
    /// Fetches the vacation responder and prints it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let settings = client.raw_get(&["settings", "vacation"])?;
        printer.out(GmailSettingsVacationGetOutput(Value::Object(settings)))
    }
}

impl From<GmailVacationSettings> for View {
    fn from(settings: GmailVacationSettings) -> Self {
        Self {
            enable_auto_reply: settings.enable_auto_reply,
            response_subject: settings.response_subject,
            response_body_plain_text: settings.response_body_plain_text,
            response_body_html: settings.response_body_html,
            restrict_to_contacts: settings.restrict_to_contacts,
            restrict_to_domain: settings.restrict_to_domain,
            start_time: settings.start_time,
            end_time: settings.end_time,
        }
    }
}

/// The fields the text form prints, read from what Gmail sent.
struct View {
    enable_auto_reply: bool,
    response_subject: Option<String>,
    response_body_plain_text: Option<String>,
    response_body_html: Option<String>,
    restrict_to_contacts: Option<bool>,
    restrict_to_domain: Option<bool>,
    start_time: Option<String>,
    end_time: Option<String>,
}

impl fmt::Display for View {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let auto_reply = if self.enable_auto_reply {
            "enabled"
        } else {
            "disabled"
        };

        let mut fields = Fields::default();
        fields.push("Auto reply", auto_reply);

        if let Some(subject) = &self.response_subject {
            fields.push("Subject", sanitize::text(subject));
        }
        if let Some(body) = &self.response_body_plain_text {
            fields.push("Body", sanitize_body(body));
        }
        if let Some(html) = &self.response_body_html {
            fields.push("HTML", sanitize_body(html));
        }
        if let Some(restrict) = self.restrict_to_contacts {
            fields.push("Restrict to contacts", yes_no(restrict));
        }
        if let Some(restrict) = self.restrict_to_domain {
            fields.push("Restrict to domain", yes_no(restrict));
        }
        if let Some(start) = &self.start_time {
            fields.push("Start", sanitize::text(start));
        }
        if let Some(end) = &self.end_time {
            fields.push("End", sanitize::text(end));
        }

        write!(f, "{fields}")
    }
}

/// The settings as Gmail sent them, every property kept, the text form
/// reading what it shows out of them.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GmailSettingsVacationGetOutput(
    #[schemars(with = "Map<String, Value>")] pub(crate) Value,
);

impl fmt::Display for GmailSettingsVacationGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let settings: GmailVacationSettings =
            serde_json::from_value(self.0.clone()).unwrap_or_default();
        View::from(settings).fmt(f)
    }
}
