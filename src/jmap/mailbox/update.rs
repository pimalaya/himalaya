use std::collections::BTreeMap;

use anyhow::{Result, bail};
use clap::Parser;
use io_jmap::rfc8621::mailbox::{JmapMailboxUpdate, set::JmapMailboxSetArgs};
use pimalaya_cli::printer::{Message, Printer};

use crate::jmap::{
    client::JmapClient,
    error::format_set_error,
    mailbox::query::{RoleArg, role_from_args},
};

/// Update a JMAP mailbox.
#[derive(Debug, Parser)]
pub struct JmapMailboxUpdateCommand {
    /// The ID of the mailbox to update.
    #[arg(value_name = "ID")]
    pub id: String,

    /// New display name.
    #[arg(long)]
    pub name: Option<String>,

    /// New parent mailbox ID.
    #[arg(long, value_name = "ID")]
    pub parent_id: Option<String>,

    /// Set a standard role.
    #[arg(long, value_name = "ROLE", conflicts_with = "custom_role")]
    pub role: Option<RoleArg>,

    /// Set a custom (non-standard) role.
    #[arg(long, value_name = "ROLE")]
    pub custom_role: Option<String>,

    /// New sort order.
    #[arg(long, value_name = "N")]
    pub sort_order: Option<u32>,

    /// Subscribe to the mailbox.
    #[arg(long, conflicts_with = "unsubscribe")]
    pub subscribe: bool,

    /// Unsubscribe from the mailbox.
    #[arg(long, conflicts_with = "subscribe")]
    pub unsubscribe: bool,
}

impl JmapMailboxUpdateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let is_subscribed = if self.subscribe {
            Some(true)
        } else if self.unsubscribe {
            Some(false)
        } else {
            None
        };

        let patch = JmapMailboxUpdate {
            name: self.name,
            parent_id: self.parent_id,
            role: role_from_args(self.role, self.custom_role),
            sort_order: self.sort_order,
            is_subscribed,
        };

        let mut update = BTreeMap::new();
        update.insert(self.id.clone(), patch);

        let args = JmapMailboxSetArgs {
            update: Some(update),
            ..Default::default()
        };

        let output = client.mailbox_set(args)?;

        if let Some(err) = output.not_updated.get(&self.id) {
            let mut msg = format!("Update JMAP mailbox `{}` error", self.id);
            msg.push_str(&format_set_error(err));
            bail!(msg);
        }

        printer.out(Message::new("Mailbox successfully updated"))
    }
}
