use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::msgraph::client::MsgraphClient;

/// Get the Microsoft Graph user profile: id, display name, mail and user
/// principal name.
#[derive(Debug, Parser)]
pub struct MsgraphProfileGetCommand;

impl MsgraphProfileGetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let user = client.me()?.response;

        printer.out(MsgraphProfileOutput {
            id: user.id,
            display_name: user.display_name,
            mail: user.mail,
            user_principal_name: user.user_principal_name,
        })
    }
}

/// Microsoft Graph user profile, rendered as aligned text or, under
/// `--json`, as a structured object (`{id, display-name, mail,
/// user-principal-name}`) instead of a wrapped human string.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct MsgraphProfileOutput {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_principal_name: Option<String>,
}

impl fmt::Display for MsgraphProfileOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Id: {}", self.id)?;
        if let Some(display_name) = &self.display_name {
            writeln!(f, "Display name: {display_name}")?;
        }
        if let Some(mail) = &self.mail {
            writeln!(f, "Mail: {mail}")?;
        }
        if let Some(upn) = &self.user_principal_name {
            writeln!(f, "User principal name: {upn}")?;
        }
        Ok(())
    }
}
