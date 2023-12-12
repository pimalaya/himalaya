use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_mangen::Man;
use log::info;
use shellexpand_utils::{canonicalize, expand};
use std::{fs, path::PathBuf};

use crate::{cli::Cli, printer::Printer};

/// Generate manual pages to a directory.
///
/// This command allows you to generate manual pages (following the
/// man page format) to the given directory. If the directory does not
/// exist, it will be created. Any existing man pages will be
/// overriden.
#[derive(Debug, Parser)]
pub struct ManualGenerateCommand {
    /// Directory where man files should be generated in.
    #[arg(value_parser = dir_parser)]
    pub dir: PathBuf,
}

impl ManualGenerateCommand {
    pub async fn execute(self, printer: &mut impl Printer) -> Result<()> {
        info!("executing manual generate command");

        let cmd = Cli::command();
        let cmd_name = cmd.get_name().to_string();
        let subcmds = cmd.get_subcommands().cloned().collect::<Vec<_>>();
        let subcmds_len = subcmds.len() + 1;

        let mut buffer = Vec::new();
        Man::new(cmd).render(&mut buffer)?;

        fs::create_dir_all(&self.dir)?;
        printer.print_log(format!("Generating man page for command {cmd_name}…"))?;
        fs::write(self.dir.join(format!("{}.1", cmd_name)), buffer)?;

        for subcmd in subcmds {
            let subcmd_name = subcmd.get_name().to_string();

            let mut buffer = Vec::new();
            Man::new(subcmd).render(&mut buffer)?;

            printer.print_log(format!("Generating man page for subcommand {subcmd_name}…"))?;
            fs::write(
                self.dir.join(format!("{}-{}.1", cmd_name, subcmd_name)),
                buffer,
            )?;
        }

        printer.print(format!(
            "{subcmds_len} man page(s) successfully generated in {:?}!",
            self.dir
        ))?;

        Ok(())
    }
}

/// Parse the given [`str`] as [`PathBuf`].
///
/// The path is first shell expanded, then canonicalized (if
/// applicable).
fn dir_parser(path: &str) -> Result<PathBuf, String> {
    expand::try_path(path)
        .map(canonicalize::path)
        .map_err(|err| err.to_string())
}
