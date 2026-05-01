use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::mailbox_set::{JmapMailboxSet, JmapMailboxSetArgs, JmapMailboxSetResult};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::{account::JmapAccount, error::format_set_error};

const READ_BUFFER_SIZE: usize = 16 * 1024;

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

        let mut coroutine = JmapMailboxSet::new(&jmap.session, &jmap.http_auth, args)?;
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let not_destroyed = loop {
            match coroutine.resume(arg.take()) {
                JmapMailboxSetResult::Ok { not_destroyed, .. } => break not_destroyed,
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

        if !not_destroyed.is_empty() {
            let mut msg = String::from("Destroy JMAP mailbox(es) error");

            for (id, err) in not_destroyed {
                msg.push_str(&format!("\n  `{id}`"));
                msg.push_str(&format_set_error(&err));
            }

            bail!(msg)
        }

        printer.out(Message::new("Mailbox successfully deleted"))
    }
}
