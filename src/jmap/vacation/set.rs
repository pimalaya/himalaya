use anyhow::{Result, bail};
use clap::Parser;
use io_jmap::rfc8621::vacation_response::{
    JmapVacationResponseUpdate, VACATION_RESPONSE_CAPABILITY,
};
use pimalaya_cli::printer::{Message, Printer};

use crate::jmap::client::JmapClient;

/// Update the JMAP vacation response (VacationResponse/set).
#[derive(Debug, Parser)]
pub struct JmapVacationSetCommand {
    /// Enable the vacation response.
    #[arg(long, conflicts_with = "disable")]
    pub enable: bool,

    /// Disable the vacation response.
    #[arg(long, conflicts_with = "enable")]
    pub disable: bool,

    /// Active from date (RFC 3339).
    #[arg(long, value_name = "DATE")]
    pub from_date: Option<String>,

    /// Active until date (RFC 3339).
    #[arg(long, value_name = "DATE")]
    pub to_date: Option<String>,

    /// Subject line for the auto-reply.
    #[arg(long, value_name = "TEXT")]
    pub subject: Option<String>,

    /// Plaintext body for the auto-reply.
    #[arg(long, value_name = "TEXT")]
    pub text_body: Option<String>,

    /// HTML body for the auto-reply.
    #[arg(long, value_name = "TEXT")]
    pub html_body: Option<String>,
}

impl JmapVacationSetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let has_vacation = client
            .session()
            .map(|s| s.capabilities.contains_key(VACATION_RESPONSE_CAPABILITY))
            .unwrap_or(false);

        if !has_vacation {
            bail!("Vacation response is not supported by the server");
        }

        let is_enabled = if self.enable {
            Some(true)
        } else if self.disable {
            Some(false)
        } else {
            None
        };

        let patch = JmapVacationResponseUpdate {
            is_enabled,
            from_date: self.from_date,
            to_date: self.to_date,
            subject: self.subject,
            text_body: self.text_body,
            html_body: self.html_body,
        };

        client.vacation_response_set(patch)?;

        printer.out(Message::new("Vacation response successfully updated"))
    }
}
