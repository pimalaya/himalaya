use anyhow::{Result, bail};
use clap::Parser;
use io_jmap::rfc8621::identity::{JmapIdentityUpdate, set::JmapIdentitySetArgs};
use pimalaya_cli::printer::{Message, Printer};

use crate::jmap::{client::JmapClient, error::format_set_error};

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
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let patch = JmapIdentityUpdate {
            name: self.name,
            reply_to: None,
            bcc: None,
            text_signature: self.text_signature,
            html_signature: self.html_signature,
        };

        let mut args = JmapIdentitySetArgs::default();
        args.update(self.id.clone(), patch);

        let output = client.identity_set(args)?;

        if let Some(err) = output.not_updated.get(&self.id) {
            let mut msg = format!("Update identity `{}` error", self.id);
            msg.push_str(&format_set_error(err));
            bail!(msg);
        }

        printer.out(Message::new("Identity successfully updated"))
    }
}
