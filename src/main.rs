use anyhow::Result;
use clap;
use env_logger;
use log::{debug, trace};
use std::{convert::TryFrom, env, path::PathBuf};

use himalaya::{
    comp,
    config::cli::config_args,
    domain::{
        account::entity::Account, config::entity::Config, imap::ImapService, smtp::SmtpService,
    },
    flag, imap, mbox, msg,
    output::{cli::output_args, service::OutputService},
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

    debug!("init output service");
    let output = OutputService::new(arg_matches.value_of("output").unwrap());
    debug!("output service: {:?}", output);

    debug!("init mbox");
    let mbox = arg_matches.value_of("mailbox").unwrap();
    debug!("mbox: {}", mbox);

    let config_path: PathBuf = arg_matches
        .value_of("config")
        .map(|s| s.into())
        .unwrap_or(Config::path()?);
    debug!("init config from `{:?}`", config_path);
    let config = Config::try_from(config_path.clone())?;
    trace!("{:#?}", config);

    let account_name = arg_matches.value_of("account");
    debug!("init account `{}`", account_name.unwrap_or("default"));
    let account = Account::try_from((&config, account_name))?;
    trace!("{:#?}", account);

    debug!("init IMAP service");
    let mut imap = ImapService::new(&account, &mbox)?;

    debug!("init SMTP service");
    let mut smtp = SmtpService::new(&account)?;

    debug!("begin matching");
    let _matched = mbox::cli::matches(&arg_matches, &output, &mut imap)?
        || flag::cli::matches(&arg_matches, &mut imap)?
        || imap::cli::matches(&arg_matches, &config, &mut imap)?
        || msg::cli::matches(&arg_matches, mbox, &account, &output, &mut imap, &mut smtp)?;

    Ok(())
}
