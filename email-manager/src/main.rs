use anyhow::Result;
use output::StdoutPrinter;
use std::{convert::TryFrom, env};
use url::Url;

mod compl;
mod config;
mod domain;
mod output;
mod ui;

use compl::{compl_arg, compl_handler};
use config::{config_arg, Account, Config};
use domain::{
    imap::{imap_arg, imap_handler, ImapService, ImapServiceInterface},
    mbox::{mbox_arg, mbox_handler, Mbox},
    msg::{flag_arg, flag_handler, msg_arg, msg_handler, tpl_arg, tpl_handler},
    smtp::SmtpService,
};
use output::{output_arg, OutputFmt};

fn create_app<'a>() -> clap::App<'a, 'a> {
    clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .global_setting(clap::AppSettings::GlobalVersion)
        .args(&config_arg::args())
        .args(&output_arg::args())
        .arg(mbox_arg::source_arg())
        .subcommands(compl_arg::subcmds())
        .subcommands(imap_arg::subcmds())
        .subcommands(mbox_arg::subcmds())
        .subcommands(msg_arg::subcmds())
}

#[allow(clippy::single_match)]
fn main() -> Result<()> {
    // Init env logger
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "off"),
    );

    // Check mailto command BEFORE app initialization.
    let raw_args: Vec<String> = env::args().collect();
    if raw_args.len() > 1 && raw_args[1].starts_with("mailto:") {
        let config = Config::try_from(None)?;
        let account = Account::try_from((&config, None))?;
        let mbox = Mbox::new(&account.inbox_folder);
        let mut printer = StdoutPrinter::from(OutputFmt::Plain);
        let url = Url::parse(&raw_args[1])?;
        let mut imap = ImapService::from((&account, &mbox));
        let mut smtp = SmtpService::from(&account);
        return msg_handler::mailto(&url, &account, &mut printer, &mut imap, &mut smtp);
    }

    let app = create_app();
    let m = app.get_matches();

    // Check completion command BEFORE entities and services initialization.
    // Related issue: https://github.com/soywod/himalaya/issues/115.
    match compl_arg::matches(&m)? {
        Some(compl_arg::Command::Generate(shell)) => {
            return compl_handler::generate(create_app(), shell);
        }
        _ => (),
    }

    // Init entities and services.
    let config = Config::try_from(m.value_of("config"))?;
    let account = Account::try_from((&config, m.value_of("account")))?;
    let mbox = Mbox::new(m.value_of("mbox-source").unwrap_or(&account.inbox_folder));
    let mut printer = StdoutPrinter::try_from(m.value_of("output"))?;
    let mut imap = ImapService::from((&account, &mbox));
    let mut smtp = SmtpService::from(&account);

    // Check IMAP commands.
    match imap_arg::matches(&m)? {
        Some(imap_arg::Command::Notify(keepalive)) => {
            return imap_handler::notify(keepalive, &config, &mut imap);
        }
        Some(imap_arg::Command::Watch(keepalive)) => {
            return imap_handler::watch(keepalive, &mut imap);
        }
        _ => (),
    }

    // Check mailbox commands.
    match mbox_arg::matches(&m)? {
        Some(mbox_arg::Cmd::List(max_width)) => {
            return mbox_handler::list(max_width, &mut printer, &mut imap);
        }
        _ => (),
    }

    // Check message commands.
    match msg_arg::matches(&m)? {
        Some(msg_arg::Command::Attachments(seq)) => {
            return msg_handler::attachments(seq, &account, &mut printer, &mut imap);
        }
        Some(msg_arg::Command::Copy(seq, mbox)) => {
            return msg_handler::copy(seq, mbox, &mut printer, &mut imap);
        }
        Some(msg_arg::Command::Delete(seq)) => {
            return msg_handler::delete(seq, &mut printer, &mut imap);
        }
        Some(msg_arg::Command::Forward(seq, atts)) => {
            return msg_handler::forward(seq, atts, &account, &mut printer, &mut imap, &mut smtp);
        }
        Some(msg_arg::Command::List(max_width, page_size, page)) => {
            return msg_handler::list(
                max_width,
                page_size,
                page,
                &account,
                &mut printer,
                &mut imap,
            );
        }
        Some(msg_arg::Command::Move(seq, mbox)) => {
            return msg_handler::move_(seq, mbox, &mut printer, &mut imap);
        }
        Some(msg_arg::Command::Read(seq, text_mime, raw)) => {
            return msg_handler::read(seq, text_mime, raw, &mut printer, &mut imap);
        }
        Some(msg_arg::Command::Reply(seq, all, atts)) => {
            return msg_handler::reply(
                seq,
                all,
                atts,
                &account,
                &mut printer,
                &mut imap,
                &mut smtp,
            );
        }
        Some(msg_arg::Command::Save(raw_msg)) => {
            return msg_handler::save(&mbox, raw_msg, &mut printer, &mut imap);
        }
        Some(msg_arg::Command::Search(query, max_width, page_size, page)) => {
            return msg_handler::search(
                query,
                max_width,
                page_size,
                page,
                &account,
                &mut printer,
                &mut imap,
            );
        }
        Some(msg_arg::Command::Send(raw_msg)) => {
            return msg_handler::send(raw_msg, &account, &mut printer, &mut imap, &mut smtp);
        }
        Some(msg_arg::Command::Write(atts)) => {
            return msg_handler::write(atts, &account, &mut printer, &mut imap, &mut smtp);
        }
        Some(msg_arg::Command::Flag(m)) => match m {
            Some(flag_arg::Command::Set(seq_range, flags)) => {
                return flag_handler::set(seq_range, flags, &mut printer, &mut imap);
            }
            Some(flag_arg::Command::Add(seq_range, flags)) => {
                return flag_handler::add(seq_range, flags, &mut printer, &mut imap);
            }
            Some(flag_arg::Command::Remove(seq_range, flags)) => {
                return flag_handler::remove(seq_range, flags, &mut printer, &mut imap);
            }
            _ => (),
        },
        Some(msg_arg::Command::Tpl(m)) => match m {
            Some(tpl_arg::Command::New(tpl)) => {
                return tpl_handler::new(tpl, &account, &mut printer);
            }
            Some(tpl_arg::Command::Reply(seq, all, tpl)) => {
                return tpl_handler::reply(seq, all, tpl, &account, &mut printer, &mut imap);
            }
            Some(tpl_arg::Command::Forward(seq, tpl)) => {
                return tpl_handler::forward(seq, tpl, &account, &mut printer, &mut imap);
            }
            _ => (),
        },
        _ => (),
    }

    imap.logout()
}
