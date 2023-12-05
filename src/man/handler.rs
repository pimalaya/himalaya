use anyhow::Result;
use clap::Command;
use clap_mangen::Man;
use std::{fs, path::PathBuf};

use crate::printer::Printer;

pub fn generate(printer: &mut impl Printer, cmd: Command, dir: PathBuf) -> Result<()> {
    let cmd_name = cmd.get_name().to_string();
    let subcmds = cmd.get_subcommands().cloned().collect::<Vec<_>>();
    let subcmds_len = subcmds.len() + 1;

    let mut buffer = Vec::new();
    Man::new(cmd).render(&mut buffer)?;

    fs::create_dir_all(&dir)?;
    printer.print_log(format!("Generating man page for command {cmd_name}…"))?;
    fs::write(dir.join(format!("{}.1", cmd_name)), buffer)?;

    for subcmd in subcmds {
        let subcmd_name = subcmd.get_name().to_string();

        let mut buffer = Vec::new();
        Man::new(subcmd).render(&mut buffer)?;

        printer.print_log(format!("Generating man page for subcommand {subcmd_name}…"))?;
        fs::write(dir.join(format!("{}-{}.1", cmd_name, subcmd_name)), buffer)?;
    }

    printer.print(format!(
        "Successfully generated {subcmds_len} man page(s) in {dir:?}!"
    ))?;

    Ok(())
}
