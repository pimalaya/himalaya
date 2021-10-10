//! Module related to completion handling.
//!
//! This module gathers all completion commands.  

use anyhow::{anyhow, Context, Result};
use clap::{App, Shell};
use std::{io, str::FromStr};

/// Generate completion script from the given [`clap::App`] for the given shell slice.
pub fn generate<'a>(mut app: App<'a, 'a>, shell: Option<&'a str>) -> Result<()> {
    let shell = Shell::from_str(shell.unwrap_or_default())
        .map_err(|err| anyhow!(err))
        .context("cannot parse shell")?;
    app.gen_completions_to("himalaya", shell, &mut io::stdout());
    Ok(())
}
