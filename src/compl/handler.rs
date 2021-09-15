//! Module related to completion handling.
//!
//! This module gathers all completion actions triggered by the CLI.

use anyhow::{anyhow, Context, Result};
use clap::{App, Shell};
use std::{io, str::FromStr};

/// Generate completion script from the given [`clap::App`] for the given shell slice.
pub fn generate<'a>(shell: &'a str, mut app: App<'a, 'a>) -> Result<()> {
    let shell = Shell::from_str(shell)
        .map_err(|err| anyhow!(err))
        .context("cannot parse shell")?;
    app.gen_completions_to("himalaya", shell, &mut io::stdout());
    Ok(())
}
