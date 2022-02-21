use anyhow::Result;
use std::{convert::TryFrom, env};
use url::Url;

use himalaya::{
    backends::{imap_arg, imap_handler, Backend, ImapBackend, MaildirBackend},
    compl::{compl_arg, compl_handler},
    config::{account_args, config_args, AccountConfig, BackendConfig, DeserializedConfig},
    domain::msg::{flag_arg, flag_handler, msg_arg, msg_handler, tpl_arg, tpl_handler},
    mbox::{mbox_arg, mbox_handler},
    output::{output_arg, OutputFmt, StdoutPrinter},
    smtp::LettreService,
};

fn create_app<'a>() -> clap::App<'a, 'a> {
    clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .global_setting(clap::AppSettings::GlobalVersion)
        .arg(&config_args::path_arg())
        .arg(&account_args::name_arg())
        .args(&output_arg::args())
        .arg(mbox_arg::source_arg())
        .subcommands(compl_arg::subcmds())
        .subcommands(imap_arg::subcmds())
        .subcommands(mbox_arg::subcmds())
        .subcommands(msg_arg::subcmds())
}

#[allow(clippy::single_match)]
fn main() -> Result<()> {
    let default_env_filter = env_logger::DEFAULT_FILTER_ENV;
    env_logger::init_from_env(env_logger::Env::default().filter_or(default_env_filter, "off"));

    // Check mailto command BEFORE app initialization.
    let raw_args: Vec<String> = env::args().collect();
    if raw_args.len() > 1 && raw_args[1].starts_with("mailto:") {
        let config = DeserializedConfig::from_opt_path(None)?;
        let (account_config, backend_config) =
            AccountConfig::from_config_and_opt_account_name(&config, None)?;
        let mut printer = StdoutPrinter::from(OutputFmt::Plain);
        let url = Url::parse(&raw_args[1])?;
        let mut smtp = LettreService::from(&account_config);

        let mut imap;
        let mut maildir;
        let backend: Box<&mut dyn Backend> = match backend_config {
            BackendConfig::Imap(ref imap_config) => {
                imap = ImapBackend::new(&account_config, imap_config);
                Box::new(&mut imap)
            }
            BackendConfig::Maildir(ref maildir_config) => {
                maildir = MaildirBackend::new(&account_config, maildir_config);
                Box::new(&mut maildir)
            }
        };

        return msg_handler::mailto(&url, &account_config, &mut printer, backend, &mut smtp);
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
    let config = DeserializedConfig::from_opt_path(m.value_of("config"))?;
    let (account_config, backend_config) =
        AccountConfig::from_config_and_opt_account_name(&config, m.value_of("account"))?;
    let mbox = m
        .value_of("mbox-source")
        .unwrap_or(&account_config.inbox_folder);
    let mut printer = StdoutPrinter::try_from(m.value_of("output"))?;
    let mut imap;
    let mut maildir;
    let backend: Box<&mut dyn Backend> = match backend_config {
        BackendConfig::Imap(ref imap_config) => {
            imap = ImapBackend::new(&account_config, imap_config);
            Box::new(&mut imap)
        }
        BackendConfig::Maildir(ref maildir_config) => {
            maildir = MaildirBackend::new(&account_config, maildir_config);
            Box::new(&mut maildir)
        }
    };

    let mut smtp = LettreService::from(&account_config);

    // Check IMAP commands.
    if let BackendConfig::Imap(ref imap_config) = backend_config {
        let mut imap = ImapBackend::new(&account_config, imap_config);
        match imap_arg::matches(&m)? {
            Some(imap_arg::Command::Notify(keepalive)) => {
                return imap_handler::notify(keepalive, mbox, &mut imap);
            }
            Some(imap_arg::Command::Watch(keepalive)) => {
                return imap_handler::watch(keepalive, mbox, &mut imap);
            }
            _ => (),
        }
    }

    // Check mailbox commands.
    match mbox_arg::matches(&m)? {
        Some(mbox_arg::Cmd::List(max_width)) => {
            return mbox_handler::list(max_width, &mut printer, backend);
        }
        _ => (),
    }

    // Check message commands.
    match msg_arg::matches(&m)? {
        Some(msg_arg::Cmd::Attachments(seq)) => {
            return msg_handler::attachments(seq, mbox, &account_config, &mut printer, backend);
        }
        Some(msg_arg::Cmd::Copy(seq, mbox_dst)) => {
            return msg_handler::copy(seq, mbox, mbox_dst, &mut printer, backend);
        }
        Some(msg_arg::Cmd::Delete(seq)) => {
            return msg_handler::delete(seq, mbox, &mut printer, backend);
        }
        Some(msg_arg::Cmd::Forward(seq, attachment_paths, encrypt)) => {
            return msg_handler::forward(
                seq,
                attachment_paths,
                encrypt,
                mbox,
                &account_config,
                &mut printer,
                backend,
                &mut smtp,
            );
        }
        Some(msg_arg::Cmd::List(max_width, page_size, page)) => {
            return msg_handler::list(
                max_width,
                page_size,
                page,
                mbox,
                &account_config,
                &mut printer,
                backend,
            );
        }
        Some(msg_arg::Cmd::Move(seq, mbox_dst)) => {
            return msg_handler::move_(seq, mbox, mbox_dst, &mut printer, backend);
        }
        Some(msg_arg::Cmd::Read(seq, text_mime, raw)) => {
            return msg_handler::read(seq, text_mime, raw, mbox, &mut printer, backend);
        }
        Some(msg_arg::Cmd::Reply(seq, all, attachment_paths, encrypt)) => {
            return msg_handler::reply(
                seq,
                all,
                attachment_paths,
                encrypt,
                mbox,
                &account_config,
                &mut printer,
                backend,
                &mut smtp,
            );
        }
        Some(msg_arg::Cmd::Save(raw_msg)) => {
            return msg_handler::save(mbox, raw_msg, &mut printer, backend);
        }
        Some(msg_arg::Cmd::Search(query, max_width, page_size, page)) => {
            return msg_handler::search(
                query,
                max_width,
                page_size,
                page,
                mbox,
                &account_config,
                &mut printer,
                backend,
            );
        }
        Some(msg_arg::Cmd::Sort(criteria, query, max_width, page_size, page)) => {
            return msg_handler::sort(
                criteria,
                query,
                max_width,
                page_size,
                page,
                mbox,
                &account_config,
                &mut printer,
                backend,
            );
        }
        Some(msg_arg::Cmd::Send(raw_msg)) => {
            return msg_handler::send(raw_msg, &account_config, &mut printer, backend, &mut smtp);
        }
        Some(msg_arg::Cmd::Write(atts, encrypt)) => {
            return msg_handler::write(
                atts,
                encrypt,
                &account_config,
                &mut printer,
                backend,
                &mut smtp,
            );
        }
        Some(msg_arg::Cmd::Flag(m)) => match m {
            Some(flag_arg::Cmd::Set(seq_range, flags)) => {
                return flag_handler::set(seq_range, mbox, &flags, &mut printer, backend);
            }
            Some(flag_arg::Cmd::Add(seq_range, flags)) => {
                return flag_handler::add(seq_range, mbox, &flags, &mut printer, backend);
            }
            Some(flag_arg::Cmd::Remove(seq_range, flags)) => {
                return flag_handler::remove(seq_range, mbox, &flags, &mut printer, backend);
            }
            _ => (),
        },
        Some(msg_arg::Cmd::Tpl(m)) => match m {
            Some(tpl_arg::Cmd::New(tpl)) => {
                return tpl_handler::new(tpl, &account_config, &mut printer);
            }
            Some(tpl_arg::Cmd::Reply(seq, all, tpl)) => {
                return tpl_handler::reply(
                    seq,
                    all,
                    tpl,
                    mbox,
                    &account_config,
                    &mut printer,
                    backend,
                );
            }
            Some(tpl_arg::Cmd::Forward(seq, tpl)) => {
                return tpl_handler::forward(
                    seq,
                    tpl,
                    mbox,
                    &account_config,
                    &mut printer,
                    backend,
                );
            }
            Some(tpl_arg::Cmd::Save(atts, tpl)) => {
                return tpl_handler::save(mbox, &account_config, atts, tpl, &mut printer, backend);
            }
            Some(tpl_arg::Cmd::Send(atts, tpl)) => {
                return tpl_handler::send(
                    mbox,
                    &account_config,
                    atts,
                    tpl,
                    &mut printer,
                    backend,
                    &mut smtp,
                );
            }
            _ => (),
        },
        _ => (),
    }

    backend.disconnect()
}
