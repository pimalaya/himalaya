//! # Gmail vacation get
//!
//! The `gmail settings vacation get` command,
//! `users.settings.getVacation`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::get_vacation::GmailVacationGet;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::gmail::{client::GmailClient, settings::convert::yes_no};

/// Get the Gmail vacation responder settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsVacationGetCommand;

impl GmailSettingsVacationGetCommand {
    /// Fetches the vacation responder and tables it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailVacationGet::new(&client.auth, &client.user_id)?;
            client.run(c)?
        };
        let settings = out.response;

        printer.out(GmailSettingsVacationGetOutput {
            enable_auto_reply: settings.enable_auto_reply,
            response_subject: settings.response_subject,
            response_body_plain_text: settings.response_body_plain_text,
            response_body_html: settings.response_body_html,
            restrict_to_contacts: settings.restrict_to_contacts,
            restrict_to_domain: settings.restrict_to_domain,
            start_time: settings.start_time,
            end_time: settings.end_time,
        })
    }
}

/// The `gmail settings vacation get` output.
///
/// A boolean stays a boolean in JSON, where the text spells it enabled or
/// disabled, yes or no.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GmailSettingsVacationGetOutput {
    enable_auto_reply: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_body_plain_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_body_html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    restrict_to_contacts: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    restrict_to_domain: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_time: Option<String>,
}

impl fmt::Display for GmailSettingsVacationGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let auto_reply = if self.enable_auto_reply {
            "enabled"
        } else {
            "disabled"
        };
        writeln!(f, "Auto reply: {auto_reply}")?;

        if let Some(subject) = &self.response_subject {
            writeln!(f, "Subject: {subject}")?;
        }
        if let Some(body) = &self.response_body_plain_text {
            writeln!(f, "Body: {body}")?;
        }
        if let Some(html) = &self.response_body_html {
            writeln!(f, "HTML: {html}")?;
        }
        if let Some(restrict) = self.restrict_to_contacts {
            writeln!(f, "Restrict to contacts: {}", yes_no(restrict))?;
        }
        if let Some(restrict) = self.restrict_to_domain {
            writeln!(f, "Restrict to domain: {}", yes_no(restrict))?;
        }
        if let Some(start) = &self.start_time {
            writeln!(f, "Start: {start}")?;
        }
        if let Some(end) = &self.end_time {
            writeln!(f, "End: {end}")?;
        }

        Ok(())
    }
}
