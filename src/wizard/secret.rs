//! # Secret prompts
//!
//! The credential prompts the backend wizards share, delegating to
//! pimalaya-cli's OS-aware pickers.
//!
//! One offers the OS keyrings and the other the OAuth 2.0 token brokers,
//! both also taking a custom command or a raw value. A known provider or
//! broker yields an argv command, a custom one a shell string.
//!
//! Himalaya only reads a secret, never stores one, so a value that is not
//! there yet surfaces when the account is tested right after.

use std::process::Command;

use anyhow::{Result, bail};
use pimalaya_cli::wizard::keyring::{self, SecretChoice};
use pimalaya_config::{command::shell, secret::Secret};

/// Prompts for a password through the shared keyring picker.
///
/// The default seeds the keyring entry, which is then used verbatim, so a
/// secret already there is read exactly as it is named.
pub fn configure_password(label: &str, key_default: &str) -> Result<Secret> {
    to_secret(keyring::prompt_secret(label, key_default)?)
}

/// Prompts for an API token through the shared token picker.
///
/// The picker offers the OS keyrings, for a token generated on the
/// provider, and the OAuth 2.0 brokers, which refresh and print a fresh
/// one on every read.
///
/// The default seeds the keyring entry or the broker account handle.
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
