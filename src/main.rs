use anyhow::{Context, Result};
use std::env;
use url::Url;

use himalaya::{
    account, compl,
    config::{self, DeserializedConfig},
    email, flag, folder,
    output::{self, OutputFmt},
    printer::StdoutPrinter,
    tpl,
};
use himalaya_lib::{BackendBuilder, BackendConfig, ImapBackend, SenderBuilder};

#[cfg(feature = "imap-backend")]
use himalaya::imap;

fn create_app<'a>() -> clap::App<'a, 'a> {
    let app = clap::App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .global_setting(clap::AppSettings::GlobalVersion)
        .arg(&config::args::path_arg())
        .arg(&account::args::name_arg())
        .args(&output::args::args())
        .arg(folder::args::source_arg())
        .subcommands(compl::args::subcmds())
        .subcommands(account::args::subcmds())
        .subcommands(folder::args::subcmds())
        .subcommands(email::args::subcmds());

    #[cfg(feature = "imap-backend")]
    let app = app.subcommands(imap::args::subcmds());

    app
}

#[allow(clippy::single_match)]
fn main() -> Result<()> {
    let default_env_filter = env_logger::DEFAULT_FILTER_ENV;
    env_logger::init_from_env(env_logger::Env::default().filter_or(default_env_filter, "off"));

    // Check mailto command BEFORE app initialization.
    let raw_args: Vec<String> = env::args().collect();
    if raw_args.len() > 1 && raw_args[1].starts_with("mailto:") {
        let url = Url::parse(&raw_args[1])?;
        let config = DeserializedConfig::from_opt_path(None)?;
        let (account_config, backend_config) = config.to_configs(None)?;
        let mut backend = BackendBuilder::build(&account_config, &backend_config)?;
        let mut sender = SenderBuilder::build(&account_config)?;
        let mut printer = StdoutPrinter::from_fmt(OutputFmt::Plain);

        return email::handlers::mailto(
            &url,
            &account_config,
            &mut printer,
            backend.as_mut(),
            sender.as_mut(),
        );
    }

    let app = create_app();
    let m = app.get_matches();

    // Check completion command BEFORE entities and services initialization.
    // Related issue: https://github.com/soywod/himalaya/issues/115.
    match compl::args::matches(&m)? {
        Some(compl::args::Command::Generate(shell)) => {
            return compl::handlers::generate(create_app(), shell);
        }
        _ => (),
    }

    // Init entities and services.
    let config = DeserializedConfig::from_opt_path(m.value_of("config"))?;
    let (account_config, backend_config) = config.to_configs(m.value_of("account"))?;
    let default_folder = account_config.folder_alias("inbox")?;
    let folder = m.value_of("folder-source").unwrap_or(&default_folder);

    // Check IMAP commands.
    #[cfg(feature = "imap-backend")]
    if let BackendConfig::Imap(imap_config) = backend_config {
        // FIXME: find a way to downcast `backend` instead.
        let mut imap = ImapBackend::new(&account_config, imap_config);
        match imap::args::matches(&m)? {
            Some(imap::args::Command::Notify(keepalive)) => {
                return imap::handlers::notify(keepalive, folder, &mut imap);
            }
            Some(imap::args::Command::Watch(keepalive)) => {
                return imap::handlers::watch(keepalive, folder, &mut imap);
            }
            _ => (),
        }
    }

    let mut backend = BackendBuilder::build(&account_config, &backend_config)?;
    let mut sender = SenderBuilder::build(&account_config)?;
    let mut printer = StdoutPrinter::from_opt_str(m.value_of("output"))?;

    // Check account commands.
    match account::args::matches(&m)? {
        Some(account::args::Cmd::List(max_width)) => {
            return account::handlers::list(max_width, &account_config, &config, &mut printer);
        }
        _ => (),
    }

    // Check mailbox commands.
    match folder::args::matches(&m)? {
        Some(folder::args::Cmd::List(max_width)) => {
            return folder::handlers::list(
                max_width,
                &account_config,
                &mut printer,
                backend.as_mut(),
            );
        }
        _ => (),
    }

    // Check message commands.
    match email::args::matches(&m)? {
        Some(email::args::Cmd::Attachments(seq)) => {
            return email::handlers::attachments(
                seq,
                folder,
                &account_config,
                &mut printer,
                backend.as_mut(),
            );
        }
        Some(email::args::Cmd::Copy(seq, mbox_dst)) => {
            return email::handlers::copy(seq, folder, mbox_dst, &mut printer, backend.as_mut());
        }
        Some(email::args::Cmd::Delete(seq)) => {
            return email::handlers::delete(seq, folder, &mut printer, backend.as_mut());
        }
        Some(email::args::Cmd::Forward(seq, attachment_paths, encrypt)) => {
            return email::handlers::forward(
                seq,
                attachment_paths,
                encrypt,
                folder,
                &account_config,
                &mut printer,
                backend.as_mut(),
                sender.as_mut(),
            );
        }
        Some(email::args::Cmd::List(max_width, page_size, page)) => {
            return email::handlers::list(
                max_width,
                page_size,
                page,
                folder,
                &account_config,
                &mut printer,
                backend.as_mut(),
            );
        }
        Some(email::args::Cmd::Move(seq, mbox_dst)) => {
            return email::handlers::move_(seq, folder, mbox_dst, &mut printer, backend.as_mut());
        }
        Some(email::args::Cmd::Read(seq, text_mime, raw, headers)) => {
            return email::handlers::read(
                seq,
                text_mime,
                raw,
                headers,
                folder,
                &account_config,
                &mut printer,
                backend.as_mut(),
            );
        }
        Some(email::args::Cmd::Reply(seq, all, attachment_paths, encrypt)) => {
            return email::handlers::reply(
                seq,
                all,
                attachment_paths,
                encrypt,
                folder,
                &account_config,
                &mut printer,
                backend.as_mut(),
                sender.as_mut(),
            );
        }
        Some(email::args::Cmd::Save(raw_msg)) => {
            return email::handlers::save(folder, raw_msg, &mut printer, backend.as_mut());
        }
        Some(email::args::Cmd::Search(query, max_width, page_size, page)) => {
            return email::handlers::search(
                query,
                max_width,
                page_size,
                page,
                folder,
                &account_config,
                &mut printer,
                backend.as_mut(),
            );
        }
        Some(email::args::Cmd::Sort(criteria, query, max_width, page_size, page)) => {
            return email::handlers::sort(
                criteria,
                query,
                max_width,
                page_size,
                page,
                folder,
                &account_config,
                &mut printer,
                backend.as_mut(),
            );
        }
        Some(email::args::Cmd::Send(raw_msg)) => {
            return email::handlers::send(
                raw_msg,
                &account_config,
                &mut printer,
                backend.as_mut(),
                sender.as_mut(),
            );
        }
        Some(email::args::Cmd::Write(tpl, atts, encrypt)) => {
            return email::handlers::write(
                tpl,
                atts,
                encrypt,
                &account_config,
                &mut printer,
                backend.as_mut(),
                sender.as_mut(),
            );
        }
        Some(email::args::Cmd::Flag(m)) => match m {
            Some(flag::args::Cmd::Set(seq_range, ref flags)) => {
                return flag::handlers::set(
                    seq_range,
                    flags,
                    folder,
                    &mut printer,
                    backend.as_mut(),
                );
            }
            Some(flag::args::Cmd::Add(seq_range, ref flags)) => {
                return flag::handlers::add(
                    seq_range,
                    flags,
                    folder,
                    &mut printer,
                    backend.as_mut(),
                );
            }
            Some(flag::args::Cmd::Remove(seq_range, ref flags)) => {
                return flag::handlers::remove(
                    seq_range,
                    flags,
                    folder,
                    &mut printer,
                    backend.as_mut(),
                );
            }
            _ => (),
        },
        Some(email::args::Cmd::Tpl(m)) => match m {
            Some(tpl::args::Cmd::New(tpl)) => {
                return tpl::handlers::new(tpl, &account_config, &mut printer);
            }
            Some(tpl::args::Cmd::Reply(seq, all, tpl)) => {
                return tpl::handlers::reply(
                    seq,
                    all,
                    tpl,
                    folder,
                    &account_config,
                    &mut printer,
                    backend.as_mut(),
                );
            }
            Some(tpl::args::Cmd::Forward(seq, tpl)) => {
                return tpl::handlers::forward(
                    seq,
                    tpl,
                    folder,
                    &account_config,
                    &mut printer,
                    backend.as_mut(),
                );
            }
            Some(tpl::args::Cmd::Save(atts, tpl)) => {
                return tpl::handlers::save(
                    folder,
                    &account_config,
                    atts,
                    tpl,
                    &mut printer,
                    backend.as_mut(),
                );
            }
            Some(tpl::args::Cmd::Send(atts, tpl)) => {
                return tpl::handlers::send(
                    folder,
                    &account_config,
                    atts,
                    tpl,
                    &mut printer,
                    backend.as_mut(),
                    sender.as_mut(),
                );
            }
            _ => (),
        },
        _ => (),
    }

    backend.as_mut().disconnect().context("cannot disconnect")
}
