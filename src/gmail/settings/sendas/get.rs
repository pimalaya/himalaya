//! # Gmail send-as get
//!
//! The `gmail settings sendas get` command,
//! `users.settings.sendAs.get`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_gmail::v1::rest::settings::send_as::{GmailSendAs, get::GmailSendAsGet};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::gmail::{client::GmailClient, settings::convert::verification_status_wire};

/// Get one Gmail send-as alias by e-mail address (settings.sendAs.get).
#[derive(Debug, Parser)]
pub struct GmailSettingsSendAsGetCommand {
    /// E-mail address of the send-as alias to get.
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

impl GmailSettingsSendAsGetCommand {
    /// Fetches the send-as alias and tables it.
    pub fn execute(self, printer: &mut impl Printer, client: &mut GmailClient) -> Result<()> {
        let out = {
            let c = GmailSendAsGet::new(&client.auth, &client.user_id, &self.email)?;
            client.run(c)?
        };
        let send_as = out.response;

        printer.out(GmailSettingsSendAsGetOutput(send_as))
    }
}

/// The `gmail settings sendas get` output.
///
/// The resource is emitted verbatim, so one alias has the very same shape
/// here as in a `list` row.
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub(crate) struct GmailSettingsSendAsGetOutput(GmailSendAs);

impl fmt::Display for GmailSettingsSendAsGetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Email: {}", self.0.send_as_email)?;

        if let Some(display_name) = &self.0.display_name {
            writeln!(f, "Name: {display_name}")?;
        }
        if let Some(reply_to_address) = &self.0.reply_to_address {
            writeln!(f, "Reply-To: {reply_to_address}")?;
        }
        if let Some(signature) = &self.0.signature {
            writeln!(f, "Signature: {signature}")?;
        }
        if let Some(is_primary) = self.0.is_primary {
            writeln!(f, "Primary: {is_primary}")?;
        }
        if let Some(is_default) = self.0.is_default {
            writeln!(f, "Default: {is_default}")?;
        }
        if let Some(treat_as_alias) = self.0.treat_as_alias {
            writeln!(f, "Treat as alias: {treat_as_alias}")?;
        }
        if let Some(status) = self.0.verification_status {
            writeln!(f, "Verification: {}", verification_status_wire(status))?;
        }

        Ok(())
    }
}
