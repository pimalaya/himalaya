mod account;
mod cli;
mod config;
#[cfg(feature = "imap")]
mod imap;
#[cfg(feature = "smtp")]
mod smtp;

use clap::Parser;
use pimalaya_toolbox::terminal::{error::ErrorReport, log::Logger, printer::StdoutPrinter};

use crate::cli::HimalayaCli;

fn main() {
    let cli = HimalayaCli::parse();

    Logger::init(&cli.log);

    let mut printer = StdoutPrinter::new(&cli.json);
    let config_paths = cli.config_paths.as_ref();
    let account_name = cli.account.name.as_deref();

    let result = cli.command.exec(&mut printer, config_paths, account_name);

    ErrorReport::eval(&mut printer, result)
}
