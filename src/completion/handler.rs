use anyhow::Result;
use clap::Command;
use clap_complete::Shell;
use std::io::stdout;

use crate::printer::Printer;

pub fn generate(printer: &mut impl Printer, mut cmd: Command, shell: Shell) -> Result<()> {
    let name = cmd.get_name().to_string();

    clap_complete::generate(shell, &mut cmd, name, &mut stdout());

    printer.print(format!(
        "Shell script successfully generated for shell {shell}!"
    ))?;

    Ok(())
}
