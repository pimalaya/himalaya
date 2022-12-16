//! Module related to man handling.
//!
//! This module gathers all man commands.  

use anyhow::Result;
use clap::Command;
use clap_mangen::Man;
use std::io::stdout;

/// Generates completion script from the given [`clap::App`] for the given shell slice.
pub fn generate(cmd: Command) -> Result<()> {
    Man::new(cmd).render(&mut stdout())?;
    Ok(())
}
