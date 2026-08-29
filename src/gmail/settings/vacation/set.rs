//! # Gmail vacation set
//!
//! The `gmail settings vacation set` command,
//! `users.settings.updateVacation`.

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::{
    get_vacation::GmailVacationGet, update_vacation::GmailVacationUpdate,
};
use pimalaya_cli::printer::{Message, Printer};

use crate::gmail::{client::GmailClient, settings::convert::enabled_flag};

/// Update the Gmail vacation responder settings.
///
/// A partial update: the settings are fetched first and only the options
/// passed are changed, so the rest survives. The responder itself is
/// toggled with `--enable` and `--disable`, never by accident.
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
    /// Applies the vacation responder settings.
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
