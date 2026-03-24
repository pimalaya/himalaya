use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::{
    coroutines::mailbox_set::{MailboxSetArgs, SetJmapMailboxes, SetJmapMailboxesResult},
    types::mailbox::MailboxCreate,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

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

        let mut create = HashMap::new();
        create.insert(self.name.clone(), new_mailbox);

        let mut args = MailboxSetArgs::default();
        args.create = Some(create);

        let mut coroutine = SetJmapMailboxes::new(jmap.context, args)?;
        let mut arg = None;

        let not_created = loop {
            match coroutine.resume(arg.take()) {
                SetJmapMailboxesResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                SetJmapMailboxesResult::Ok { not_created, .. } => break not_created,
                SetJmapMailboxesResult::Err { err, .. } => bail!(err),
            }
        };

        if let Some(err) = not_created.get(&self.name) {
            let mut ctx = anyhow!("Create JMAP mailbox `{}` error", self.name);

            if let Some(desc) = &err.description {
                ctx = anyhow!(desc.clone()).context(ctx);
            }

            if !err.properties.is_empty() {
                ctx = anyhow!("Invalid properties: {}", err.properties.join(", ")).context(ctx);
            }

            bail!(ctx);
        }

        printer.out(Message::new("Mailbox successfully created"))
    }
}
