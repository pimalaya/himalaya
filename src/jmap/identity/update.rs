use anyhow::{bail, Result};
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

        let not_updated = loop {
            match coroutine.resume(arg.take()) {
                JmapIdentitySetResult::Io { io } => arg = Some(handle(&mut jmap.stream, io)?),
                JmapIdentitySetResult::Ok { not_updated, .. } => break not_updated,
                JmapIdentitySetResult::Err { err, .. } => bail!(err),
            }
        };

        if let Some(err) = not_updated.get(&self.id) {
            let mut msg = format!("Update identity `{}` error", self.id);

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

            bail!(msg);
        }

        printer.out(Message::new("Identity successfully updated"))
    }
}
