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

/// Delete message(s) from the active account.
///
/// Follows a trash-first policy: the messages are moved to the trash
/// mailbox, unless they already are in the trash, in which case they are
/// permanently removed. The trash mailbox is resolved from the backend
/// when it can be (otherwise from the `mailbox.alias.trash` config
/// entry, and failing that the command errors). Note that on IMAP
/// servers without UIDPLUS the in-trash removal only flags the messages
/// `\Deleted`; a later `expunge` reclaims them.
#[derive(Debug, Parser)]
pub struct MessageDeleteCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    #[command(flatten)]
    pub message_ids: MessageIdsArg,
}

impl MessageDeleteCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut EmailClient,
    ) -> Result<()> {
        let mailbox = self.mailbox.resolve(account)?;
        let ids: Vec<&str> = self.message_ids.inner.iter().map(String::as_str).collect();

        // Resolve the trash mailbox: the backend's own trash when it can
        // resolve one, else the `mailbox.alias.trash` config entry, else
        // an error.
        let trash = match client.native_trash()? {
            Some(trash) => trash,
            None => account.mailbox_alias.get("trash").cloned().ok_or_else(|| {
                anyhow!(
                    "Cannot determine the trash mailbox; set `mailbox.alias.trash` in your config"
                )
            })?,
        };

        // `resolve_mailbox_id` is idempotent, so this compares the current
        // mailbox against the trash regardless of how each was addressed.
        let current_id = client.resolve_mailbox_id(&mailbox)?;
        let trash_id = client.resolve_mailbox_id(&trash)?;

        let report = if current_id == trash_id {
            // Already in the trash: permanently delete these ids.
            let count = ids.len();
            if client.delete_messages(&current_id, &ids)? {
                DeleteReport::new(DeleteAction::Deleted, count)
            } else {
                DeleteReport::new(DeleteAction::Flagged, count)
            }
        } else {
            // Elsewhere: move them to the trash.
            let moved = client.move_messages(&current_id, &trash_id, &ids)?;
            DeleteReport::new(DeleteAction::MovedToTrash, moved)
        };

        printer.out(report)
    }
}

/// Structured result of `messages delete`: which action was taken and how
/// many messages it affected.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct DeleteReport {
    action: DeleteAction,
    count: usize,
}

impl DeleteReport {
    fn new(action: DeleteAction, count: usize) -> Self {
        Self { action, count }
    }
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum DeleteAction {
    MovedToTrash,
    Deleted,
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
