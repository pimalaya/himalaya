//! # Mailbox configuration
//!
//! The `mailbox` block, global or per account.

use std::collections::HashMap;

use serde::Deserialize;

/// Mailbox aliases.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MailboxConfig {
    /// Friendly names mapped to backend-native mailbox ids, resolved
    /// case-insensitively.
    ///
    /// An alias named after a mailbox role overrides the role the backend
    /// reports, the `inbox` one being the default mailbox of a unified
    /// command omitting `-m/--mailbox`.
    #[serde(default, rename = "alias", alias = "aliases")]
    pub aliases: HashMap<String, String>,
}
