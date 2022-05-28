use log::debug;
use std::{io, process::Command, result, string};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("cannot run command")]
    RunCmdError(#[from] io::Error),
    #[error("cannot parse command output")]
    ParseCmdOutputError(#[from] string::FromUtf8Error),
}

type Result<T> = result::Result<T, ProcessError>;

pub fn run_cmd(cmd: &str) -> Result<String> {
    debug!("running command: {}", cmd);

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(&["/C", cmd]).output()
    } else {
        Command::new("sh").arg("-c").arg(cmd).output()
    }?;

    Ok(String::from_utf8(output.stdout)?)
}
