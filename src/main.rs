use clap::{self, ArgMatches};
use env_logger;
use error_chain::error_chain;
use log::{debug, error, trace};
use std::{env, path::PathBuf, process::exit};
use url::{self, Url};

use himalaya::{
    comp,
    config::{cli::config_args, model::Config},
    ctx::Ctx,
    flag, imap, mbox,
    msg::{self, cli::msg_matches_mailto},
    output::{cli::output_args, model::Output},
};

error_chain! {
    links {
        CompletionCli(himalaya::comp::cli::Error, himalaya::comp::cli::ErrorKind);
        Config(himalaya::config::model::Error, himalaya::config::model::ErrorKind);
        FlagCli(himalaya::flag::cli::Error, himalaya::flag::cli::ErrorKind);
        ImapCli(himalaya::imap::cli::Error, himalaya::imap::cli::ErrorKind);
        MboxCli(himalaya::mbox::cli::Error, himalaya::mbox::cli::ErrorKind);
        MsgCli(himalaya::msg::cli::Error, himalaya::msg::cli::ErrorKind);
    }
    foreign_links {
        Url(url::ParseError);
    }
}

fn parse_args<'a>() -> clap::App<'a, 'a> {
    clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .setting(clap::AppSettings::InferSubcommands)
        .args(&output_args())
        .args(&config_args())
        .arg(mbox::cli::source_arg())
        .subcommands(flag::cli::subcmds())
        .subcommands(imap::cli::subcmds())
        .subcommands(mbox::cli::subcmds())
        .subcommands(msg::cli::subcmds())
        .subcommands(comp::cli::subcmds())
}

fn run() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "off"),
    );

    let raw_args: Vec<String> = env::args().collect();
    if raw_args.len() > 1 && raw_args[1].starts_with("mailto:") {
        let config = Config::new(None)?;
        let account = config.find_account_by_name(None)?;
        let output = Output::new("plain");
        let mbox = "INBOX";
        let arg_matches = ArgMatches::default();
        let app = Ctx::new(&config, &account, &output, &mbox, &arg_matches);
        let url = Url::parse(&raw_args[1])?;
        return Ok(msg_matches_mailto(&app, &url)?);
    }

    let args = parse_args();
    let arg_matches = args.get_matches();

    // Check completion before init config
    if comp::cli::matches(parse_args, &arg_matches)? {
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
    let app = Ctx::new(&config, &account, &output, &mbox, &arg_matches);
    let _matched = mbox::cli::matches(&app)?
        || flag::cli::matches(&app)?
        || imap::cli::matches(&app)?
        || msg::cli::matches(&app)?;

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
