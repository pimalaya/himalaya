use std::fmt;

use anyhow::Result;
use clap::{Parser, Subcommand};
use io_gmail::v1::rest::settings::{
    get_vacation::GmailVacationGet, update_vacation::GmailVacationUpdate,
};
use pimalaya_cli::printer::{Message, Printer};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    gmail::{
        client::GmailClient,
        settings::convert::{enabled_flag, yes_no},
    },
};

/// Manage the Gmail vacation responder settings
/// (users.settings.getVacation / updateVacation).
#[derive(Debug, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum GmailSettingsVacationCommand {
    Get(GmailSettingsVacationGetCommand),
    #[command(visible_aliases = ["update"])]
    Set(GmailSettingsVacationSetCommand),
}

impl GmailSettingsVacationCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: &mut GmailClient,
    ) -> Result<()> {
        match self {
            Self::Get(cmd) => cmd.execute(printer, client),
            Self::Set(cmd) => cmd.execute(printer, client),
        }
    }
}

/// Get the Gmail vacation responder settings.
#[derive(Debug, Parser)]
pub struct GmailSettingsVacationGetCommand;

impl GmailSettingsVacationGetCommand {
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

/// Update the Gmail vacation responder settings.
///
/// Partial update: the settings are fetched first and only the options
/// you pass are changed, so unspecified fields are preserved. The
/// responder is toggled with `--enable` / `--disable` and never by
/// accident.
#[derive(Debug, Parser)]
pub struct GmailSettingsVacationSetCommand {
    /// Turn the responder on.
    #[arg(long, conflicts_with = "disable")]
    pub enable: bool,

    /// Turn the responder off.
    #[arg(long)]
    pub disable: bool,

    /// Subject of the auto-reply message.
    #[arg(long)]
    pub subject: Option<String>,

    /// Plain-text body of the auto-reply message.
    #[arg(long)]
    pub body: Option<String>,

    /// HTML body of the auto-reply message.
    #[arg(long)]
    pub html: Option<String>,

    /// Send the auto-reply only to people in your contacts.
    #[arg(long)]
    pub restrict_to_contacts: Option<bool>,

    /// Send the auto-reply only to people in your domain.
    #[arg(long)]
    pub restrict_to_domain: Option<bool>,

    /// First day the responder is active; Gmail expects epoch
    /// milliseconds.
    #[arg(long, value_name = "EPOCH_MS")]
    pub start_time: Option<String>,

    /// Last day the responder is active; Gmail expects epoch
    /// milliseconds.
    #[arg(long, value_name = "EPOCH_MS")]
    pub end_time: Option<String>,
}

impl GmailSettingsVacationSetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let mut settings = {
            let c = GmailVacationGet::new(&client.auth, &client.user_id)?;
            client.run(c)?.response
        };

        if let Some(enabled) = enabled_flag(self.enable, self.disable) {
            settings.enable_auto_reply = enabled;
        }
        if let Some(subject) = self.subject {
            settings.response_subject = Some(subject);
        }
        if let Some(body) = self.body {
            settings.response_body_plain_text = Some(body);
        }
        if let Some(html) = self.html {
            settings.response_body_html = Some(html);
        }
        if let Some(restrict) = self.restrict_to_contacts {
            settings.restrict_to_contacts = Some(restrict);
        }
        if let Some(restrict) = self.restrict_to_domain {
            settings.restrict_to_domain = Some(restrict);
        }
        if let Some(start) = self.start_time {
            settings.start_time = Some(start);
        }
        if let Some(end) = self.end_time {
            settings.end_time = Some(end);
        }

        let _out = {
            let c = GmailVacationUpdate::new(&client.auth, &client.user_id, settings)?;
            client.run(c)?
        };

        printer.out(Message::new("Gmail vacation settings successfully updated"))
    }
}

/// Gmail vacation responder settings, rendered as aligned text or, under
/// `--json`, as a structured object instead of a wrapped human string.
///
/// The booleans stay booleans in JSON, where the text rendering spells
/// them enabled and disabled, or yes and no.
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
