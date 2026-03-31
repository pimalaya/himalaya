use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::coroutines::identity_set::{
    JmapIdentitySet, JmapIdentitySetArgs, JmapIdentitySetResult,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

/// Delete a JMAP sender identity (Identity/set).
#[derive(Debug, Parser)]
pub struct DeleteIdentityCommand {
    /// Identity ID(s) to delete.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
}

impl DeleteIdentityCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut args = JmapIdentitySetArgs::default();

        for id in self.ids {
            args.destroy(id);
        }

        let mut coroutine = JmapIdentitySet::new(&jmap.session, &jmap.http_auth, args)?;
        let mut arg = None;

        let not_destroyed = loop {
            match coroutine.resume(arg.take()) {
                JmapIdentitySetResult::Io { io } => arg = Some(handle(&mut jmap.stream, io)?),
                JmapIdentitySetResult::Ok { not_destroyed, .. } => break not_destroyed,
                JmapIdentitySetResult::Err { err, .. } => bail!(err),
            }
        };

        if !not_destroyed.is_empty() {
            let mut msg = String::from("Destroy JMAP identities error");

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

        printer.out(Message::new("Identity successfully deleted"))
    }
}
