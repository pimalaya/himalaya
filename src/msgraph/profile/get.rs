use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::{Message, Printer};

use crate::msgraph::client::MsgraphClient;

/// Get the Microsoft Graph user profile: id, display name, mail and user
/// principal name.
#[derive(Debug, Parser)]
pub struct MsgraphProfileGetCommand;

impl MsgraphProfileGetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let user = client.me()?.response;

        let mut out = String::new();
        out.push_str(&format!("Id: {}\n", user.id));
        if let Some(display_name) = &user.display_name {
            out.push_str(&format!("Display name: {display_name}\n"));
        }
        if let Some(mail) = &user.mail {
            out.push_str(&format!("Mail: {mail}\n"));
        }
        if let Some(upn) = &user.user_principal_name {
            out.push_str(&format!("User principal name: {upn}\n"));
        }

        printer.out(Message::new(out))
    }
}
