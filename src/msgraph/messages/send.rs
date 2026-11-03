//! # Microsoft Graph message send
//!
//! The `msgraph messages send` command, sending raw MIME through
//! `sendMail`.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{msgraph::client::MsgraphClient, shared::message::arg::MessageArg};

/// Send a Microsoft Graph message from raw MIME (`POST /me/sendMail`);
/// Graph saves it to Sent Items.
#[derive(Debug, Parser)]
pub struct MsgraphMessageSendCommand {
    #[command(flatten)]
    pub message: MessageArg,
}

impl MsgraphMessageSendCommand {
    /// Sends the given MIME bytes.
    pub fn execute(self, printer: &mut impl Printer, client: &mut MsgraphClient) -> Result<()> {
        let raw = self.message.parse()?;
        client.mail_send_mime(&raw)?;
        printer.out(MsgraphMessageSendOutput { id: None })
    }
}

/// The `msgraph messages send` output.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MsgraphMessageSendOutput {
    /// The id of the sent message, always `null`: `sendMail` answers
    /// without one.
    id: Option<String>,
}

impl fmt::Display for MsgraphMessageSendOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Microsoft Graph message successfully sent")
    }
}
