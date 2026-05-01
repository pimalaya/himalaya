use std::{
    collections::BTreeMap,
    io::{Read, Write},
};

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::{
    mailbox::MailboxCreate,
    mailbox_set::{JmapMailboxSet, JmapMailboxSetArgs, JmapMailboxSetResult},
};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::{account::JmapAccount, error::format_set_error};

const READ_BUFFER_SIZE: usize = 16 * 1024;

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
    #[arg(long, value_name = "NAME")]
    pub subscribe: bool,
}

impl JmapMailboxCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let new_mailbox = MailboxCreate {
            name: Some(self.name.clone()),
            parent_id: self.parent_id,
            is_subscribed: if self.subscribe { Some(true) } else { None },
            ..Default::default()
        };

        let mut create = BTreeMap::new();
        create.insert(self.name.clone(), new_mailbox);

        let mut args = JmapMailboxSetArgs::default();
        args.create = Some(create);

        let mut coroutine = JmapMailboxSet::new(&jmap.session, &jmap.http_auth, args)?;
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let not_created = loop {
            match coroutine.resume(arg.take()) {
                JmapMailboxSetResult::Ok { not_created, .. } => break not_created,
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

        if let Some(err) = not_created.get(&self.name) {
            let mut msg = format!("Create JMAP mailbox `{}` error", self.name);
            msg.push_str(&format_set_error(err));
            bail!(msg)
        }

        printer.out(Message::new("Mailbox successfully created"))
    }
}
