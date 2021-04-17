use clap;
use error_chain::error_chain;
use log::{debug, error, trace};
use std::{env, path::PathBuf};

mod comp;
mod config;
mod flag;
mod imap;
mod input;
mod mbox;
mod msg;
mod output;
mod smtp;
mod table;

use crate::{
    comp::cli::{comp_matches, comp_subcmds},
    config::{cli::config_args, model::Config},
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
        CompletionCli(crate::comp::cli::Error, crate::comp::cli::ErrorKind);
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
        .args(&config_args())
        .arg(mbox_source_arg())
        .subcommands(flag_subcmds())
        .subcommands(imap_subcmds())
        .subcommands(mbox_subcmds())
        .subcommands(msg_subcmds())
        .subcommands(comp_subcmds())
}

fn run() -> Result<()> {
    let app = build_app();
    let matches = app.get_matches();

    let output_fmt: OutputFmt = matches.value_of("output").unwrap().into();
    let log_level: LogLevel = matches.value_of("log").unwrap().into();
    let custom_config: Option<PathBuf> = matches.value_of("config").map(|s| s.into());
    init_logger(&output_fmt, &log_level)?;
    debug!("[main] output format: {}", output_fmt);
    debug!("[main] log level: {}", log_level);
    debug!("[main] custom config path: {:?}", custom_config);

    debug!("[main] init config");
    let config = Config::new(custom_config)?;
    trace!("[main] {:#?}", config);

    let account_name = matches.value_of("account");
    debug!("[main] find {} account", account_name.unwrap_or("default"));
    let account = config.find_account_by_name(account_name)?;
    trace!("[main] {:#?}", account);

    let mbox = matches.value_of("mailbox").unwrap();
    debug!("[main] mailbox: {}", mbox);

    debug!("[main] begin matching");
    let _matched = mbox_matches(&account, &matches)?
        || flag_matches(&account, &mbox, &matches)?
        || imap_matches(&config, &account, &mbox, &matches)?
        || comp_matches(build_app(), &matches)?
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
