//! # Mailbox argument
//!
//! The `-m/--mailbox` flag every shared command targeting one mailbox
//! takes, and the alias resolution behind it.

use clap::Parser;

use crate::{account::context::Account, email::mailbox::MailboxRole};

/// The `-m/--mailbox` flag of a command targeting one mailbox.
#[derive(Clone, Debug, Default, Parser)]
pub struct MailboxArg {
    /// Mailbox name, alias, role or backend-native id.
    ///
    /// The value is looked up against `mailbox.alias` case-insensitively,
    /// then against the mailbox roles the backend knows (inbox, sent,
    /// drafts, trash...), and passed through verbatim when nothing
    /// matches. Omitted, the inbox is used.
    #[arg(short = 'm', long = "mailbox", value_name = "NAME")]
    pub inner: Option<String>,
}

impl MailboxArg {
    /// Resolves the flag through the aliases, the client resolving roles
    /// and names afterwards.
    pub fn resolve(&self, account: &Account) -> String {
        resolve_mailbox_or_default(account, self.inner.as_deref())
    }
}

/// Resolves an optional mailbox name through the aliases, falling back to
/// the inbox.
///
/// An omitted mailbox is the `inbox` alias when set, and the `inbox` role
/// otherwise, which the client maps onto the backend's own inbox.
pub fn resolve_mailbox_or_default(account: &Account, name: Option<&str>) -> String {
    let name = name.unwrap_or(MailboxRole::Inbox.as_str());
    account.resolve_mailbox(name).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, MailboxConfig};

    #[test]
    fn omitted_mailbox_is_the_inbox_alias_else_the_inbox_role() {
        let bare = Account::from(Config::default());
        assert_eq!(resolve_mailbox_or_default(&bare, None), "inbox");

        let aliased = Account::from(Config {
            mailbox: MailboxConfig {
                aliases: [("inbox".to_string(), "raw-id".to_string())].into(),
                ..MailboxConfig::default()
            },
            ..Config::default()
        });
        assert_eq!(resolve_mailbox_or_default(&aliased, None), "raw-id");
        assert_eq!(resolve_mailbox_or_default(&aliased, Some("Sent")), "Sent");
    }
}
