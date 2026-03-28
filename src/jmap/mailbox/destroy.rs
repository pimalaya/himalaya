use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::rfc8621::coroutines::mailbox_set::{
    JmapMailboxSet, JmapMailboxSetArgs, JmapMailboxSetResult,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

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
        let mut jmap = account.new_jmap_session()?;

        let mut args = JmapMailboxSetArgs::default();
        args.destroy = Some(self.ids.clone());
        args.on_destroy_remove_emails = if self.purge { Some(true) } else { None };

        let mut arg = None;
        let mut coroutine = JmapMailboxSet::new(&jmap.session, &jmap.http_auth, args)?;

        let not_destroyed = loop {
            match coroutine.resume(arg.take()) {
                JmapMailboxSetResult::Io { io } => arg = Some(handle(&mut jmap.stream, io)?),
                JmapMailboxSetResult::Ok { not_destroyed, .. } => break not_destroyed,
                JmapMailboxSetResult::Err { err, .. } => bail!(err),
            }
        };

        if !not_destroyed.is_empty() {
            let mut ctx = anyhow!("Destroy JMAP mailbox(es) error");

            for (id, err) in not_destroyed {
                if let Some(desc) = &err.description {
                    ctx = anyhow!("{id}: {desc}").context(ctx);
                }

                if !err.properties.is_empty() {
                    let props = err.properties.join(", ");
                    ctx = anyhow!("{id}: Invalid properties {props}").context(ctx);
                }
            }

            bail!(ctx)
        }

        printer.out(Message::new("Mailbox successfully deleted"))
    }
}
