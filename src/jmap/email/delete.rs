use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::coroutines::email_set::{EmailSetArgs, SetJmapEmails, SetJmapEmailsResult};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

/// Delete JMAP emails (Email/set destroy).
#[derive(Debug, Parser)]
pub struct DeleteEmailCommand {
    /// Email ID(s) to delete.
    #[arg(value_name = "ID", required = true, num_args = 1..)]
    pub ids: Vec<String>,
}

impl DeleteEmailCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let mut args = EmailSetArgs::default();

        for id in self.ids {
            args.destroy(id);
        }

        let mut coroutine = SetJmapEmails::new(jmap.context, args)?;
        let mut arg = None;

        let not_destroyed = loop {
            match coroutine.resume(arg.take()) {
                SetJmapEmailsResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                SetJmapEmailsResult::Ok { not_destroyed, .. } => break not_destroyed,
                SetJmapEmailsResult::Err { err, .. } => bail!(err),
            }
        };

        for (id, err) in &not_destroyed {
            let mut ctx = anyhow!("Failed to delete email `{id}`");

            if let Some(desc) = &err.description {
                ctx = anyhow!(desc.clone()).context(ctx);
            }

            bail!(ctx);
        }

        printer.out(Message::new("Email(s) successfully deleted"))
    }
}
