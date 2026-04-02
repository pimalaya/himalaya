use std::collections::HashMap;

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::{
    rfc8621::coroutines::mailbox_set::{JmapMailboxSet, JmapMailboxSetArgs, JmapMailboxSetResult},
    rfc8621::types::mailbox::MailboxCreate,
};
use io_socket::runtimes::std_stream::handle;
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

        let mut args = JmapMailboxSetArgs::default();
        args.create = Some(create);

        let mut coroutine = JmapMailboxSet::new(&jmap.session, &jmap.http_auth, args)?;
        let mut arg = None;

        let not_created = loop {
            match coroutine.resume(arg.take()) {
                JmapMailboxSetResult::Io { io } => arg = Some(handle(&mut jmap.stream, io)?),
                JmapMailboxSetResult::Ok { not_created, .. } => break not_created,
                JmapMailboxSetResult::Err { err, .. } => bail!(err),
            }
        };

        if let Some(err) = not_created.get(&self.name) {
            let mut msg = format!("Create JMAP mailbox `{}` error", self.name);

            if !err.properties.is_empty() {
                msg.push_str(": invalid properties `");
                msg.push_str(&err.properties.join("`, `"));
                msg.push('`');
            }

            if let Some(desc) = &err.description {
                msg.push_str(" (");
                msg.push_str(desc.to_lowercase().trim_end_matches(['.', '\n']));
                msg.push(')');
            }

            bail!(msg)
        }

        printer.out(Message::new("Mailbox successfully created"))
    }
}
