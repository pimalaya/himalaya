use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::email_set::{JmapEmailSet, JmapEmailSetArgs, JmapEmailSetResult};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::{account::JmapAccount, error::format_set_error};

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Delete JMAP emails (Email/set destroy).
#[derive(Debug, Parser)]
pub struct JmapEmailDestroyCommand {
    /// Email ID(s) to delete.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl JmapEmailDestroyCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut args = JmapEmailSetArgs::default();

        for id in self.ids {
            args.destroy(id);
        }

        let mut coroutine = JmapEmailSet::new(&jmap.session, &jmap.http_auth, args)?;
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let not_destroyed = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailSetResult::Ok { not_destroyed, .. } => break not_destroyed,
                JmapEmailSetResult::WantsRead => {
                    let n = jmap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapEmailSetResult::WantsWrite(bytes) => {
                    jmap.stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapEmailSetResult::Err(err) => bail!("{err}"),
            }
        };

        if !not_destroyed.is_empty() {
            let mut msg = String::from("Destroy JMAP email(s) error");

            for (id, err) in not_destroyed {
                msg.push_str(&format!("\n  `{id}`"));
                msg.push_str(&format_set_error(&err));
            }

            bail!(msg)
        }

        printer.out(Message::new("Email(s) successfully deleted"))
    }
}
