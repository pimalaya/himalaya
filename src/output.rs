use error_chain::error_chain;
use serde::Serialize;
use std::{fmt::Display, process::Command};

error_chain! {}

pub fn run_cmd(cmd: &str) -> Result<String> {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(&["/C", cmd]).output()
    } else {
        Command::new("sh").arg("-c").arg(cmd).output()
    }
    .chain_err(|| "Run command failed")?;

    Ok(String::from_utf8(output.stdout).chain_err(|| "Invalid utf8 output")?)
}

pub fn print<T: Display + Serialize>(output_type: &str, item: T) -> Result<()> {
    match output_type {
        "json" => print!(
            "{}",
            serde_json::to_string(&item).chain_err(|| "Invalid JSON string")?
        ),
        "text" | _ => println!("{}", item.to_string()),
    }

    Ok(())
}
