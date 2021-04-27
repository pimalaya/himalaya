use clap;
use env_logger;
use error_chain::error_chain;
use log::{debug, error, trace};
use std::{env, path::PathBuf, process::exit};

mod app;
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
    app::App,
    comp::cli::{comp_matches, comp_subcmds},
    config::{cli::config_args, model::Config},
    flag::cli::{flag_matches, flag_subcmds},
    imap::cli::{imap_matches, imap_subcmds},
    mbox::cli::{mbox_matches, mbox_source_arg, mbox_subcmds},
    msg::cli::{msg_matches, msg_subcmds},
    output::{cli::output_args, model::Output},
};

error_chain! {
    links {
        CompletionCli(crate::comp::cli::Error, crate::comp::cli::ErrorKind);
        Config(crate::config::model::Error, crate::config::model::ErrorKind);
        FlagCli(crate::flag::cli::Error, crate::flag::cli::ErrorKind);
        ImapCli(crate::imap::cli::Error, crate::imap::cli::ErrorKind);
        MboxCli(crate::mbox::cli::Error, crate::mbox::cli::ErrorKind);
        MsgCli(crate::msg::cli::Error, crate::msg::cli::ErrorKind);
    }
}

fn parse_args<'a>() -> clap::App<'a, 'a> {
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
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "off"),
    );

    let args = parse_args();
    let arg_matches = args.get_matches();

    // Check completion before init config
    if comp_matches(parse_args, &arg_matches)? {
        return Ok(());
    }

    let output = Output::new(arg_matches.value_of("output").unwrap());
    debug!("output: {:?}", output);

    debug!("init config");
    let custom_config: Option<PathBuf> = arg_matches.value_of("config").map(|s| s.into());
    debug!("custom config path: {:?}", custom_config);
    let config = Config::new(custom_config)?;
    trace!("config: {:?}", config);

    let account_name = arg_matches.value_of("account");
    debug!("init account: {}", account_name.unwrap_or("default"));
    let account = config.find_account_by_name(account_name)?;
    trace!("account: {:?}", account);

    let mbox = arg_matches.value_of("mailbox").unwrap();
    debug!("mailbox: {}", mbox);

    debug!("begin matching");
    let app = App::new(&config, &account, &output, &mbox, &arg_matches);
    let _matched =
        mbox_matches(&app)? || flag_matches(&app)? || imap_matches(&app)? || msg_matches(&app)?;

    Ok(())
}

fn main() {
    if let Err(ref errs) = run() {
        let mut errs = errs.iter();

        match errs.next() {
            None => (),
            Some(err) => {
                error!("{}", err);
                eprintln!("{}", err);

                errs.for_each(|err| {
                    error!("{}", err);
                    eprintln!(" ↳ {}", err);
                });
            }
        }

        exit(1);
    } else {
        exit(0);
    }
}
