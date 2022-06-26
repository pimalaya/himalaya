//! Process module.
//!
//! This module contains cross platform helpers around the
//! `std::process` crate.

use log::{debug, trace};
use std::{io, process::Command, string};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("cannot run command {1:?}")]
    RunCmdError(#[source] io::Error, String),

    #[error("cannot parse command output")]
    ParseCmdOutputError(#[source] string::FromUtf8Error),
}

pub fn run(cmd: &str) -> Result<String, ProcessError> {
    debug!(">> run command");
    debug!("command: {}", cmd);

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(&["/C", cmd]).output()
    } else {
        Command::new("sh").arg("-c").arg(cmd).output()
    };
    let output = output.map_err(|err| ProcessError::RunCmdError(err, cmd.to_string()))?;
    let output = String::from_utf8(output.stdout).map_err(ProcessError::ParseCmdOutputError)?;

    trace!("command output: {}", output);
    debug!("<< run command");
    Ok(output)
}
