use anyhow::Result;
use clap::{Parser, Subcommand};
use io_gmail::v1::rest::settings::{
    GmailVacationSettings, get_vacation::GmailVacationGet, update_vacation::GmailVacationUpdate,
};
use pimalaya_cli::printer::{Message, Printer};

use crate::{account::context::Account, gmail::client::GmailClient};

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
#[derive(Debug, Parser)]
pub struct GmailSettingsVacationSetCommand {
    #[arg(long)]
    pub enable: bool,

    #[arg(long)]
    pub subject: Option<String>,

    #[arg(long)]
    pub body: Option<String>,

    #[arg(long)]
    pub html: Option<String>,

    #[arg(long)]
    pub restrict_to_contacts: bool,

    #[arg(long)]
    pub restrict_to_domain: bool,

    #[arg(long)]
    pub start_time: Option<String>,

    #[arg(long)]
    pub end_time: Option<String>,
}

impl GmailSettingsVacationSetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let settings = GmailVacationSettings {
            enable_auto_reply: self.enable,
            response_subject: self.subject,
            response_body_plain_text: self.body,
            response_body_html: self.html,
            restrict_to_contacts: self.restrict_to_contacts.then_some(true),
            restrict_to_domain: self.restrict_to_domain.then_some(true),
            start_time: self.start_time,
            end_time: self.end_time,
        };

        let _out = {
            let c = GmailVacationUpdate::new(&client.auth, &client.user_id, settings)?;
            client.run(c)?
        };

        printer.out(Message::new("Gmail vacation settings successfully updated"))
    }
}
