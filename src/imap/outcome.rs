//! # IMAP outcome
//!
//! What an `imap` command prints: the tagged completion of its exchange.
//! A completion the server accepts prints `status: "OK"` beside the data
//! the command understood; one it refuses prints `status: "NO"` or
//! `"BAD"` with the server's text, and the command exits 3, the server
//! having answered. A command that gets no completion at all (network,
//! TLS, a stream that does not parse) fails with the failure object.

use std::fmt;

use anyhow::{Error, Result};
use io_imap::{
    client::ImapClientError,
    rfc2971::id::ImapServerIdError,
    rfc3501::{
        append::ImapMessageAppendError, close::ImapMailboxCloseError, copy::ImapMessageCopyError,
        create::ImapMailboxCreateError, delete::ImapMailboxDeleteError,
        expunge::ImapMailboxExpungeError, fetch::ImapMessageFetchError, list::ImapMailboxListError,
        lsub::ImapMailboxLsubError, rename::ImapMailboxRenameError, search::ImapMessageSearchError,
        select::ImapMailboxSelectError, status::ImapMailboxStatusError,
        store::ImapMessageStoreError, subscribe::ImapMailboxSubscribeError,
        unsubscribe::ImapMailboxUnsubscribeError,
    },
    rfc3691::unselect::ImapMailboxUnselectError,
    rfc5256::{sort::ImapMessageSortError, thread::ImapMessageThreadError},
    rfc6851::r#move::ImapMessageMoveError,
};
use pimalaya_cli::{error::PartialOutcome, printer::Printer};
use schemars::JsonSchema;
use serde::Serialize;

/// The status of a tagged completion, in its protocol spelling.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum ImapStatus {
    /// The command completed.
    Ok,
    /// The server refused the command.
    No,
    /// The server rejected the command as malformed or out of place.
    Bad,
}

/// The completion an `imap` command prints.
///
/// The two alternatives are told apart by `text`, which only a refusal
/// carries.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum ImapOutcome<T> {
    /// The server accepted the command.
    Done {
        /// Always `OK`.
        status: ImapStatus,
        /// What the command understood of the server's answer.
        #[serde(flatten)]
        data: T,
    },
    /// The server refused the command.
    Refused {
        /// `NO` or `BAD`.
        status: ImapStatus,
        /// The text of the completion.
        text: String,
    },
}

impl<T: fmt::Display> fmt::Display for ImapOutcome<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Done { data, .. } => data.fmt(f),
            Self::Refused { status, text } => {
                let status = match status {
                    ImapStatus::Bad => "BAD",
                    _ => "NO",
                };
                writeln!(f, "IMAP server answered {status}: {text}")
            }
        }
    }
}

/// Prints the outcome of an IMAP exchange: the data when the server
/// accepted it, the refusal when it did not, the refusal ending the
/// command with the partial-outcome status. Any other error is returned
/// as is.
pub fn print<T>(printer: &mut impl Printer, result: Result<T>) -> Result<()>
where
    T: fmt::Display + Serialize,
{
    let err = match result {
        Ok(data) => {
            return printer.out(ImapOutcome::Done {
                status: ImapStatus::Ok,
                data,
            });
        }
        Err(err) => err,
    };

    let Some((status, text)) = refusal_of(&err) else {
        return Err(err);
    };

    printer.out(ImapOutcome::<T>::Refused { status, text })?;
    Err(PartialOutcome::default().into())
}

/// The status and text of the tagged `NO` or `BAD` an IMAP call failed
/// on, if that is how it failed.
fn refusal_of(err: &Error) -> Option<(ImapStatus, String)> {
    let err = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<ImapClientError>())?;

    macro_rules! refusal {
        ($($variant:ident($ty:ident)),* $(,)?) => {
            match err {
                $(
                    ImapClientError::$variant($ty::No(text)) => {
                        Some((ImapStatus::No, text.clone()))
                    }
                    ImapClientError::$variant($ty::Bad(text)) => {
                        Some((ImapStatus::Bad, text.clone()))
                    }
                )*
                _ => None,
            }
        };
    }

    refusal!(
        ServerId(ImapServerIdError),
        MailboxSelect(ImapMailboxSelectError),
        MailboxCreate(ImapMailboxCreateError),
        MailboxDelete(ImapMailboxDeleteError),
        MailboxRename(ImapMailboxRenameError),
        MailboxSubscribe(ImapMailboxSubscribeError),
        MailboxUnsubscribe(ImapMailboxUnsubscribeError),
        MailboxList(ImapMailboxListError),
        MailboxLsub(ImapMailboxLsubError),
        MailboxStatus(ImapMailboxStatusError),
        MailboxClose(ImapMailboxCloseError),
        MailboxUnselect(ImapMailboxUnselectError),
        MailboxExpunge(ImapMailboxExpungeError),
        MessageSearch(ImapMessageSearchError),
        MessageSort(ImapMessageSortError),
        MessageThread(ImapMessageThreadError),
        MessageStore(ImapMessageStoreError),
        MessageFetch(ImapMessageFetchError),
        MessageAppend(ImapMessageAppendError),
        MessageCopy(ImapMessageCopyError),
        MessageMove(ImapMessageMoveError),
    )
}

/// What an `imap` command acting on one mailbox understood: the mailbox.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ImapMailboxOutput {
    /// The mailbox acted on, as it is after the command.
    pub mailbox: String,
    /// What the text form says was done, as in `Mailbox created`.
    #[serde(skip)]
    pub done: &'static str,
}

impl fmt::Display for ImapMailboxOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}: {}", self.done, self.mailbox)
    }
}

/// What an `imap` command acting on the selected mailbox understood:
/// nothing beyond the completion.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ImapDoneOutput {
    /// What the text form says was done, as in `Mailbox closed`.
    #[serde(skip)]
    pub done: &'static str,
}

impl fmt::Display for ImapDoneOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.done)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Error;
    use io_imap::{
        client::ImapClientError,
        rfc3501::{lsub::ImapMailboxLsubError, select::ImapMailboxSelectError},
    };
    use serde_json::json;

    use crate::imap::outcome::{ImapDoneOutput, ImapOutcome, ImapStatus, refusal_of};

    #[test]
    fn a_tagged_no_reads_as_a_refusal() {
        let err = Error::from(ImapClientError::MailboxSelect(ImapMailboxSelectError::No(
            "Mailbox does not exist".into(),
        )))
        .context("Select mailbox");

        assert_eq!(
            refusal_of(&err),
            Some((ImapStatus::No, "Mailbox does not exist".into()))
        );
        assert_eq!(refusal_of(&anyhow::anyhow!("boom")), None);
    }

    #[test]
    fn a_refused_lsub_reads_as_a_refusal() {
        let err = Error::from(ImapClientError::MailboxLsub(ImapMailboxLsubError::Bad(
            "Parse Error".into(),
        )));

        assert_eq!(
            refusal_of(&err),
            Some((ImapStatus::Bad, "Parse Error".into()))
        );
    }

    #[test]
    fn a_completion_prints_its_status_beside_its_data() {
        let done = ImapOutcome::Done {
            status: ImapStatus::Ok,
            data: ImapDoneOutput {
                done: "Mailbox closed",
            },
        };
        let refused = ImapOutcome::<ImapDoneOutput>::Refused {
            status: ImapStatus::Bad,
            text: "Unknown command".into(),
        };

        assert_eq!(
            serde_json::to_value(done).unwrap(),
            json!({ "status": "OK" })
        );
        assert_eq!(
            serde_json::to_value(refused).unwrap(),
            json!({ "status": "BAD", "text": "Unknown command" })
        );
    }
}
