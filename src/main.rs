mod account;
mod backend;
mod cli;
mod config;
#[cfg(feature = "imap")]
mod imap;
#[cfg(feature = "jmap")]
mod jmap;
#[cfg(feature = "maildir")]
mod maildir;
mod shared;
#[cfg(feature = "smtp")]
mod smtp;
mod wizard;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{error::ErrorReport, log::Logger, printer::StdoutPrinter};

use crate::cli::HimalayaCli;

fn main() {
    let cli = HimalayaCli::parse();
    let mut printer = StdoutPrinter::new(&cli.json);
    let result = execute(cli, &mut printer);
    ErrorReport::eval(&mut printer, result);
}

fn execute(cli: HimalayaCli, printer: &mut StdoutPrinter) -> Result<()> {
    Logger::try_init(&cli.log)?;
    let config = cli.config_paths.as_ref();
    let account = cli.account.name.as_deref();
    let backend = cli.backend;
    cli.command.execute(printer, config, account, backend)
}
