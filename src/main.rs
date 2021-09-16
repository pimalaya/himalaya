use anyhow::Result;
use clap;
use env_logger;
use std::{convert::TryFrom, env};

use himalaya::{
    compl,
    config::cli::config_args,
    domain::{
        account::entity::Account,
        config::entity::Config,
        imap::{self, service::ImapService},
        mbox::{self, entity::Mbox},
        msg,
        smtp::service::SmtpService,
    },
    flag,
    output::{cli::output_args, service::OutputService},
};

fn create_app<'a>() -> clap::App<'a, 'a> {
    clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .args(&output_args())
        .args(&config_args())
        .arg(mbox::arg::source_arg())
        .subcommands(flag::arg::subcmds())
        .subcommands(imap::arg::subcmds())
        .subcommands(mbox::arg::subcmds())
        .subcommands(msg::arg::subcmds())
        .subcommands(compl::arg::subcmds())
}

fn main() -> Result<()> {
    // Init env logger
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "off"),
    );

    // TODO: put in a `mailto` module
    // let raw_args: Vec<String> = env::args().collect();
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

    let app = create_app();
    let m = app.get_matches();

    // Check completion match BEFORE any entity or service initialization.
    // See https://github.com/soywod/himalaya/issues/115.
    match compl::arg::matches(&m)? {
        Some(compl::arg::Match::Generate(shell)) => {
            return compl::handler::generate(shell, create_app());
        }
        _ => (),
    }

    let mbox = Mbox::try_from(m.value_of("mailbox"))?;
    let config = Config::try_from(m.value_of("config"))?;
    let account = Account::try_from((&config, m.value_of("account")))?;
    let output = OutputService::try_from(m.value_of("output"))?;
    let mut imap = ImapService::from((&account, &mbox));
    let mut smtp = SmtpService::from(&account);

    // Check IMAP matches.
    match imap::arg::matches(&m)? {
        Some(imap::arg::Match::Notify(keepalive)) => {
            return imap::handler::notify(keepalive, &config, &mut imap);
        }
        Some(imap::arg::Match::Watch(keepalive)) => {
            return imap::handler::watch(keepalive, &mut imap);
        }
        _ => (),
    }

    // Check mailbox matches.
    match mbox::arg::matches(&m)? {
        Some(mbox::arg::Match::List) => {
            return mbox::handler::list(&output, &mut imap);
        }
        _ => (),
    }

    // TODO: use same system as compl
    let _matched = flag::arg::matches(&m, &mut imap)?
        || msg::arg::matches(&m, &mbox, &account, &output, &mut imap, &mut smtp)?;

    Ok(())
}
