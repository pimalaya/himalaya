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
        .setting(clap::AppSettings::InferSubcommands)
        .args(&output_args())
        .args(&config_args())
        .arg(mbox::cli::source_arg())
        .subcommands(flag::cli::subcmds())
        .subcommands(imap::cli::subcmds())
        .subcommands(mbox::cli::subcmds())
        .subcommands(msg::cli::subcmds())
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

    // Check shell completion BEFORE any entity or service initialization.
    if let Some(compl::arg::Match::Generate(shell)) = compl::arg::matches(&m)? {
        return compl::handler::generate(shell, create_app());
    }

    let mbox = Mbox::try_from(m.value_of("mailbox"))?;
    let config = Config::try_from(m.value_of("config"))?;
    let account = Account::try_from((&config, m.value_of("account")))?;
    let output = OutputService::try_from(m.value_of("output"))?;
    let mut imap = ImapService::from((&account, &mbox));
    let mut smtp = SmtpService::from(&account);

    // TODO: use same system as compl
    let _matched = mbox::cli::matches(&m, &output, &mut imap)?
        || flag::cli::matches(&m, &mut imap)?
        || imap::cli::matches(&m, &config, &mut imap)?
        || msg::cli::matches(&m, &mbox, &account, &output, &mut imap, &mut smtp)?;

    Ok(())
}
