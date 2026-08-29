//! # Mailbox argument
//!
//! The `-m/--mailbox` flag every shared command targeting one mailbox
//! takes, and the alias resolution behind it.

use anyhow::{Result, anyhow};
use clap::Parser;

use crate::account::context::Account;

/// The `-m/--mailbox` flag of a command targeting one mailbox.
#[derive(Clone, Debug, Default, Parser)]
pub struct MailboxArg {
    /// Mailbox name, alias or backend-native id.
    ///
    /// The value is looked up against `mailbox.alias` case-insensitively
    /// and passed through verbatim when nothing matches. Omitted, the
    /// mailbox the `inbox` alias names is used.
    #[arg(short = 'm', long = "mailbox", value_name = "NAME")]
    pub inner: Option<String>,
}

impl MailboxArg {
    /// Resolves the flag to a backend-native id, erroring only when it
    /// is omitted and no `inbox` alias is configured.
    pub fn resolve(&self, account: &Account) -> Result<String> {
        resolve_mailbox_or_default(account, self.inner.as_deref())
    }
}

/// Resolves an optional mailbox name to a backend-native id, falling back
/// to the `inbox` alias and erroring when there is none.
///
/// A shared command cannot guess a backend's inbox id, JMAP's being an
/// opaque server-assigned string, so an omitted mailbox wants the alias
/// rather than a literal name.
pub fn resolve_mailbox_or_default(account: &Account, name: Option<&str>) -> Result<String> {
    match name {
        Some(name) => Ok(account.resolve_mailbox(name).to_string()),
        None => account.default_mailbox().map(str::to_owned).ok_or_else(|| {
            anyhow!(
                "Mailbox is required: pass the mailbox name or alias, \
                 or set `mailbox.alias.inbox = \"<id>\"` in your configuration."
            )
        }),
    }
}
