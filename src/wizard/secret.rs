//! Secret prompt shared by the discovered-backend wizards.
//!
//! Every remote backend needs at least one secret (password, API
//! token). The prompt offers the same two strategies everywhere: a
//! shell command retrieving the secret at runtime (recommended; OAuth
//! tokens typically shell out to Ortie), or the raw value stored in the
//! configuration file.

use anyhow::Result;
use pimalaya_cli::prompt;
use pimalaya_config::{command::shell, secret::Secret};

const CMD: &str = "Use a shell command to retrieve my secret (recommended)";
const RAW: &str = "Save secret in the configuration file (plaintext, NOT recommended)";
const SECRETS: [&str; 2] = [CMD, RAW];

/// Prompts for a [`Secret`]: strategy picker, then either the shell
/// command line (seeded with `default_cmd`) or the raw value.
pub fn configure(label: &str, default_cmd: Option<&str>) -> Result<Secret> {
    let strategy = prompt::item(format!("{label} strategy:"), SECRETS, None)?;

    match strategy {
        CMD => {
            let cmd = prompt::text("Shell command:", default_cmd)?;
            Ok(Secret::Command(shell(&cmd)))
        }
        RAW => {
            let secret = prompt::password(format!("{label}:"), format!("Confirm {label}:"))?;
            Ok(Secret::Raw(secret))
        }
        _ => unreachable!(),
    }
}
