//! # Message delete
//!
//! The `message delete` command, trashing messages or, once they are in
//! the trash, removing them.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    error::{ItemError, partial_when, write_item_errors},
    unified::{
        capability::{self, Capability},
        client::EmailClient,
        email::mailbox::MailboxRole,
        flag::arg::MessageIdsArg,
        mailbox::arg::MailboxArg,
    },
};

/// Delete messages, trash first.
///
/// The messages are moved to the trash, or removed for good when they are
/// already there. The trash comes from `mailbox.alias.trash` when set, and
/// from the backend's trash role otherwise.
///
/// On an IMAP server without UIDPLUS, removing from the trash only flags
/// the messages `\Deleted`, and a later expunge reclaims them.
///
/// An id the mailbox does not hold is an entry of `errors`, the other
/// messages still acted on; the command then exits 3.
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
        let ids = self.message_ids.ids();

        let trash = match account.mailbox_alias.get("trash") {
            Some(trash) => trash.clone(),
            None => match client.role_mailbox_id(&MailboxRole::Trash)? {
                Some(trash) => trash,
                None => return Err(no_trash(client)),
            },
        };

        let current_id = client.resolve_mailbox_id(&mailbox)?;
        let trash_id = client.resolve_mailbox_id(&trash)?;

        let queued = client.queues_writes();
        let output = if current_id == trash_id {
            let outcome = client.delete_messages(&current_id, &ids)?;
            let (count, removed) = outcome.value;
            let action = if removed {
                DeleteAction::Deleted
            } else {
                DeleteAction::Flagged
            };
            MessageDeleteOutput::new(action, count, &ids, queued, outcome.errors)
        } else {
            let outcome = client.move_messages(&current_id, &trash_id, &ids)?;
            let action = DeleteAction::MovedToTrash;
            MessageDeleteOutput::new(action, outcome.value, &ids, queued, outcome.errors)
        };

        printer.out(&output)?;
        partial_when(&output.errors)
    }
}

/// The refusal of a delete on an account with no trash: neither
/// `mailbox.alias.trash` nor a mailbox the backend marks as the trash.
fn no_trash(client: &EmailClient) -> anyhow::Error {
    let resolution = client.resolution();
    let account = &resolution.account;
    let backend = match resolution.transport(Capability::MessageTrash) {
        Some(backend) => format!(" over {backend}"),
        None => String::new(),
    };
    let capability = Capability::MessageTrash;
    capability::missing(format!(
        "Account `{account}` cannot `{capability}`{backend}: no mailbox is marked as the trash; set `mailbox.alias.trash`"
    ))
}

/// The `message delete` output: what was done, and to how many messages.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MessageDeleteOutput {
    /// What became of the messages.
    action: DeleteAction,
    /// How many messages were acted on.
    count: usize,
    /// The ids of the messages acted on, as given and in that order, those
    /// of `errors` left out.
    ids: Vec<String>,
    /// Whether a sync engine applies the write later rather than now.
    queued: bool,
    /// The error of each message not acted on, in the order given.
    errors: Vec<ItemError>,
}

impl MessageDeleteOutput {
    /// The output of `action` on `count` of `given`, the ids `errors`
    /// names left out of `ids`.
    fn new(
        action: DeleteAction,
        count: usize,
        given: &[&str],
        queued: bool,
        errors: Vec<ItemError>,
    ) -> Self {
        let failed = |id: &&str| errors.iter().any(|err| err.id.as_deref() == Some(*id));
        let ids = given
            .iter()
            .filter(|id| !failed(id))
            .map(ToString::to_string);

        Self {
            action,
            count,
            ids: ids.collect(),
            queued,
            errors,
        }
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
        match (&self.action, self.queued) {
            (DeleteAction::MovedToTrash, true) => {
                writeln!(f, "Queued the move of {count} message(s) to the trash")
            }
            (DeleteAction::Deleted, true) => {
                writeln!(
                    f,
                    "Queued the deletion of {count} message(s) from the trash"
                )
            }
            (DeleteAction::MovedToTrash, false) => {
                writeln!(f, "Successfully moved {count} message(s) to the trash")
            }
            (DeleteAction::Deleted, false) => {
                writeln!(f, "Successfully deleted {count} message(s) from the trash")
            }
            (DeleteAction::Flagged, _) if count == 0 => {
                writeln!(f, "Flagged no message as deleted")
            }
            (DeleteAction::Flagged, _) => writeln!(
                f,
                "Flagged {count} message(s) as deleted; run an expunge on the trash to remove them",
            ),
        }?;

        write_item_errors(f, &self.errors)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        error::missing_message,
        unified::message::delete::{DeleteAction, MessageDeleteOutput},
    };

    fn moved(queued: bool) -> MessageDeleteOutput {
        MessageDeleteOutput {
            action: DeleteAction::MovedToTrash,
            count: 2,
            ids: vec!["1".into(), "2".into()],
            queued,
            errors: Vec::new(),
        }
    }

    #[test]
    fn a_queued_delete_says_it_is_queued() {
        let queued = moved(true);
        let done = moved(false);

        assert_eq!(
            queued.to_string(),
            "Queued the move of 2 message(s) to the trash\n"
        );
        assert_eq!(
            done.to_string(),
            "Successfully moved 2 message(s) to the trash\n"
        );
        assert_eq!(
            serde_json::to_string(&queued).unwrap(),
            r#"{"action":"moved-to-trash","count":2,"ids":["1","2"],"queued":true,"errors":[]}"#
        );
    }

    /// Each message not acted on prints one `Error:` line after the
    /// sentence and a blank line, and one item error under `errors`.
    #[test]
    fn an_unknown_id_prints_after_the_sentence() {
        let mut output = moved(false);
        output.count = 0;
        output.errors = vec![missing_message("99")];

        assert_eq!(
            output.to_string(),
            "Successfully moved 0 message(s) to the trash\n\n\
             Error:  99  message-not-found  Message not found in the mailbox\n"
        );
        assert_eq!(
            serde_json::to_value(&output).unwrap()["errors"],
            serde_json::json!([{
                "id": "99",
                "code": "message-not-found",
                "error": "Message not found in the mailbox",
                "sources": [],
            }])
        );
    }

    /// An expunge removes every `\Deleted` message of the trash, so a
    /// delete that flagged nothing never advises one (#814).
    #[test]
    fn a_flag_delete_advises_an_expunge_only_when_it_flagged() {
        let mut output = moved(false);
        output.action = DeleteAction::Flagged;

        assert!(output.to_string().contains("run an expunge"), "{output}");

        output.count = 0;
        output.errors = vec![missing_message("99")];
        assert_eq!(
            output.to_string(),
            "Flagged no message as deleted\n\n\
             Error:  99  message-not-found  Message not found in the mailbox\n"
        );
    }
}
