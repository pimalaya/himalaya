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
        resolve_mailbox_or_default(account, self.inner.as_deref())
    }
}

/// Resolves a shared command's optional mailbox argument to a
/// backend-native id: a provided `name` goes through the account's
/// `[mailbox.alias]` map ([`Account::resolve_mailbox`]); an omitted one
/// falls back to the `inbox` alias ([`Account::default_mailbox`]),
/// erroring when none is configured.
///
/// The shared commands cannot guess a backend's inbox id (JMAP's is an
/// opaque server-assigned string), so an omitted mailbox requires the
/// `inbox` alias rather than silently defaulting to a literal name.
/// Shared by `-m/--mailbox` and the `--from` source of `message
/// copy`/`move`.
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
