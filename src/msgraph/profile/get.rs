//! # Microsoft Graph profile get
//!
//! The `msgraph profile get` command, fetching the signed-in user.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::msgraph::client::MsgraphClient;

/// Get the Microsoft Graph user profile: id, display name, mail and user
/// principal name.
#[derive(Debug, Parser)]
pub struct MsgraphProfileGetCommand;

impl MsgraphProfileGetCommand {
    /// Fetches the signed-in user and prints the profile.
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
/// `--json`, as a structured object (`{id, displayName, mail,
/// userPrincipalName}`) instead of a wrapped human string.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MsgraphProfileOutput {
    id: String,
    display_name: Option<String>,
    mail: Option<String>,
    user_principal_name: Option<String>,
}

impl fmt::Display for MsgraphProfileOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = Fields::default();
        fields.push("Id", sanitize::text(&self.id));
        if let Some(display_name) = &self.display_name {
            fields.push("Display name", sanitize::text(display_name));
        }
        if let Some(mail) = &self.mail {
            fields.push("Mail", sanitize::text(mail));
        }
        if let Some(upn) = &self.user_principal_name {
            fields.push("User principal name", sanitize::text(upn));
        }
        write!(f, "{fields}")
    }
}
