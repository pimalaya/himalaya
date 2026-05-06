use anyhow::{bail, Result};
use clap::Parser;
use io_jmap::rfc8621::{identity::IdentityCreate, identity_set::JmapIdentitySetArgs};
use pimalaya_cli::printer::{Message, Printer};

use crate::jmap::{client::JmapClient, error::format_set_error};

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
    pub fn execute(self, printer: &mut impl Printer, mut client: JmapClient) -> Result<()> {
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

        let output = client.identity_set(args)?;

        if let Some(err) = output.not_created.get(create_id) {
            let mut msg = format!("Create identity for `{}` error", self.email);
            msg.push_str(&format_set_error(err));
            bail!(msg);
        }

        printer.out(Message::new("Identity successfully created"))
    }
}
