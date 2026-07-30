//! Secret prompts shared by the discovered-backend wizards (JMAP, Gmail,
//! Microsoft Graph).
//!
//! Delegates to pimalaya-cli's OS-aware pickers: [`configure_password`]
//! offers the OS keyrings, [`configure_token`] the OAuth 2.0 token
//! brokers (Ortie, pizauth, oama). Both also allow a custom command or a
//! raw value. A known provider or broker yields an argv command (a TOML
//! array); a custom command is a shell string. Himalaya only *reads* the
//! secret: the value must already be stored, and a missing one surfaces
//! when the account is tested right after.

use std::process::Command;

use anyhow::{Result, bail};
use pimalaya_cli::wizard::keyring::{self, SecretChoice};
use pimalaya_config::{command::shell, secret::Secret};

/// Prompts for a password [`Secret`] through the shared keyring picker.
///
/// `key_default` seeds the keyring entry (typically
/// `<account>-<protocol>`); the entry is used verbatim, so a pre-existing
/// secret is read exactly as named.
pub fn configure_password(label: &str, key_default: &str) -> Result<Secret> {
    to_secret(keyring::prompt_secret(label, key_default)?)
}

/// Prompts for an API token [`Secret`] through the shared token picker,
/// which combines the OS keyrings (for a token generated on the provider)
/// with the OAuth 2.0 brokers when `oauth` is true (a broker refreshes and
/// prints a fresh token on every read).
///
/// `key_default` seeds the keyring entry or the broker account handle
/// (typically the Himalaya account name).
pub fn configure_token(label: &str, key_default: &str, oauth: bool) -> Result<Secret> {
    to_secret(keyring::prompt_token(label, key_default, oauth)?)
}

fn to_secret(choice: SecretChoice) -> Result<Secret> {
    Ok(match choice {
        SecretChoice::Command(argv) => command_secret(argv)?,
        SecretChoice::Shell(line) => shell_secret(&line)?,
        SecretChoice::Raw(secret) => Secret::Raw(secret),
    })
}

/// Builds a [`Secret::Command`] from an argv (program + arguments, no
/// shell), the form a known keyring provider or token broker yields. It
/// serializes back as a TOML array.
fn command_secret(argv: Vec<String>) -> Result<Secret> {
    let Some((program, args)) = argv.split_first() else {
        bail!("Empty command for secret");
    };

    let mut cmd = Command::new(program);
    cmd.args(args);
    Ok(Secret::Command(cmd))
}

/// Builds a [`Secret::Command`] from a shell command line, the fallback
/// form a user typed by hand. It serializes back as a TOML string.
fn shell_secret(line: &str) -> Result<Secret> {
    let line = line.trim();
    if line.is_empty() {
        bail!("Empty shell command for secret");
    }

    Ok(Secret::Command(shell(line)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_command_secret_is_rejected() {
        assert!(command_secret(Vec::new()).is_err());
    }

    #[test]
    fn blank_shell_secret_is_rejected() {
        assert!(shell_secret("   ").is_err());
    }
}
