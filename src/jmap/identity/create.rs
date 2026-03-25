use anyhow::{anyhow, bail, Result};
use clap::Parser;
use io_jmap::{
    coroutines::identity_set::{IdentitySetArgs, SetJmapIdentities, SetJmapIdentitiesResult},
    types::identity::IdentityCreate,
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

        let mut args = IdentitySetArgs::default();
        args.create(create_id, identity);

        let mut coroutine = SetJmapIdentities::new(jmap.context, args)?;
        let mut arg = None;

        let errs = loop {
            match coroutine.resume(arg.take()) {
                SetJmapIdentitiesResult::Io(io) => arg = Some(handle(&mut jmap.stream, io)?),
                SetJmapIdentitiesResult::Ok { not_created, .. } => break not_created,
                SetJmapIdentitiesResult::Err { err, .. } => bail!(err),
            }
        };

        if let Some(err) = errs.get(create_id) {
            let mut ctx = anyhow!("Create identity for `{}` error", &self.email);

            if let Some(desc) = &err.description {
                ctx = anyhow!("{desc}").context(ctx);
            }

            if !err.properties.is_empty() {
                let props = err.properties.join(", ");
                ctx = anyhow!("Invalid properties {props}").context(ctx);
            }

            bail!(ctx);
        }

        printer.out(Message::new("Identity successfully created"))
    }
}
