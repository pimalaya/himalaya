use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::{
    rfc8621::coroutines::identity_set::{
        JmapIdentitySet, JmapIdentitySetArgs, JmapIdentitySetResult,
    },
    rfc8621::types::identity::IdentityUpdate,
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

        let mut args = JmapIdentitySetArgs::default();
        args.update(self.id.clone(), patch);

        let mut coroutine = JmapIdentitySet::new(&jmap.session, &jmap.http_auth, args)?;
        let mut arg = None;

        let errs = loop {
            match coroutine.resume(arg.take()) {
                JmapIdentitySetResult::Io { io } => arg = Some(handle(&mut jmap.stream, io)?),
                JmapIdentitySetResult::Ok { not_updated, .. } => break not_updated,
                JmapIdentitySetResult::Err { err, .. } => bail!(err),
            }
        };

        if let Some(err) = errs.get(&self.id) {
            let mut ctx = anyhow!("Update identity `{}` error", &self.id);

            if let Some(desc) = &err.description {
                ctx = anyhow!("{desc}").context(ctx);
            }

            if !err.properties.is_empty() {
                let props = err.properties.join(", ");
                ctx = anyhow!("Invalid properties {props}").context(ctx);
            }

            bail!(ctx);
        }

        printer.out(Message::new("Identity successfully updated"))
    }
}
