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

use clap;
use error_chain::error_chain;
use log::{debug, error};
use std::env;

use crate::{
    config::cli::account_arg,
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
        FlagCli(crate::flag::cli::Error, crate::flag::cli::ErrorKind);
        ImapCli(crate::imap::cli::Error, crate::imap::cli::ErrorKind);
        MboxCli(crate::mbox::cli::Error, crate::mbox::cli::ErrorKind);
        MsgCli(crate::msg::cli::Error, crate::msg::cli::ErrorKind);
        OutputLog(crate::output::log::Error, crate::output::log::ErrorKind);
    }
}

fn run() -> Result<()> {
    let matches = clap::App::new(env!("CARGO_PKG_NAME"))
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
        .get_matches();

    let output_fmt: OutputFmt = matches.value_of("output").unwrap().into();
    let log_level: LogLevel = matches.value_of("log").unwrap().into();

    init_logger(&output_fmt, &log_level)?;
    debug!("Logger initialized");
    debug!("Output format: {}", &output_fmt);
    debug!("Log level: {}", &log_level);

    loop {
        if mbox_matches(&matches)? {
            break;
        }

        if flag_matches(&matches)? {
            break;
        }

        if imap_matches(&matches)? {
            break;
        }

        msg_matches(&matches)?;
        break;
    }

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
