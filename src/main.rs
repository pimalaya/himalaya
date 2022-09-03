use anyhow::Result;
use clap::Command;
use std::env;
use url::Url;

use himalaya::{
    account, compl,
    config::{self, DeserializedConfig},
    email, flag, folder, man, output,
    printer::StdoutPrinter,
    tpl,
};
use himalaya_lib::{BackendBuilder, BackendConfig, ImapBackend, SenderBuilder};

#[cfg(feature = "imap-backend")]
use himalaya::imap;

fn create_app() -> Command {
    let app = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .propagate_version(true)
        .arg(&config::args::arg())
        .arg(&account::args::arg())
        .args(&output::args::args())
        .arg(folder::args::source_arg())
        .subcommands(compl::args::subcmds())
        .subcommands(man::args::subcmds())
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

    // checks mailto command before app initialization
    let raw_args: Vec<String> = env::args().collect();
    if raw_args.len() > 1 && raw_args[1].starts_with("mailto:") {
        let url = Url::parse(&raw_args[1])?;
        let config = DeserializedConfig::from_opt_path(None)?;
        let (account_config, backend_config) = config.to_configs(None)?;
        let mut backend = BackendBuilder::build(&account_config, &backend_config)?;
        let mut sender = SenderBuilder::build(&account_config)?;
        let mut printer = StdoutPrinter::default();

        return email::handlers::mailto(
            &account_config,
            &mut printer,
            backend.as_mut(),
            sender.as_mut(),
            &url,
        );
    }

    let app = create_app();
    let m = app.get_matches();

    // checks completion command before configs
    // https://github.com/soywod/himalaya/issues/115
    match compl::args::matches(&m)? {
        Some(compl::args::Cmd::Generate(shell)) => {
            return compl::handlers::generate(create_app(), shell);
        }
        _ => (),
    }

    // checks completion command before configs
    // https://github.com/soywod/himalaya/issues/115
    match man::args::matches(&m)? {
        Some(man::args::Cmd::Generate) => {
            return man::handlers::generate(create_app());
        }
        _ => (),
    }

    // inits config
    let config = DeserializedConfig::from_opt_path(config::args::parse_arg(&m))?;
    let (account_config, backend_config) = config.to_configs(account::args::parse_arg(&m))?;
    let folder = account_config.folder_alias(folder::args::parse_source_arg(&m))?;

    // checks IMAP commands
    #[cfg(feature = "imap-backend")]
    if let BackendConfig::Imap(imap_config) = backend_config {
        // FIXME: find a way to downcast `backend` instead of
        // recreating an instance.
        let mut imap = ImapBackend::new(imap_config)?;
        match imap::args::matches(&m)? {
            Some(imap::args::Cmd::Notify(keepalive)) => {
                return imap::handlers::notify(&mut imap, &folder, keepalive);
            }
            Some(imap::args::Cmd::Watch(keepalive)) => {
                return imap::handlers::watch(&mut imap, &folder, keepalive);
            }
            _ => (),
        }
    }

    // inits services
    let mut backend = BackendBuilder::build(&account_config, &backend_config)?;
    let mut sender = SenderBuilder::build(&account_config)?;
    let mut printer = StdoutPrinter::try_from(&m)?;

    // checks account commands
    match account::args::matches(&m)? {
        Some(account::args::Cmd::List(max_width)) => {
            return account::handlers::list(max_width, &account_config, &config, &mut printer);
        }
        _ => (),
    }

    // checks folder commands
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

    // checks email commands
    match email::args::matches(&m)? {
        Some(email::args::Cmd::Attachments(id)) => {
            return email::handlers::attachments(
                &account_config,
                &mut printer,
                backend.as_mut(),
                &folder,
                id,
            );
        }
        Some(email::args::Cmd::Copy(ids, folder_target)) => {
            return email::handlers::copy(
                &account_config,
                &mut printer,
                backend.as_mut(),
                &folder,
                folder_target,
                ids,
            );
        }
        Some(email::args::Cmd::Delete(ids)) => {
            return email::handlers::delete(
                &account_config,
                &mut printer,
                backend.as_mut(),
                &folder,
                ids,
            );
        }
        Some(email::args::Cmd::Forward(id, headers, body)) => {
            return email::handlers::forward(
                &account_config,
                &mut printer,
                backend.as_mut(),
                sender.as_mut(),
                &folder,
                id,
                headers,
                body,
            );
        }
        Some(email::args::Cmd::List(max_width, page_size, page)) => {
            return email::handlers::list(
                &account_config,
                &mut printer,
                backend.as_mut(),
                &folder,
                max_width,
                page_size,
                page,
            );
        }
        Some(email::args::Cmd::Move(ids, folder_target)) => {
            return email::handlers::move_(
                &account_config,
                &mut printer,
                backend.as_mut(),
                &folder,
                folder_target,
                ids,
            );
        }
        Some(email::args::Cmd::Read(id, text_mime, sanitize, raw, headers)) => {
            return email::handlers::read(
                &account_config,
                &mut printer,
                backend.as_mut(),
                &folder,
                id,
                text_mime,
                sanitize,
                raw,
                headers,
            );
        }
        Some(email::args::Cmd::Reply(id, all, headers, body)) => {
            return email::handlers::reply(
                &account_config,
                &mut printer,
                backend.as_mut(),
                sender.as_mut(),
                &folder,
                id,
                all,
                headers,
                body,
            );
        }
        Some(email::args::Cmd::Save(raw_email)) => {
            return email::handlers::save(
                &account_config,
                &mut printer,
                backend.as_mut(),
                &folder,
                raw_email,
            );
        }
        Some(email::args::Cmd::Search(query, max_width, page_size, page)) => {
            return email::handlers::search(
                &account_config,
                &mut printer,
                backend.as_mut(),
                &folder,
                query,
                max_width,
                page_size,
                page,
            );
        }
        Some(email::args::Cmd::Sort(criteria, query, max_width, page_size, page)) => {
            return email::handlers::sort(
                &account_config,
                &mut printer,
                backend.as_mut(),
                &folder,
                criteria,
                query,
                max_width,
                page_size,
                page,
            );
        }
        Some(email::args::Cmd::Send(raw_email)) => {
            return email::handlers::send(
                &account_config,
                &mut printer,
                backend.as_mut(),
                sender.as_mut(),
                raw_email,
            );
        }
        Some(email::args::Cmd::Flag(m)) => match m {
            Some(flag::args::Cmd::Set(id, ref flags)) => {
                return flag::handlers::set(&mut printer, backend.as_mut(), &folder, id, flags);
            }
            Some(flag::args::Cmd::Add(id, ref flags)) => {
                return flag::handlers::add(&mut printer, backend.as_mut(), &folder, id, flags);
            }
            Some(flag::args::Cmd::Remove(id, ref flags)) => {
                return flag::handlers::remove(&mut printer, backend.as_mut(), &folder, id, flags);
            }
            _ => (),
        },
        Some(email::args::Cmd::Tpl(m)) => match m {
            Some(tpl::args::Cmd::Forward(id, headers, body)) => {
                return tpl::handlers::forward(
                    &account_config,
                    &mut printer,
                    backend.as_mut(),
                    &folder,
                    id,
                    headers,
                    body,
                );
            }
            Some(tpl::args::Cmd::Write(headers, body)) => {
                return tpl::handlers::write(&account_config, &mut printer, headers, body);
            }
            Some(tpl::args::Cmd::Reply(id, all, headers, body)) => {
                return tpl::handlers::reply(
                    &account_config,
                    &mut printer,
                    backend.as_mut(),
                    &folder,
                    id,
                    all,
                    headers,
                    body,
                );
            }
            Some(tpl::args::Cmd::Save(tpl)) => {
                return tpl::handlers::save(
                    &account_config,
                    &mut printer,
                    backend.as_mut(),
                    &folder,
                    tpl,
                );
            }
            Some(tpl::args::Cmd::Send(tpl)) => {
                return tpl::handlers::send(
                    &account_config,
                    &mut printer,
                    backend.as_mut(),
                    sender.as_mut(),
                    &folder,
                    tpl,
                );
            }
            _ => (),
        },
        Some(email::args::Cmd::Write(headers, body)) => {
            return email::handlers::write(
                &account_config,
                &mut printer,
                backend.as_mut(),
                sender.as_mut(),
                headers,
                body,
            );
        }
        _ => (),
    }

    Ok(())
}
