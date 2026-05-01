use std::io::{Read, Write};

use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::{
    identity::IdentityUpdate,
    identity_set::{JmapIdentitySet, JmapIdentitySetArgs, JmapIdentitySetResult},
};
use pimalaya_toolbox::terminal::printer::{Message, Printer};

use crate::jmap::{account::JmapAccount, error::format_set_error};

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Update a JMAP sender identity (Identity/set).
#[derive(Debug, Parser)]
pub struct JmapIdentityUpdateCommand {
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

impl JmapIdentityUpdateCommand {
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
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        let not_updated = loop {
            match coroutine.resume(arg.take()) {
                JmapIdentitySetResult::Ok { not_updated, .. } => break not_updated,
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

        if let Some(err) = not_updated.get(&self.id) {
            let mut msg = format!("Update identity `{}` error", self.id);
            msg.push_str(&format_set_error(err));
            bail!(msg);
        }

        printer.out(Message::new("Identity successfully updated"))
    }
}
