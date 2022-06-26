use anyhow::{Context, Result};
use himalaya_lib::{
    account::{AccountConfig, BackendConfig, DeserializedConfig, DEFAULT_INBOX_FOLDER},
    backend::Backend,
};
use std::{convert::TryFrom, env};
use url::Url;

use himalaya::{
    compl::{compl_args, compl_handlers},
    config::{account_args, account_handlers, config_args},
    mbox::{mbox_args, mbox_handlers},
    msg::{flag_args, flag_handlers, msg_args, msg_handlers, tpl_args, tpl_handlers},
    output::{output_args, OutputFmt, StdoutPrinter},
    smtp::LettreService,
};

#[cfg(feature = "imap-backend")]
use himalaya::imap::{imap_args, imap_handlers};

#[cfg(feature = "imap-backend")]
use himalaya_lib::backend::ImapBackend;

#[cfg(feature = "maildir-backend")]
use himalaya_lib::backend::MaildirBackend;

#[cfg(feature = "notmuch-backend")]
use himalaya_lib::{account::MaildirBackendConfig, backend::NotmuchBackend};

fn create_app<'a>() -> clap::App<'a, 'a> {
    let app = clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .global_setting(clap::AppSettings::GlobalVersion)
        .arg(&config_args::path_arg())
        .arg(&account_args::name_arg())
        .args(&output_args::args())
        .arg(mbox_args::source_arg())
        .subcommands(compl_args::subcmds())
        .subcommands(account_args::subcmds())
        .subcommands(mbox_args::subcmds())
        .subcommands(msg_args::subcmds());

    #[cfg(feature = "imap-backend")]
    let app = app.subcommands(imap_args::subcmds());

    app
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

        #[cfg(feature = "imap-backend")]
        let mut imap;

        #[cfg(feature = "maildir-backend")]
        let mut maildir;

        #[cfg(feature = "notmuch-backend")]
        let maildir_config: MaildirBackendConfig;
        #[cfg(feature = "notmuch-backend")]
        let mut notmuch;

        let backend: Box<&mut dyn Backend> = match backend_config {
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(ref imap_config) => {
                imap = ImapBackend::new(&account_config, imap_config);
                Box::new(&mut imap)
            }
            #[cfg(feature = "maildir-backend")]
            BackendConfig::Maildir(ref maildir_config) => {
                maildir = MaildirBackend::new(&account_config, maildir_config);
                Box::new(&mut maildir)
            }
            #[cfg(feature = "notmuch-backend")]
            BackendConfig::Notmuch(ref notmuch_config) => {
                maildir_config = MaildirBackendConfig {
                    maildir_dir: notmuch_config.notmuch_database_dir.clone(),
                };
                maildir = MaildirBackend::new(&account_config, &maildir_config);
                notmuch = NotmuchBackend::new(&account_config, notmuch_config, &mut maildir)?;
                Box::new(&mut notmuch)
            }
        };

        return msg_handlers::mailto(&url, &account_config, &mut printer, backend, &mut smtp);
    }

    let app = create_app();
    let m = app.get_matches();

    // Check completion command BEFORE entities and services initialization.
    // Related issue: https://github.com/soywod/himalaya/issues/115.
    match compl_args::matches(&m)? {
        Some(compl_args::Command::Generate(shell)) => {
            return compl_handlers::generate(create_app(), shell);
        }
        _ => (),
    }

    // Init entities and services.
    let config = DeserializedConfig::from_opt_path(m.value_of("config"))?;
    let (account_config, backend_config) =
        AccountConfig::from_config_and_opt_account_name(&config, m.value_of("account"))?;
    let mbox = m
        .value_of("mbox-source")
        .or_else(|| account_config.mailboxes.get("inbox").map(|s| s.as_str()))
        .unwrap_or(DEFAULT_INBOX_FOLDER);
    let mut printer = StdoutPrinter::try_from(m.value_of("output"))?;
    #[cfg(feature = "imap-backend")]
    let mut imap;

    #[cfg(feature = "maildir-backend")]
    let mut maildir;

    #[cfg(feature = "notmuch-backend")]
    let maildir_config: MaildirBackendConfig;
    #[cfg(feature = "notmuch-backend")]
    let mut notmuch;

    let backend: Box<&mut dyn Backend> = match backend_config {
        #[cfg(feature = "imap-backend")]
        BackendConfig::Imap(ref imap_config) => {
            imap = ImapBackend::new(&account_config, imap_config);
            Box::new(&mut imap)
        }
        #[cfg(feature = "maildir-backend")]
        BackendConfig::Maildir(ref maildir_config) => {
            maildir = MaildirBackend::new(&account_config, maildir_config);
            Box::new(&mut maildir)
        }
        #[cfg(feature = "notmuch-backend")]
        BackendConfig::Notmuch(ref notmuch_config) => {
            maildir_config = MaildirBackendConfig {
                maildir_dir: notmuch_config.notmuch_database_dir.clone(),
            };
            maildir = MaildirBackend::new(&account_config, &maildir_config);
            notmuch = NotmuchBackend::new(&account_config, notmuch_config, &mut maildir)?;
            Box::new(&mut notmuch)
        }
    };

    let mut smtp = LettreService::from(&account_config);

    // Check IMAP commands.
    #[allow(irrefutable_let_patterns)]
    #[cfg(feature = "imap-backend")]
    if let BackendConfig::Imap(ref imap_config) = backend_config {
        let mut imap = ImapBackend::new(&account_config, imap_config);
        match imap_args::matches(&m)? {
            Some(imap_args::Command::Notify(keepalive)) => {
                return imap_handlers::notify(keepalive, mbox, &mut imap);
            }
            Some(imap_args::Command::Watch(keepalive)) => {
                return imap_handlers::watch(keepalive, mbox, &mut imap);
            }
            _ => (),
        }
    }

    // Check account commands.
    match account_args::matches(&m)? {
        Some(account_args::Cmd::List(max_width)) => {
            return account_handlers::list(max_width, &config, &account_config, &mut printer);
        }
        _ => (),
    }

    // Check mailbox commands.
    match mbox_args::matches(&m)? {
        Some(mbox_args::Cmd::List(max_width)) => {
            return mbox_handlers::list(max_width, &account_config, &mut printer, backend);
        }
        _ => (),
    }

    // Check message commands.
    match msg_args::matches(&m)? {
        Some(msg_args::Cmd::Attachments(seq)) => {
            return msg_handlers::attachments(seq, mbox, &account_config, &mut printer, backend);
        }
        Some(msg_args::Cmd::Copy(seq, mbox_dst)) => {
            return msg_handlers::copy(seq, mbox, mbox_dst, &mut printer, backend);
        }
        Some(msg_args::Cmd::Delete(seq)) => {
            return msg_handlers::delete(seq, mbox, &mut printer, backend);
        }
        Some(msg_args::Cmd::Forward(seq, attachment_paths, encrypt)) => {
            return msg_handlers::forward(
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
        Some(msg_args::Cmd::List(max_width, page_size, page)) => {
            return msg_handlers::list(
                max_width,
                page_size,
                page,
                mbox,
                &account_config,
                &mut printer,
                backend,
            );
        }
        Some(msg_args::Cmd::Move(seq, mbox_dst)) => {
            return msg_handlers::move_(seq, mbox, mbox_dst, &mut printer, backend);
        }
        Some(msg_args::Cmd::Read(seq, text_mime, raw, headers)) => {
            return msg_handlers::read(
                seq,
                text_mime,
                raw,
                headers,
                mbox,
                &account_config,
                &mut printer,
                backend,
            );
        }
        Some(msg_args::Cmd::Reply(seq, all, attachment_paths, encrypt)) => {
            return msg_handlers::reply(
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
        Some(msg_args::Cmd::Save(raw_msg)) => {
            return msg_handlers::save(mbox, raw_msg, &mut printer, backend);
        }
        Some(msg_args::Cmd::Search(query, max_width, page_size, page)) => {
            return msg_handlers::search(
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
        Some(msg_args::Cmd::Sort(criteria, query, max_width, page_size, page)) => {
            return msg_handlers::sort(
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
        Some(msg_args::Cmd::Send(raw_msg)) => {
            return msg_handlers::send(raw_msg, &account_config, &mut printer, backend, &mut smtp);
        }
        Some(msg_args::Cmd::Write(tpl, atts, encrypt)) => {
            return msg_handlers::write(
                tpl,
                atts,
                encrypt,
                &account_config,
                &mut printer,
                backend,
                &mut smtp,
            );
        }
        Some(msg_args::Cmd::Flag(m)) => match m {
            Some(flag_args::Cmd::Set(seq_range, ref flags)) => {
                return flag_handlers::set(seq_range, flags, mbox, &mut printer, backend);
            }
            Some(flag_args::Cmd::Add(seq_range, ref flags)) => {
                return flag_handlers::add(seq_range, flags, mbox, &mut printer, backend);
            }
            Some(flag_args::Cmd::Remove(seq_range, ref flags)) => {
                return flag_handlers::remove(seq_range, flags, mbox, &mut printer, backend);
            }
            _ => (),
        },
        Some(msg_args::Cmd::Tpl(m)) => match m {
            Some(tpl_args::Cmd::New(tpl)) => {
                return tpl_handlers::new(tpl, &account_config, &mut printer);
            }
            Some(tpl_args::Cmd::Reply(seq, all, tpl)) => {
                return tpl_handlers::reply(
                    seq,
                    all,
                    tpl,
                    mbox,
                    &account_config,
                    &mut printer,
                    backend,
                );
            }
            Some(tpl_args::Cmd::Forward(seq, tpl)) => {
                return tpl_handlers::forward(
                    seq,
                    tpl,
                    mbox,
                    &account_config,
                    &mut printer,
                    backend,
                );
            }
            Some(tpl_args::Cmd::Save(atts, tpl)) => {
                return tpl_handlers::save(mbox, &account_config, atts, tpl, &mut printer, backend);
            }
            Some(tpl_args::Cmd::Send(atts, tpl)) => {
                return tpl_handlers::send(
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

    backend.disconnect().context("cannot disconnect")
}
