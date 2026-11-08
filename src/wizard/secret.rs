//! # Secret choices
//!
//! Turns what pimalaya-cli's secret prompts answer into the [`Secret`] the
//! configuration holds.
//!
//! A known keyring or token broker yields an argv command, a custom one a
//! shell string, and raw storage the value itself.

use anyhow::{Result, bail};
use pimalaya_cli::wizard::keyring::{self, SecretChoice};
use pimalaya_config::{command::CommandConfig, secret::Secret};

/// Prompts for an OAuth 2.0 access token through the shared token picker,
/// which offers the token brokers beside the keyrings.
///
/// The default seeds the keyring entry or the broker account handle.
pub fn configure_token(label: &str, key_default: &str) -> Result<Secret> {
    to_secret(keyring::prompt_token(label, key_default, true)?)
}

/// Turns a picker choice into the [`Secret`] the configuration stores.
pub fn to_secret(choice: SecretChoice) -> Result<Secret> {
    Ok(match choice {
        SecretChoice::Command(argv) => command_secret(argv)?,
        SecretChoice::Shell(line) => shell_secret(&line)?,
        SecretChoice::Raw(secret) => Secret::Raw(secret),
    })
}

/// Builds a [`Secret::Command`] from an argv (program + arguments, no
/// shell), the form a known keyring provider or token broker yields. It
/// serializes back as a TOML array.
pub fn command_secret(argv: Vec<String>) -> Result<Secret> {
    let Some((program, args)) = argv.split_first() else {
        bail!("Empty command for secret");
    };

    Ok(Secret::Command(CommandConfig::Argv {
        program: program.clone(),
        args: args.to_vec(),
    }))
}

/// Builds a [`Secret::Command`] from a shell command line, the fallback
/// form a user typed by hand. It serializes back as a TOML string.
fn shell_secret(line: &str) -> Result<Secret> {
    let line = line.trim();
    if line.is_empty() {
        bail!("Empty shell command for secret");
    }

    Ok(Secret::Command(CommandConfig::Shell(line.to_owned())))
}

#[cfg(test)]
mod tests {
    use crate::wizard::secret::*;

    #[test]
    fn empty_command_secret_is_rejected() {
        assert!(command_secret(Vec::new()).is_err());
    }

    #[test]
    fn blank_shell_secret_is_rejected() {
        assert!(shell_secret("   ").is_err());
    }
}
