use anyhow::Result;
use clap::{Parser, Subcommand};
use io_gmail::v1::rest::settings::{
    get_vacation::GmailVacationGet, update_vacation::GmailVacationUpdate,
};
use pimalaya_cli::printer::{Message, Printer};

use crate::{
    account::context::Account,
    gmail::{client::GmailClient, settings::convert::enabled_flag},
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

        let mut text = String::new();
        text.push_str(&format!(
            "Auto reply: {}\n",
            if settings.enable_auto_reply {
                "enabled"
            } else {
                "disabled"
            }
        ));
        if let Some(subject) = settings.response_subject {
            text.push_str(&format!("Subject: {subject}\n"));
        }
        if let Some(body) = settings.response_body_plain_text {
            text.push_str(&format!("Body: {body}\n"));
        }
        if let Some(html) = settings.response_body_html {
            text.push_str(&format!("HTML: {html}\n"));
        }
        if let Some(restrict) = settings.restrict_to_contacts {
            text.push_str(&format!(
                "Restrict to contacts: {}\n",
                if restrict { "yes" } else { "no" }
            ));
        }
        if let Some(restrict) = settings.restrict_to_domain {
            text.push_str(&format!(
                "Restrict to domain: {}\n",
                if restrict { "yes" } else { "no" }
            ));
        }
        if let Some(start) = settings.start_time {
            text.push_str(&format!("Start: {start}\n"));
        }
        if let Some(end) = settings.end_time {
            text.push_str(&format!("End: {end}\n"));
        }

        printer.out(Message::new(text))
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
