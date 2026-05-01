use std::{
    collections::BTreeMap,
    io::{Read, Write},
};

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::{
    mailbox::MailboxUpdate,
    mailbox_set::{JmapMailboxSet, JmapMailboxSetArgs, JmapMailboxSetResult},
};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::{account::JmapAccount, error::format_set_error, mailbox::query::RoleArg};

const READ_BUFFER_SIZE: usize = 16 * 1024;

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

        let mut update = BTreeMap::new();
        update.insert(self.id.clone(), patch);

        let mut args = JmapMailboxSetArgs::default();
        args.update = Some(update);

        let mut coroutine = JmapMailboxSet::new(&jmap.session, &jmap.http_auth, args)?;
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let not_updated = loop {
            match coroutine.resume(arg.take()) {
                JmapMailboxSetResult::Ok { not_updated, .. } => break not_updated,
                JmapMailboxSetResult::WantsRead => {
                    let n = jmap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapMailboxSetResult::WantsWrite(bytes) => {
                    jmap.stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapMailboxSetResult::Err(err) => bail!("{err}"),
            }
        };

        if let Some(err) = not_updated.get(&self.id) {
            let mut msg = format!("Update JMAP mailbox `{}` error", self.id);
            msg.push_str(&format_set_error(err));
            bail!(msg);
        }

        printer.out(Message::new("Mailbox successfully updated"))
    }
}
