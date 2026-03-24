use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::coroutines::mailbox_set::{MailboxSetArgs, SetJmapMailboxes, SetJmapMailboxesResult};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

/// Delete a JMAP mailbox.
#[derive(Debug, Parser)]
pub struct JmapMailboxDestroyCommand {
    /// The ID of the mailbox to delete.
    #[arg(value_name = "ID", required = true, num_args = 1..)]
    pub ids: Vec<String>,

    /// Destroy all emails in the mailbox when deleting.
    #[arg(long, default_value_t)]
    pub purge: bool,
}

impl JmapMailboxDestroyCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut args = MailboxSetArgs::default();
        args.destroy = Some(self.ids.clone());
        args.on_destroy_remove_emails = if self.purge { Some(true) } else { None };

        let mut arg = None;
        let mut coroutine = SetJmapMailboxes::new(jmap.context, args)?;

        let not_destroyed = loop {
            match coroutine.resume(arg.take()) {
                SetJmapMailboxesResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                SetJmapMailboxesResult::Ok { not_destroyed, .. } => break not_destroyed,
                SetJmapMailboxesResult::Err { err, .. } => bail!(err),
            }
        };

        for ref id in self.ids {
            if let Some(err) = not_destroyed.get(id) {
                let mut ctx = anyhow!("Update JMAP mailbox `{id}` error");

                if let Some(desc) = &err.description {
                    ctx = anyhow!(desc.clone()).context(ctx);
                }

                if !err.properties.is_empty() {
                    ctx = anyhow!("Invalid properties: {}", err.properties.join(", ")).context(ctx);
                }

                bail!(ctx);
            }
        }

        printer.out(Message::new("Mailbox successfully deleted"))
    }
}
