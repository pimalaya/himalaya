use anyhow::Result;
use std::process::Command;

/// TODO: move this in a more approriate place.
pub fn run_cmd(cmd: &str) -> Result<String> {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(&["/C", cmd]).output()
    } else {
        Command::new("sh").arg("-c").arg(cmd).output()
    }?;

    Ok(String::from_utf8(output.stdout)?)
}
