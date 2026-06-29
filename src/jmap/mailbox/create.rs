use std::collections::BTreeMap;

use anyhow::{Result, bail};
use clap::Parser;
use io_jmap::rfc8621::mailbox::{JmapMailboxCreate, set::JmapMailboxSetArgs};
use pimalaya_cli::printer::{Message, Printer};

use crate::jmap::{client::JmapClient, error::format_set_error};

/// Create a JMAP mailbox.
#[derive(Debug, Parser)]
pub struct JmapMailboxCreateCommand {
    /// The name of the new mailbox.
    #[arg(value_name = "NAME")]
    pub name: String,

    /// Attach the new mailbox to the parent mailbox matching the
    /// given identifier.
    #[arg(long, value_name = "ID")]
    pub parent_id: Option<String>,

    /// Should subscribe to the new mailbox.
    #[arg(long)]
    pub subscribe: bool,
}

impl JmapMailboxCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let new_mailbox = JmapMailboxCreate {
            name: Some(self.name.clone()),
            parent_id: self.parent_id,
            is_subscribed: if self.subscribe { Some(true) } else { None },
            ..Default::default()
        };

        let mut create = BTreeMap::new();
        create.insert(self.name.clone(), new_mailbox);

        let args = JmapMailboxSetArgs {
            create: Some(create),
            ..Default::default()
        };

        let output = client.mailbox_set(args)?;

        if let Some(err) = output.not_created.get(&self.name) {
            let mut msg = format!("Create JMAP mailbox `{}` error", self.name);
            msg.push_str(&format_set_error(err));
            bail!(msg)
        }

        printer.out(Message::new("Mailbox successfully created"))
    }
}
