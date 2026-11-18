//! # JMAP mailbox destroy
//!
//! The `jmap mailbox destroy` command, the destroy half of RFC 8621
//! `Mailbox/set`.

use anyhow::Result;
use clap::Parser;
use io_jmap::rfc8621::mailbox::set::JmapMailboxSetArgs;
use pimalaya_cli::printer::Printer;

use crate::{
    error::ErrorCode,
    jmap::{client::JmapClient, set::JmapSetReceipt},
};

/// Delete a JMAP mailbox.
#[derive(Debug, Parser)]
pub struct JmapMailboxDestroyCommand {
    /// The ID of the mailbox to delete.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,
    /// Destroy all emails in the mailbox when deleting.
    #[arg(long, default_value_t)]
    pub purge: bool,
}

impl JmapMailboxDestroyCommand {
    /// Destroys the mailbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut JmapClient) -> Result<()> {
        let args = JmapMailboxSetArgs {
            destroy: Some(self.ids.clone()),
            on_destroy_remove_emails: if self.purge { Some(true) } else { None },
            ..Default::default()
        };

        let output = client.mailbox_set(args)?;

        JmapSetReceipt::new(
            "JMAP mailboxes successfully deleted",
            output.destroyed,
            output.not_destroyed,
            Some(ErrorCode::MailboxNotFound),
        )
        .print(printer)
    }
}
