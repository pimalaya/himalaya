use anyhow::Result;
use clap::{self, AppSettings};
use env_logger;
use std::{convert::TryFrom, env};
use url::Url;

mod compl;
mod config;
mod domain;
mod output;
mod ui;

use config::entity::{Account, Config};
use domain::{
    imap::{self, ImapService, ImapServiceInterface},
    mbox::{self, entity::Mbox},
    msg,
    smtp::service::SmtpService,
};
use output::service::OutputService;

fn create_app<'a>() -> clap::App<'a, 'a> {
    clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .setting(AppSettings::GlobalVersion)
        .args(&config::arg::args())
        .args(&output::arg::args())
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

    // Check completion match BEFORE entities and services initialization.
    // Linked issue: https://github.com/soywod/himalaya/issues/115.
    match compl::arg::matches(&m)? {
        Some(compl::arg::Command::Generate(shell)) => {
            return compl::handler::generate(create_app(), shell);
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
        Some(imap::arg::Command::Notify(keepalive)) => {
            return imap::handler::notify(keepalive, &config, &mut imap);
        }
        Some(imap::arg::Command::Watch(keepalive)) => {
            return imap::handler::watch(keepalive, &mut imap);
        }
        _ => (),
    }

    // Check mailbox matches.
    match mbox::arg::matches(&m)? {
        Some(mbox::arg::Command::List) => {
            return mbox::handler::list(&output, &mut imap);
        }
        _ => (),
    }

    // Check message matches.
    match msg::arg::matches(&m)? {
        Some(msg::arg::Command::Attachments(seq)) => {
            return msg::handler::attachments(seq, &account, &output, &mut imap);
        }
        Some(msg::arg::Command::Copy(seq, target)) => {
            return msg::handler::copy(seq, target, &output, &mut imap);
        }
        Some(msg::arg::Command::Delete(seq)) => {
            return msg::handler::delete(seq, &output, &mut imap);
        }
        Some(msg::arg::Command::Forward(seq, atts)) => {
            return msg::handler::forward(seq, atts, &account, &output, &mut imap, &mut smtp);
        }
        Some(msg::arg::Command::List(page_size, page)) => {
            return msg::handler::list(page_size, page, &account, &output, &mut imap);
        }
        Some(msg::arg::Command::Move(seq, target)) => {
            return msg::handler::move_(seq, target, &output, &mut imap);
        }
        Some(msg::arg::Command::Read(seq, mime, raw)) => {
            return msg::handler::read(seq, mime, raw, &output, &mut imap);
        }
        Some(msg::arg::Command::Reply(seq, all, atts)) => {
            return msg::handler::reply(seq, all, atts, &account, &output, &mut imap, &mut smtp);
        }
        Some(msg::arg::Command::Save(target, msg)) => {
            return msg::handler::save(target, msg, &mut imap);
        }
        Some(msg::arg::Command::Search(query, page_size, page)) => {
            return msg::handler::search(query, page_size, page, &account, &output, &mut imap);
        }
        Some(msg::arg::Command::Send(raw_msg)) => {
            return msg::handler::send(raw_msg, &output, &mut imap, &mut smtp);
        }
        Some(msg::arg::Command::Write(atts)) => {
            return msg::handler::write(atts, &account, &output, &mut imap, &mut smtp);
        }
        Some(msg::arg::Command::Flag(m)) => match m {
            Some(msg::flag::arg::Command::Set(seq_range, flags)) => {
                return msg::flag::handler::set(seq_range, flags, &output, &mut imap);
            }
            Some(msg::flag::arg::Command::Add(seq_range, flags)) => {
                return msg::flag::handler::add(seq_range, flags, &output, &mut imap);
            }
            Some(msg::flag::arg::Command::Remove(seq_range, flags)) => {
                return msg::flag::handler::remove(seq_range, flags, &output, &mut imap);
            }
            _ => (),
        },
        Some(msg::arg::Command::Tpl(m)) => match m {
            Some(msg::tpl::arg::Command::New(tpl)) => {
                return msg::tpl::handler::new(tpl, &account, &output);
            }
            Some(msg::tpl::arg::Command::Reply(seq, all, tpl)) => {
                return msg::tpl::handler::reply(seq, all, tpl, &account, &output, &mut imap);
            }
            Some(msg::tpl::arg::Command::Forward(seq, tpl)) => {
                return msg::tpl::handler::forward(seq, tpl, &account, &output, &mut imap);
            }
            _ => (),
        },
        _ => (),
    }

    imap.logout()
}
