use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::{
    identity::IdentityCreate,
    identity_set::{JmapIdentitySet, JmapIdentitySetArgs, JmapIdentitySetResult},
};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::{account::JmapAccount, error::format_set_error};

const READ_BUFFER_SIZE: usize = 16 * 1024;

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
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let not_created = loop {
            match coroutine.resume(arg.take()) {
                JmapIdentitySetResult::Ok { not_created, .. } => break not_created,
                JmapIdentitySetResult::WantsRead => {
                    let n = jmap.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                JmapIdentitySetResult::WantsWrite(bytes) => {
                    jmap.stream.write_all(&bytes)?;
                    arg = None;
                }
                JmapIdentitySetResult::Err(err) => bail!("{err}"),
            }
        };

        if let Some(err) = not_created.get(create_id) {
            let mut msg = format!("Create identity for `{}` error", self.email);
            msg.push_str(&format_set_error(err));
            bail!(msg);
        }

        printer.out(Message::new("Identity successfully created"))
    }
}
