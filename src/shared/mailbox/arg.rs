use anyhow::{Result, anyhow};
use clap::Parser;

use crate::account::context::Account;

/// Optional `-m|--mailbox <NAME>` flag shared by every cross-protocol
/// command that targets a single mailbox. The argument is resolved
/// through [`Self::resolve`] so callers can transparently consult the
/// account's `[mailbox.alias]` map and fall back to the implicit
/// default mailbox bound to the `inbox` alias.
#[derive(Clone, Debug, Default, Parser)]
pub struct MailboxArg {
    /// Mailbox name. Looked up against `[mailbox.alias]`
    /// case-insensitively; raw backend-native ids are accepted too
    /// and returned verbatim when no alias matches. Omit to fall
    /// back to the id mapped to the `inbox` alias.
    #[arg(short = 'm', long = "mailbox", value_name = "NAME")]
    pub inner: Option<String>,
}

impl MailboxArg {
    /// Resolves the mailbox name to a backend-native id, returning
    /// an owned `String` (the borrowed view from
    /// [`Account::resolve_mailbox`] does not survive past the temporary
    /// lookup key).
    ///
    /// Errors only when `-m/--mailbox` is omitted and the account has
    /// no `inbox` alias configured.
    pub fn resolve(&self, account: &Account) -> Result<String> {
        match self.inner.as_deref() {
            Some(name) => Ok(account.resolve_mailbox(name).to_string()),
            None => account.default_mailbox().map(str::to_owned).ok_or_else(|| {
                anyhow!(
                    "Mailbox is required: pass -m/--mailbox <NAME>, \
                         or set `mailbox.alias.inbox = \"<id>\"` in your configuration."
                )
            }),
        }
    }
}
