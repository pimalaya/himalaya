use anyhow::{anyhow, Context, Result};
use log::debug;
use std::{
    io::prelude::*,
    process::{Command, Stdio},
};

/// TODO: move this in a more approriate place.
pub fn run_cmd(cmd: &str) -> Result<String> {
    debug!("running command: {}", cmd);

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(&["/C", cmd]).output()
    } else {
        Command::new("sh").arg("-c").arg(cmd).output()
    }?;

    Ok(String::from_utf8(output.stdout)?)
}

pub fn pipe_cmd(cmd: &str, data: &[u8]) -> Result<Vec<u8>> {
    let mut res = Vec::new();

    let process = Command::new(cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .with_context(|| format!("cannot spawn process from command {:?}", cmd))?;
    process
        .stdin
        .ok_or_else(|| anyhow!("cannot get stdin"))?
        .write_all(data)
        .with_context(|| "cannot write data to stdin")?;
    process
        .stdout
        .ok_or_else(|| anyhow!("cannot get stdout"))?
        .read_to_end(&mut res)
        .with_context(|| "cannot read data from stdout")?;

    Ok(res)
}
