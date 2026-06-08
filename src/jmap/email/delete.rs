use anyhow::{Result, bail};
use clap::Parser;
use io_jmap::rfc8621::email::set::JmapEmailSetArgs;
use pimalaya_cli::printer::{Message, Printer};

use crate::jmap::{client::JmapClient, error::format_set_error};

/// Delete JMAP emails (Email/set destroy).
#[derive(Debug, Parser)]
pub struct JmapEmailDestroyCommand {
    /// Email ID(s) to delete.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapEmailDestroyCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let mut args = JmapEmailSetArgs::default();

        for id in self.ids {
            args.destroy(id);
        }

        let output = client.email_set(args)?;

        if !output.not_destroyed.is_empty() {
            let mut msg = String::from("Destroy JMAP email(s) error");

            for (id, err) in output.not_destroyed {
                msg.push_str(&format!("\n  `{id}`"));
                msg.push_str(&format_set_error(&err));
            }

            bail!(msg)
        }

        printer.out(Message::new("Email(s) successfully deleted"))
    }
}
