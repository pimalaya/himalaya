use log::{debug, trace};
use std::{io, process, result, string};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot run command: {1}")]
    RunCmdError(#[source] io::Error, String),
    #[error("cannot parse command output")]
    ParseCmdOutputError(#[source] string::FromUtf8Error),
}

pub type Result<T> = result::Result<T, Error>;

pub fn run_cmd(cmd: &str) -> Result<String> {
    trace!(">> run command");
    debug!("command: {}", cmd);

    let output = if cfg!(target_os = "windows") {
        process::Command::new("cmd").args(&["/C", cmd]).output()
    } else {
        process::Command::new("sh").arg("-c").arg(cmd).output()
    };
    let output = output.map_err(|err| Error::RunCmdError(err, cmd.to_string()))?;
    let output = String::from_utf8(output.stdout).map_err(Error::ParseCmdOutputError)?;

    debug!("command output: {}", output);
    trace!("<< run command");
    Ok(output)
}
