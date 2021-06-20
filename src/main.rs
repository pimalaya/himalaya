use clap;
use env_logger;
use error_chain::error_chain;
use log::{debug, error, trace};
use std::{env, path::PathBuf, process::exit};

use himalaya::{
    // All subcommands and cli-arguments when calling himalaya
    cli,
    ctx::Ctx,
    config::model::Config,
    output::model::Output,
};

error_chain! {
    links {
        Config(himalaya::config::model::Error, himalaya::config::model::ErrorKind);
        FlagCli(himalaya::cli::flag::Error, himalaya::cli::flag::ErrorKind);
        ImapCli(himalaya::cli::imap::Error, himalaya::cli::imap::ErrorKind);
        MboxCli(himalaya::cli::mbox::Error, himalaya::cli::mbox::ErrorKind);
        MsgCli(himalaya::cli::msg::Error, himalaya::cli::msg::ErrorKind);
    }
}

fn parse_args<'a>() -> clap::App<'a, 'a> {
    clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .setting(clap::AppSettings::InferSubcommands)
        .args(&cli::output::options())
        .args(&cli::config::options())
        .args(&cli::mbox::options())
        .subcommands(cli::flag::subcmds())
        .subcommands(cli::imap::subcmds())
        .subcommands(cli::mbox::subcmds())
        .subcommands(cli::msg::subcmds())
        .subcommands(cli::shell_completion::subcmds())
}

fn run() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "off"),
        );

    let args = parse_args();
    let arg_matches = args.get_matches();

    // Check completion before init config
    if cli::shell_completion::matches(parse_args, &arg_matches) {
        return Ok(());
    }

    let output = Output::new(arg_matches.value_of("output").unwrap());
    debug!("output: {:?}", output);
    debug!("-- Init config --");

    let custom_config: Option<PathBuf> = arg_matches.value_of("config").map(|s| s.into());
    debug!("Custom config path: {:?}", custom_config);

    let config = Config::new(custom_config)?;
    trace!("Config: {:?}", config);

    let account_name = arg_matches.value_of("account");
    debug!("Init account: {}", account_name.unwrap_or("default"));
    let account = config.find_account_by_name(account_name)?;
    trace!("Account: {:?}", account);

    let mbox = arg_matches.value_of("mailbox").unwrap();
    debug!("Mailbox: {}", mbox);

    debug!("-- Begin matching --");
    let app = Ctx::new(&config, &account, &output, &mbox, &arg_matches);
    let _matched = cli::mbox::matches(&app)?
        || cli::flag::matches(&app)?
        || cli::imap::matches(&app)?
        || cli::msg::matches(&app)?;

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
