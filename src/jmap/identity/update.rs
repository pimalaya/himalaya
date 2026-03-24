use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::{
    coroutines::identity_set::{IdentitySetArgs, SetJmapIdentities, SetJmapIdentitiesResult},
    types::identity::IdentityUpdate,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

/// Update a JMAP sender identity (Identity/set).
#[derive(Debug, Parser)]
pub struct UpdateIdentityCommand {
    /// Identity ID to update.
    pub id: String,

    /// New display name.
    #[arg(long)]
    pub name: Option<String>,

    /// New plaintext signature.
    #[arg(long)]
    pub text_signature: Option<String>,

    /// New HTML signature.
    #[arg(long)]
    pub html_signature: Option<String>,
}

impl UpdateIdentityCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let patch = IdentityUpdate {
            name: self.name,
            reply_to: None,
            bcc: None,
            text_signature: self.text_signature,
            html_signature: self.html_signature,
        };

        let mut args = IdentitySetArgs::default();
        args.update(self.id.clone(), patch);

        let mut coroutine = SetJmapIdentities::new(jmap.context, args)?;
        let mut arg = None;

        let not_updated = loop {
            match coroutine.resume(arg.take()) {
                SetJmapIdentitiesResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                SetJmapIdentitiesResult::Ok { not_updated, .. } => break not_updated,
                SetJmapIdentitiesResult::Err { err, .. } => bail!(err),
            }
        };

        if let Some(err) = not_updated.get(&self.id) {
            let mut ctx = anyhow!("Failed to update identity `{}`", self.id);

            if let Some(desc) = &err.description {
                ctx = anyhow!(desc.clone()).context(ctx);
            }

            bail!(ctx);
        }

        printer.out(Message::new("Identity successfully updated"))
    }
}
