//! # Message delete
//!
//! The `message delete` command, trashing messages or, once they are in
//! the trash, removing them.

use std::fmt;

use anyhow::{Result, anyhow, bail};
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    email::mailbox::MailboxRole,
    shared::{client::EmailClient, flag::arg::MessageIdsArg, mailbox::arg::MailboxArg},
};

/// Delete messages, trash first.
///
/// The messages are moved to the trash, or removed for good when they are
/// already there. The trash comes from `mailbox.alias.trash` when set, and
/// from the backend's trash role otherwise.
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
        let mailbox = self.mailbox.resolve(account);
        let ids: Vec<&str> = self.message_ids.inner.iter().map(String::as_str).collect();

        let trash = match account.mailbox_alias.get("trash") {
            Some(trash) => trash.clone(),
            None => client.role_mailbox_id(&MailboxRole::Trash)?.ok_or_else(|| {
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
                MessageDeleteOutput::new(DeleteAction::Deleted, count)
            } else {
                MessageDeleteOutput::new(DeleteAction::Flagged, count)
            }
        } else {
            let moved = client.move_messages(&current_id, &trash_id, &ids)?;
            if moved == 0 {
                bail!("No message deleted: no id matched in the mailbox");
            }
            MessageDeleteOutput::new(DeleteAction::MovedToTrash, moved)
        };

        printer.out(report)
    }
}

/// The `message delete` output: what was done, and to how many messages.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MessageDeleteOutput {
    action: DeleteAction,
    count: usize,
}

impl MessageDeleteOutput {
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

impl fmt::Display for MessageDeleteOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let count = self.count;
        match self.action {
            DeleteAction::MovedToTrash => {
                writeln!(f, "Successfully moved {count} message(s) to the trash")
            }
            DeleteAction::Deleted => {
                writeln!(f, "Successfully deleted {count} message(s) from the trash")
            }
            DeleteAction::Flagged => writeln!(
                f,
                "Flagged {count} message(s) as deleted; run an expunge on the trash to remove them",
            ),
        }
    }
}
