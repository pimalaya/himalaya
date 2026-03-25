use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::coroutines::identity_set::{
    IdentitySetArgs, SetJmapIdentities, SetJmapIdentitiesResult,
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

        let mut args = IdentitySetArgs::default();

        for id in self.ids {
            args.destroy(id);
        }

        let mut coroutine = SetJmapIdentities::new(jmap.context, args)?;
        let mut arg = None;

        let not_destroyed = loop {
            match coroutine.resume(arg.take()) {
                SetJmapIdentitiesResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                SetJmapIdentitiesResult::Ok { not_destroyed, .. } => break not_destroyed,
                SetJmapIdentitiesResult::Err { err, .. } => bail!(err),
            }
        };

        if !not_destroyed.is_empty() {
            let mut ctx = anyhow!("Destroy JMAP identities error");

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

        printer.out(Message::new("Identity successfully deleted"))
    }
}
