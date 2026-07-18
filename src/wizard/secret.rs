//! Secret prompt shared by the discovered-backend wizards (JMAP, Gmail,
//! Microsoft Graph).
//!
//! Delegates to pimalaya-cli's OS-aware keyring picker: a well-known
//! credential CLI, a custom command, or a raw value. Token backends pass
//! the Ortie broker as an extra first option. Himalaya only *reads* the
//! secret, so a keyring choice prints a reminder (on stderr) to store it
//! under the chosen entry first.

use anyhow::Result;
use pimalaya_cli::wizard::keyring::{self, SecretChoice};
use pimalaya_config::{command::shell, secret::Secret};

/// Prompts for a [`Secret`] through the shared keyring picker.
///
/// `key_default` seeds the keyring entry (typically
/// `<account>-<protocol>`); the entry is used verbatim, so a
/// pre-existing secret is read exactly as named. `extra` prepends
/// product-specific options such as [`ortie`].
pub fn configure(label: &str, key_default: &str, extra: &[(&str, String)]) -> Result<Secret> {
    let choice = keyring::prompt_secret(label, key_default, extra)?;

    Ok(match choice {
        SecretChoice::Command(command) => Secret::Command(shell(&command)),
        SecretChoice::Raw(secret) => Secret::Raw(secret),
    })
}

/// The Ortie OAuth-broker extra option for token backends: reads (and
/// transparently refreshes) the access token for `account`.
pub fn ortie(account: &str) -> [(&'static str, String); 1] {
    [(
        "ortie (OAuth token broker)",
        format!("ortie token show -a {account}"),
    )]
}
