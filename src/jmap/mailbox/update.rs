use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::{
    coroutines::mailbox_set::{MailboxSetArgs, SetJmapMailboxes, SetJmapMailboxesResult},
    types::mailbox::MailboxUpdate,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::{account::JmapAccount, mailbox::query::RoleArg};

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

    /// New role.
    #[arg(long, value_name = "ROLE")]
    pub role: Option<RoleArg>,

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
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let is_subscribed = if self.subscribe {
            Some(true)
        } else if self.unsubscribe {
            Some(false)
        } else {
            None
        };

        let patch = MailboxUpdate {
            name: self.name,
            parent_id: self.parent_id,
            role: self.role.map(Into::into),
            sort_order: self.sort_order,
            is_subscribed,
        };

        let mut update = HashMap::new();
        update.insert(self.id.clone(), patch);

        let mut args = MailboxSetArgs::default();
        args.update = Some(update);

        let mut arg = None;
        let mut coroutine = SetJmapMailboxes::new(jmap.context, args)?;

        let not_updated = loop {
            match coroutine.resume(arg.take()) {
                SetJmapMailboxesResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                SetJmapMailboxesResult::Ok { not_updated, .. } => break not_updated,
                SetJmapMailboxesResult::Err { err, .. } => bail!(err),
            }
        };

        if let Some(err) = not_updated.get(&self.id) {
            let mut ctx = anyhow!("Update JMAP mailbox `{}` error", self.id);

            if let Some(desc) = &err.description {
                ctx = anyhow!(desc.clone()).context(ctx);
            }

            if !err.properties.is_empty() {
                ctx = anyhow!("Invalid properties: {}", err.properties.join(", ")).context(ctx);
            }

            bail!(ctx);
        }

        printer.out(Message::new("Mailbox successfully updated"))
    }
}
