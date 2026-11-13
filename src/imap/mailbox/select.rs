//! # IMAP select
//!
//! The `imap select` command, RFC 3501 `SELECT`.

use std::{fmt, num::NonZeroU32};

use anyhow::Result;
use clap::Parser;
use io_imap::{
    client::ImapClient as _, rfc3501::select::ImapMailboxSelectOptions, types::flag::FlagPerm,
};
use pimalaya_cli::{fields::Fields, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::imap::{client::ImapClient, mailbox::arg::MailboxNameArg, outcome, utils::output_data};

/// Select the given mailbox (SELECT, RFC 3501).
///
/// Opens it for read-write access and returns its status. The selection
/// only outlives the command over a stateful IMAP session such as
/// `himalaya repl` or a Sirup proxy.
#[derive(Debug, Parser)]
pub struct ImapMailboxSelectCommand {
    #[command(flatten)]
    pub mailbox_name: MailboxNameArg,
}

impl ImapMailboxSelectCommand {
    /// Selects the mailbox.
    pub fn execute(self, printer: &mut impl Printer, client: &mut ImapClient) -> Result<()> {
        let name = self.mailbox_name.inner;
        let mailbox = name.clone().try_into()?;

        let result = client
            .select(mailbox, ImapMailboxSelectOptions::default())
            .map_err(Into::into)
            .map(|output| output_data("SELECT", output))
            .map(|data| ImapMailboxSelectOutput {
                mailbox: name,
                flags: data
                    .flags
                    .unwrap_or_default()
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                exists: data.exists,
                recent: data.recent,
                first_unseen: data.unseen.map(NonZeroU32::get),
                permanent_flags: data
                    .permanent_flags
                    .unwrap_or_default()
                    .iter()
                    .map(|flag| match flag {
                        FlagPerm::Flag(flag) => flag.to_string(),
                        FlagPerm::Asterisk => "\\*".to_string(),
                    })
                    .collect(),
                uid_next: data.uid_next.map(NonZeroU32::get),
                uid_validity: data.uid_validity.map(NonZeroU32::get),
                highest_mod_seq: data.highest_mod_seq,
            });

        outcome::print(printer, result)
    }
}

/// What `imap select` understood of the server's answer.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImapMailboxSelectOutput {
    /// The mailbox selected.
    mailbox: String,
    /// The flags defined in the mailbox, as spelled on the wire.
    flags: Vec<String>,
    /// How many messages the mailbox holds, `null` when not sent.
    exists: Option<u32>,
    /// How many messages are recent, `null` when not sent.
    recent: Option<u32>,
    /// The sequence number of the first unseen message (`[UNSEEN n]`, RFC
    /// 3501 section 7.1), not a count, `null` when not sent.
    first_unseen: Option<u32>,
    /// The flags a client can change permanently; `\\*` allows new
    /// keywords.
    permanent_flags: Vec<String>,
    /// The next UID, `null` when not sent.
    uid_next: Option<u32>,
    /// The UID validity, `null` when not sent.
    uid_validity: Option<u32>,
    /// The highest mod-sequence (CONDSTORE), `null` when not sent.
    highest_mod_seq: Option<u64>,
}

impl fmt::Display for ImapMailboxSelectOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = Fields::default();
        fields.push("Mailbox", sanitize::text(&self.mailbox));

        if let Some(exists) = self.exists {
            fields.push("Messages", exists);
        }
        if let Some(recent) = self.recent {
            fields.push("Recent", recent);
        }
        if let Some(first_unseen) = self.first_unseen {
            fields.push("First unseen", first_unseen);
        }
        if !self.flags.is_empty() {
            fields.push("Flags", sanitize::text(&self.flags.join(" ")));
        }
        if !self.permanent_flags.is_empty() {
            let permanent = self.permanent_flags.join(" ");
            fields.push("Permanent flags", sanitize::text(&permanent));
        }
        if let Some(uid_validity) = self.uid_validity {
            fields.push("UID validity", uid_validity);
        }
        if let Some(uid_next) = self.uid_next {
            fields.push("UID next", uid_next);
        }
        if let Some(highest_mod_seq) = self.highest_mod_seq {
            fields.push("Highest mod-seq", highest_mod_seq);
        }

        write!(f, "{fields}")
    }
}

#[cfg(test)]
mod tests {
    use crate::imap::mailbox::select::ImapMailboxSelectOutput;

    #[test]
    fn the_first_unseen_number_is_not_named_as_a_count() {
        let output = ImapMailboxSelectOutput {
            mailbox: String::from("INBOX"),
            flags: vec![String::from("\\Seen")],
            exists: Some(6),
            recent: Some(0),
            first_unseen: Some(4),
            permanent_flags: Vec::new(),
            uid_next: None,
            uid_validity: None,
            highest_mod_seq: None,
        };
        let json = serde_json::to_value(&output).unwrap();

        assert_eq!(json["firstUnseen"], 4);
        assert!(json.get("unseen").is_none());
        assert!(output.to_string().contains("First unseen"));
    }
}
