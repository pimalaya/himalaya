use anyhow::Result;
use clap;
use env_logger;
use std::{convert::TryFrom, env};
use url::Url;

use himalaya::{
    compl,
    config::{self, entity::Config},
    domain::{
        account::{self, entity::Account},
        imap::{self, service::ImapService},
        mbox::{self, entity::Mbox},
        msg,
        smtp::service::SmtpService,
    },
    output::{cli::output_args, service::OutputService},
};

fn create_app<'a>() -> clap::App<'a, 'a> {
    clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .args(&output_args())
        .arg(config::arg::path_arg())
        .arg(account::arg::name_arg())
        .arg(mbox::arg::source_arg())
        .subcommands(compl::arg::subcmds())
        .subcommands(imap::arg::subcmds())
        .subcommands(mbox::arg::subcmds())
        .subcommands(msg::arg::subcmds())
}

fn main() -> Result<()> {
    // Init env logger
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "off"),
    );

    // Check mailto match BEFORE app initialization.
    let raw_args: Vec<String> = env::args().collect();
    if raw_args.len() > 1 && raw_args[1].starts_with("mailto:") {
        let mbox = Mbox::from("INBOX");
        let config = Config::try_from(None)?;
        let account = Account::try_from((&config, None))?;
        let output = OutputService::from("plain");
        let url = Url::parse(&raw_args[1])?;
        let mut imap = ImapService::from((&account, &mbox));
        let mut smtp = SmtpService::from(&account);
        return msg::handler::mailto(&url, &account, &output, &mut imap, &mut smtp);
    }

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

    // Check message matches.
    match msg::arg::matches(&m)? {
        Some(msg::arg::Match::Attachments(uid)) => {
            return msg::handler::attachments(uid, &account, &output, &mut imap);
        }
        Some(msg::arg::Match::Copy(uid, mbox)) => {
            return msg::handler::copy(uid, mbox, &output, &mut imap);
        }
        Some(msg::arg::Match::Delete(uid)) => {
            return msg::handler::delete(uid, &output, &mut imap);
        }
        Some(msg::arg::Match::Forward(uid, paths)) => {
            return msg::handler::forward(uid, paths, &account, &output, &mut imap, &mut smtp);
        }
        Some(msg::arg::Match::List(page_size, page)) => {
            return msg::handler::list(page_size, page, &account, &output, &mut imap);
        }
        Some(msg::arg::Match::Move(uid, mbox)) => {
            return msg::handler::move_(uid, mbox, &output, &mut imap);
        }
        Some(msg::arg::Match::Read(uid, mime, raw)) => {
            return msg::handler::read(uid, mime, raw, &output, &mut imap);
        }
        Some(msg::arg::Match::Reply(uid, all, paths)) => {
            return msg::handler::reply(uid, all, paths, &account, &output, &mut imap, &mut smtp);
        }
        Some(msg::arg::Match::Save(mbox, msg)) => {
            return msg::handler::save(mbox, msg, &mut imap);
        }
        Some(msg::arg::Match::Search(query, page_size, page)) => {
            return msg::handler::search(page_size, page, query, &account, &output, &mut imap);
        }
        Some(msg::arg::Match::Send(msg)) => {
            return msg::handler::send(msg, &output, &mut imap, &mut smtp);
        }
        Some(msg::arg::Match::Write(paths)) => {
            return msg::handler::write(paths, &account, &output, &mut imap, &mut smtp);
        }

        Some(msg::arg::Match::Flag(m)) => match m {
            msg::flag::arg::Match::Set(uid, flags) => {
                return msg::flag::handler::set(uid, flags, &mut imap);
            }
            msg::flag::arg::Match::Add(uid, flags) => {
                return msg::flag::handler::add(uid, flags, &mut imap);
            }
            msg::flag::arg::Match::Remove(uid, flags) => {
                return msg::flag::handler::remove(uid, flags, &mut imap);
            }
        },
        Some(msg::arg::Match::Tpl(m)) => match m {
            msg::tpl::arg::Match::New => {
                return msg::tpl::handler::new(&account, &output, &mut imap);
            }
            msg::tpl::arg::Match::Reply(uid, all) => {
                return msg::tpl::handler::reply(uid, all, &account, &output, &mut imap);
            }
            msg::tpl::arg::Match::Forward(uid) => {
                return msg::tpl::handler::forward(uid, &account, &output, &mut imap);
            }
        },
        _ => (),
    }

    Ok(())
}
