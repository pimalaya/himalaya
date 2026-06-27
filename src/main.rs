mod account;
mod backend;
mod cli;
mod config;
#[cfg(feature = "gmail")]
mod gmail;
#[cfg(feature = "imap")]
mod imap;
#[cfg(feature = "jmap")]
mod jmap;
#[cfg(feature = "m2dir")]
mod m2dir;
#[cfg(feature = "maildir")]
mod maildir;
#[cfg(feature = "msgraph")]
mod msgraph;
mod shared;
#[cfg(feature = "smtp")]
mod smtp;
mod wizard;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{error::ErrorReport, log::Logger, printer::StdoutPrinter};

use crate::cli::Cli;

fn main() {
    let cli = Cli::parse();
    let mut printer = StdoutPrinter::new(&cli.json);
    let result = execute(cli, &mut printer);
    ErrorReport::eval(&mut printer, result);
}

fn execute(cli: Cli, printer: &mut StdoutPrinter) -> Result<()> {
    Logger::try_init(&cli.log)?;
    let config = cli.config.paths.as_ref();
    let account = cli.account.name.as_deref();
    let backend = cli.backend;
    cli.cmd.execute(printer, config, account, backend)
}
