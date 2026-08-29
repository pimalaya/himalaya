//! # Message delete
//!
//! The `message delete` command, trashing messages or, once they are in
//! the trash, removing them.

use std::fmt;

use anyhow::{Result, anyhow};
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    shared::{client::EmailClient, flag::arg::MessageIdsArg, mailbox::arg::MailboxArg},
};

/// Delete messages, trash first.
///
/// The messages are moved to the trash, or removed for good when they are
/// already there. The trash comes from the backend when it names one, and
/// from `mailbox.alias.trash` otherwise.
///
/// On an IMAP server without UIDPLUS, removing from the trash only flags
/// the messages `\Deleted`, and a later expunge reclaims them.
#[derive(Debug, Parser)]
pub struct MessageDeleteCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    #[command(flatten)]
    pub message_ids: MessageIdsArg,
}

impl MessageDeleteCommand {
    /// Trashes or removes the messages and reports which happened.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let mailbox = self.mailbox.resolve(account)?;
        let ids: Vec<&str> = self.message_ids.inner.iter().map(String::as_str).collect();

        let trash = match client.native_trash()? {
            Some(trash) => trash,
            None => account.mailbox_alias.get("trash").cloned().ok_or_else(|| {
                anyhow!(
                    "Cannot determine the trash mailbox; set `mailbox.alias.trash` in your config"
                )
            })?,
        };

        // NOTE: `resolve_mailbox_id` is idempotent, so the comparison holds
        // however each of the two mailboxes was addressed.
        let current_id = client.resolve_mailbox_id(&mailbox)?;
        let trash_id = client.resolve_mailbox_id(&trash)?;

        let report = if current_id == trash_id {
            let count = ids.len();
            if client.delete_messages(&current_id, &ids)? {
                DeleteReport::new(DeleteAction::Deleted, count)
            } else {
                DeleteReport::new(DeleteAction::Flagged, count)
            }
        } else {
            let moved = client.move_messages(&current_id, &trash_id, &ids)?;
            DeleteReport::new(DeleteAction::MovedToTrash, moved)
        };

        printer.out(report)
    }
}

/// The `message delete` output: what was done, and to how many messages.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct DeleteReport {
    action: DeleteAction,
    count: usize,
}

impl DeleteReport {
    /// Reports one action over a message count.
    fn new(action: DeleteAction, count: usize) -> Self {
        Self { action, count }
    }
}

/// What `message delete` did to the messages it was given.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum DeleteAction {
    /// They were moved to the trash.
    MovedToTrash,
    /// They were in the trash and were removed for good.
    Deleted,
    /// They were in the trash and were flagged `\Deleted`, an expunge
    /// still owing.
    Flagged,
}

impl fmt::Display for DeleteReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.count;
        match self.action {
            DeleteAction::MovedToTrash => {
                write!(f, "Successfully moved {count} message(s) to the trash")
            }
            DeleteAction::Deleted => {
                write!(f, "Successfully deleted {count} message(s) from the trash")
            }
            DeleteAction::Flagged => write!(
                f,
                "Flagged {count} message(s) as deleted; run an expunge on the trash to remove them",
            ),
        }
    }
}
