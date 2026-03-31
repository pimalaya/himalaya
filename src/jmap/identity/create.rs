use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::{
    rfc8621::coroutines::identity_set::{
        JmapIdentitySet, JmapIdentitySetArgs, JmapIdentitySetResult,
    },
    rfc8621::types::identity::IdentityCreate,
};
use io_stream::runtimes::std::handle;
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::account::JmapAccount;

/// Create a JMAP sender identity (Identity/set).
#[derive(Debug, Parser)]
pub struct JmapIdentityCreateCommand {
    /// Display name for the sender.
    pub name: String,

    /// Email address for the sender.
    pub email: String,

    /// Plaintext signature to append to outgoing emails.
    #[arg(long)]
    pub text_signature: Option<String>,

    /// HTML signature to append to outgoing emails.
    #[arg(long)]
    pub html_signature: Option<String>,
}

impl JmapIdentityCreateCommand {
    pub fn execute(self, printer: &mut impl Printer, account: JmapAccount) -> Result<()> {
        let mut jmap = account.new_jmap_session()?;

        let identity = IdentityCreate {
            name: self.name.clone(),
            email: self.email.clone(),
            reply_to: None,
            bcc: None,
            text_signature: self.text_signature,
            html_signature: self.html_signature,
        };

        let create_id = "new";

        let mut args = JmapIdentitySetArgs::default();
        args.create(create_id, identity);

        let mut coroutine = JmapIdentitySet::new(&jmap.session, &jmap.http_auth, args)?;
        let mut arg = None;

        let not_created = loop {
            match coroutine.resume(arg.take()) {
                JmapIdentitySetResult::Io { io } => arg = Some(handle(&mut jmap.stream, io)?),
                JmapIdentitySetResult::Ok { not_created, .. } => break not_created,
                JmapIdentitySetResult::Err { err, .. } => bail!(err),
            }
        };

        if let Some(err) = not_created.get(create_id) {
            let mut msg = format!("Create identity for `{}` error", self.email);

            if !err.properties.is_empty() {
                msg.push_str(": invalid propertie(s) `");
                msg.push_str(&err.properties.join("`, `"));
                msg.push('`');
            }

            if let Some(desc) = &err.description {
                msg.push_str(" (");
                msg.push_str(desc.to_lowercase().trim_end_matches(['.', '\n']));
                msg.push_str(")");
            }

            bail!(msg);
        }

        printer.out(Message::new("Identity successfully created"))
    }
}
