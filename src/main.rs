use clap;
use error_chain::error_chain;
use log::{debug, error, trace};
use std::env;

mod input;
mod smtp;
mod table;
mod config {
    pub(crate) mod cli;
    pub(crate) mod model;
}
mod output {
    pub(crate) mod cli;
    pub(crate) mod fmt;
    pub(crate) mod log;
    pub(crate) mod utils;
}
mod imap {
    pub(crate) mod cli;
    pub(crate) mod model;
}
mod flag {
    pub(crate) mod cli;
    pub(crate) mod model;
}
mod msg {
    pub(crate) mod cli;
    pub(crate) mod model;
}
mod mbox {
    pub(crate) mod cli;
    pub(crate) mod model;
}
mod completion {
    pub(crate) mod cli;
}

use crate::{
    completion::cli::{completion_matches, completion_subcmds},
    config::{cli::account_arg, model::Config},
    flag::cli::{flag_matches, flag_subcmds},
    imap::cli::{imap_matches, imap_subcmds},
    mbox::cli::{mbox_matches, mbox_source_arg, mbox_subcmds},
    msg::cli::{msg_matches, msg_subcmds},
    output::{
        cli::output_args,
        fmt::OutputFmt,
        log::{init as init_logger, LogLevel},
    },
};

error_chain! {
    links {
        CompletionCli(crate::completion::cli::Error, crate::completion::cli::ErrorKind);
        Config(crate::config::model::Error, crate::config::model::ErrorKind);
        FlagCli(crate::flag::cli::Error, crate::flag::cli::ErrorKind);
        ImapCli(crate::imap::cli::Error, crate::imap::cli::ErrorKind);
        MboxCli(crate::mbox::cli::Error, crate::mbox::cli::ErrorKind);
        MsgCli(crate::msg::cli::Error, crate::msg::cli::ErrorKind);
        OutputLog(crate::output::log::Error, crate::output::log::ErrorKind);
    }
}

fn build_app<'a>() -> clap::App<'a, 'a> {
    clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .args(&output_args())
        .arg(account_arg())
        .arg(mbox_source_arg())
        .subcommands(flag_subcmds())
        .subcommands(imap_subcmds())
        .subcommands(mbox_subcmds())
        .subcommands(msg_subcmds())
        .subcommands(completion_subcmds())
}

fn run() -> Result<()> {
    let app = build_app();
    let matches = app.get_matches();

    let output_fmt: OutputFmt = matches.value_of("output").unwrap().into();
    let log_level: LogLevel = matches.value_of("log").unwrap().into();
    init_logger(&output_fmt, &log_level)?;
    debug!("[main] output format: {}", output_fmt);
    debug!("[main] log level: {}", log_level);

    debug!("[main] init config");
    let config = Config::new_from_file()?;
    trace!("[main] {:#?}", config);

    let account_name = matches.value_of("account");
    debug!("[main] find {} account", account_name.unwrap_or("default"));
    let account = config.find_account_by_name(account_name)?;
    trace!("[main] {:#?}", account);

    let mbox = matches.value_of("mailbox").unwrap();
    debug!("[msg::cli::matches] mailbox: {}", mbox);

    debug!("[main] begin matching");
    let _matched = mbox_matches(&account, &matches)?
        || flag_matches(&account, &mbox, &matches)?
        || imap_matches(&config, &account, &mbox, &matches)?
        || completion_matches(build_app(), &matches)?
        || msg_matches(&config, &account, &mbox, &matches)?;

    Ok(())
}

fn main() {
    if let Err(ref errs) = run() {
        let mut errs = errs.iter();
        match errs.next() {
            None => (),
            Some(err) => {
                error!("{}", err);
                errs.for_each(|err| error!(" ↳ {}", err));
            }
        }
    }
}
