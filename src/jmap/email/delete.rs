use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::coroutines::email_set::{JmapEmailSet, JmapEmailSetArgs, JmapEmailSetResult};
use io_socket::runtimes::std_stream::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

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
        let mut arg = None;

        let not_destroyed = loop {
            match coroutine.resume(arg.take()) {
                JmapEmailSetResult::Io { io } => arg = Some(handle(&mut jmap.stream, io)?),
                JmapEmailSetResult::Ok { not_destroyed, .. } => break not_destroyed,
                JmapEmailSetResult::Err { err, .. } => bail!(err),
            }
        };

        if !not_destroyed.is_empty() {
            let mut msg = String::from("Destroy JMAP email(s) error");

            for (id, err) in not_destroyed {
                msg.push_str(&format!("\n  `{id}`"));

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
            }

            bail!(msg)
        }

        printer.out(Message::new("Email(s) successfully deleted"))
    }
}
