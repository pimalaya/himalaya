use anyhow::Result;
use clap;
use env_logger;
use log::{debug, trace};
use std::{convert::TryFrom, env, path::PathBuf};

use himalaya::{
    comp,
    config::cli::config_args,
    ctx::Ctx,
    domain::{account::entity::Account, config::entity::Config, smtp::service::SMTPService},
    flag, imap, mbox, msg,
    output::{cli::output_args, model::Output},
};

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

fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "off"),
    );

    // let raw_args: Vec<String> = env::args().collect();

    // // This is used if you click on a mailaddress in the webbrowser
    // if raw_args.len() > 1 && raw_args[1].starts_with("mailto:") {
    //     let config = Config::new(None)?;
    //     let account = config.find_account_by_name(None)?.clone();
    //     let output = Output::new("plain");
    //     let mbox = "INBOX";
    //     let arg_matches = ArgMatches::default();
    //     let app = Ctx::new(config, output, mbox, arg_matches);
    //     let url = Url::parse(&raw_args[1])?;
    //     let smtp = domain::smtp::service::SMTPService::new(&app.account);
    //     return Ok(msg_matches_mailto(&app, &url, smtp)?);
    // }

    let args = parse_args();
    let arg_matches = args.get_matches();

    // Check completion before init config
    if comp::cli::matches(parse_args, &arg_matches)? {
        return Ok(());
    }

    let output = Output::new(arg_matches.value_of("output").unwrap());
    debug!("output: {:?}", output);

    debug!("init config");

    let config_path: PathBuf = arg_matches
        .value_of("config")
        .map(|s| s.into())
        .unwrap_or(Config::path()?);
    debug!("config path: {:?}", config_path);

    let config = Config::try_from(config_path.clone())?;
    trace!("config: {:?}", config);

    let account_name = arg_matches.value_of("account");
    let account = Account::try_from((&config, account_name))?;
    let smtp_service = SMTPService::new(&account)?;
    debug!("account name: {}", account_name.unwrap_or("default"));
    trace!("account: {:?}", account);

    let mbox = arg_matches.value_of("mailbox").unwrap().to_string();
    debug!("mailbox: {}", mbox);

    let ctx = Ctx::new(config, output, mbox, arg_matches);
    trace!("context: {:?}", ctx);

    debug!("begin matching");
    let _matched = mbox::cli::matches(&ctx, &account)?
        || flag::cli::matches(&ctx, &account)?
        || imap::cli::matches(&ctx, &account)?
        || msg::cli::matches(&ctx, &account, smtp_service)?;

    Ok(())
}
