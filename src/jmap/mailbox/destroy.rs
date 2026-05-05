use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::mailbox_set::JmapMailboxSetArgs;
use pimalaya_cli::printer::{Message, Printer};

use crate::jmap::{account::JmapAccount, error::format_set_error};

/// Delete a JMAP mailbox.
#[derive(Debug, Parser)]
pub struct JmapMailboxDestroyCommand {
    /// The ID of the mailbox to delete.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,

    /// Destroy all emails in the mailbox when deleting.
    #[arg(long, default_value_t)]
    pub purge: bool,
}

impl JmapMailboxDestroyCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut client = account.new_jmap_client()?;

        let mut args = JmapMailboxSetArgs::default();
        args.destroy = Some(self.ids.clone());
        args.on_destroy_remove_emails = if self.purge { Some(true) } else { None };

        let output = client.mailbox_set(args)?;

        if !output.not_destroyed.is_empty() {
            let mut msg = String::from("Destroy JMAP mailbox(es) error");

            for (id, err) in output.not_destroyed {
                msg.push_str(&format!("\n  `{id}`"));
                msg.push_str(&format_set_error(&err));
            }

            bail!(msg)
        }

        printer.out(Message::new("Mailbox successfully deleted"))
    }
}
