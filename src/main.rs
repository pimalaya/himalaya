use anyhow::{anyhow, Context, Result};
use clap::Command;
use pimalaya_email::{
    BackendBuilder, BackendConfig, ImapBackend, SenderBuilder, DEFAULT_INBOX_FOLDER,
};
use std::env;
use url::Url;

use himalaya::{
    account, cache, compl,
    config::{self, DeserializedConfig},
    email, flag, folder, man, output,
    printer::StdoutPrinter,
    tpl, IdMapper,
};

#[cfg(feature = "imap-backend")]
use himalaya::imap;

fn create_app() -> Command {
    let app = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .propagate_version(true)
        .infer_subcommands(true)
        .arg(config::args::arg())
        .arg(account::args::arg())
        .arg(cache::args::arg())
        .args(output::args::args())
        .arg(folder::args::source_arg())
        .subcommand(compl::args::subcmd())
        .subcommand(man::args::subcmd())
        .subcommand(account::args::subcmd())
        .subcommand(folder::args::subcmd())
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
        let account_config = config.to_account_config(None)?;
        let mut backend = BackendBuilder::new().build(&account_config)?;
        let mut sender = SenderBuilder::new().build(&account_config)?;
        let mut printer = StdoutPrinter::default();

        return email::handlers::mailto(
            &account_config,
            backend.as_mut(),
            sender.as_mut(),
            &mut printer,
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

    // also checks man command before configs
    match man::args::matches(&m)? {
        Some(man::args::Cmd::GenerateAll(dir)) => {
            return man::handlers::generate(dir, create_app());
        }
        _ => (),
    }

    // inits config
    let config = DeserializedConfig::from_opt_path(config::args::parse_arg(&m))?;
    let account_config = config.to_account_config(account::args::parse_arg(&m))?;
    let folder = folder::args::parse_source_arg(&m);

    // checks IMAP commands
    #[cfg(feature = "imap-backend")]
    if let BackendConfig::Imap(imap_config) = &account_config.backend {
        let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;

        // FIXME: find a way to downcast `backend` instead of
        // recreating an instance.
        match imap::args::matches(&m)? {
            Some(imap::args::Cmd::Notify(keepalive)) => {
                let imap = ImapBackend::new(account_config.clone(), imap_config.clone())?;
                return imap::handlers::notify(&imap, &folder, keepalive);
            }
            Some(imap::args::Cmd::Watch(keepalive)) => {
                let imap = ImapBackend::new(account_config.clone(), imap_config.clone())?;
                return imap::handlers::watch(&imap, &folder, keepalive);
            }
            _ => (),
        }
    }

    // inits services
    let disable_cache = cache::args::parse_disable_cache_flag(&m);
    let mut printer = StdoutPrinter::try_from(&m)?;

    // checks account commands
    match account::args::matches(&m)? {
        Some(account::args::Cmd::List(max_width)) => {
            return account::handlers::list(max_width, &account_config, &config, &mut printer);
        }
        Some(account::args::Cmd::Sync(folders_strategy, dry_run)) => {
            let backend = BackendBuilder::new()
                .sessions_pool_size(8)
                .disable_cache(true)
                .build(&account_config)?;
            account::handlers::sync(
                &account_config,
                &mut printer,
                backend.as_ref(),
                folders_strategy,
                dry_run,
            )?;
            backend.close()?;
            return Ok(());
        }
        Some(account::args::Cmd::Configure(reset)) => {
            return account::handlers::configure(&account_config, reset);
        }
        _ => (),
    }

    // checks folder commands
    match folder::args::matches(&m)? {
        Some(folder::args::Cmd::Create) => {
            let folder = folder
                .ok_or_else(|| anyhow!("the folder argument is missing"))
                .context("cannot create folder")?;
            let folder = account_config.folder_alias(folder)?;
            let mut backend = BackendBuilder::new()
                .disable_cache(disable_cache)
                .build(&account_config)?;
            return folder::handlers::create(&mut printer, backend.as_mut(), &folder);
        }
        Some(folder::args::Cmd::List(max_width)) => {
            let mut backend = BackendBuilder::new()
                .disable_cache(disable_cache)
                .build(&account_config)?;
            return folder::handlers::list(
                &account_config,
                &mut printer,
                backend.as_mut(),
                max_width,
            );
        }
        Some(folder::args::Cmd::Expunge) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = BackendBuilder::new()
                .disable_cache(disable_cache)
                .build(&account_config)?;
            return folder::handlers::expunge(&mut printer, backend.as_mut(), &folder);
        }
        Some(folder::args::Cmd::Delete) => {
            let folder = folder
                .ok_or_else(|| anyhow!("the folder argument is missing"))
                .context("cannot delete folder")?;
            let folder = account_config.folder_alias(folder)?;
            let mut backend = BackendBuilder::new()
                .disable_cache(disable_cache)
                .build(&account_config)?;
            return folder::handlers::delete(&mut printer, backend.as_mut(), &folder);
        }
        _ => (),
    }

    // checks email commands
    match email::args::matches(&m)? {
        Some(email::args::Cmd::Attachments(ids)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = BackendBuilder::new()
                .disable_cache(disable_cache)
                .build(&account_config)?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_config.name, &folder)?;
            return email::handlers::attachments(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                &folder,
                ids,
            );
        }
        Some(email::args::Cmd::Copy(ids, to_folder)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = BackendBuilder::new()
                .disable_cache(disable_cache)
                .build(&account_config)?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_config.name, &folder)?;
            return email::handlers::copy(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                &folder,
                to_folder,
                ids,
            );
        }
        Some(email::args::Cmd::Delete(ids)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = BackendBuilder::new()
                .disable_cache(disable_cache)
                .build(&account_config)?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_config.name, &folder)?;
            return email::handlers::delete(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                &folder,
                ids,
            );
        }
        Some(email::args::Cmd::Forward(id, headers, body)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = BackendBuilder::new()
                .disable_cache(disable_cache)
                .build(&account_config)?;
            let mut sender = SenderBuilder::new().build(&account_config)?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_config.name, &folder)?;
            return email::handlers::forward(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                sender.as_mut(),
                &folder,
                id,
                headers,
                body,
            );
        }
        Some(email::args::Cmd::List(max_width, page_size, page)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = BackendBuilder::new()
                .disable_cache(disable_cache)
                .build(&account_config)?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_config.name, &folder)?;
            return email::handlers::list(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                &folder,
                max_width,
                page_size,
                page,
            );
        }
        Some(email::args::Cmd::Move(ids, to_folder)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = BackendBuilder::new()
                .disable_cache(disable_cache)
                .build(&account_config)?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_config.name, &folder)?;
            return email::handlers::move_(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                &folder,
                to_folder,
                ids,
            );
        }
        Some(email::args::Cmd::Read(ids, text_mime, raw, headers)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = BackendBuilder::new()
                .disable_cache(disable_cache)
                .build(&account_config)?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_config.name, &folder)?;
            return email::handlers::read(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                &folder,
                ids,
                text_mime,
                raw,
                headers,
            );
        }
        Some(email::args::Cmd::Reply(id, all, headers, body)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = BackendBuilder::new()
                .disable_cache(disable_cache)
                .build(&account_config)?;
            let mut sender = SenderBuilder::new().build(&account_config)?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_config.name, &folder)?;
            return email::handlers::reply(
                &account_config,
                &mut printer,
                &id_mapper,
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
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = BackendBuilder::new()
                .disable_cache(disable_cache)
                .build(&account_config)?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_config.name, &folder)?;
            return email::handlers::save(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                &folder,
                raw_email,
            );
        }
        Some(email::args::Cmd::Search(query, max_width, page_size, page)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = BackendBuilder::new()
                .disable_cache(disable_cache)
                .build(&account_config)?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_config.name, &folder)?;
            return email::handlers::search(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                &folder,
                query,
                max_width,
                page_size,
                page,
            );
        }
        Some(email::args::Cmd::Sort(criteria, query, max_width, page_size, page)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = BackendBuilder::new()
                .disable_cache(disable_cache)
                .build(&account_config)?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_config.name, &folder)?;
            return email::handlers::sort(
                &account_config,
                &mut printer,
                &id_mapper,
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
            let mut backend = BackendBuilder::new()
                .disable_cache(disable_cache)
                .build(&account_config)?;
            let mut sender = SenderBuilder::new().build(&account_config)?;
            return email::handlers::send(
                &account_config,
                &mut printer,
                backend.as_mut(),
                sender.as_mut(),
                raw_email,
            );
        }
        Some(email::args::Cmd::Flag(m)) => match m {
            Some(flag::args::Cmd::Set(ids, ref flags)) => {
                let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
                let mut backend = BackendBuilder::new()
                    .disable_cache(disable_cache)
                    .build(&account_config)?;
                let id_mapper = IdMapper::new(backend.as_ref(), &account_config.name, &folder)?;
                return flag::handlers::set(
                    &mut printer,
                    &id_mapper,
                    backend.as_mut(),
                    &folder,
                    ids,
                    flags,
                );
            }
            Some(flag::args::Cmd::Add(ids, ref flags)) => {
                let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
                let mut backend = BackendBuilder::new()
                    .disable_cache(disable_cache)
                    .build(&account_config)?;
                let id_mapper = IdMapper::new(backend.as_ref(), &account_config.name, &folder)?;
                return flag::handlers::add(
                    &mut printer,
                    &id_mapper,
                    backend.as_mut(),
                    &folder,
                    ids,
                    flags,
                );
            }
            Some(flag::args::Cmd::Remove(ids, ref flags)) => {
                let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
                let mut backend = BackendBuilder::new()
                    .disable_cache(disable_cache)
                    .build(&account_config)?;
                let id_mapper = IdMapper::new(backend.as_ref(), &account_config.name, &folder)?;
                return flag::handlers::remove(
                    &mut printer,
                    &id_mapper,
                    backend.as_mut(),
                    &folder,
                    ids,
                    flags,
                );
            }
            _ => (),
        },
        Some(email::args::Cmd::Tpl(m)) => match m {
            Some(tpl::args::Cmd::Forward(id, headers, body)) => {
                let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
                let mut backend = BackendBuilder::new()
                    .disable_cache(disable_cache)
                    .build(&account_config)?;
                let id_mapper = IdMapper::new(backend.as_ref(), &account_config.name, &folder)?;
                return tpl::handlers::forward(
                    &account_config,
                    &mut printer,
                    &id_mapper,
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
                let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
                let mut backend = BackendBuilder::new()
                    .disable_cache(disable_cache)
                    .build(&account_config)?;
                let id_mapper = IdMapper::new(backend.as_ref(), &account_config.name, &folder)?;
                return tpl::handlers::reply(
                    &account_config,
                    &mut printer,
                    &id_mapper,
                    backend.as_mut(),
                    &folder,
                    id,
                    all,
                    headers,
                    body,
                );
            }
            Some(tpl::args::Cmd::Save(tpl)) => {
                let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
                let mut backend = BackendBuilder::new()
                    .disable_cache(disable_cache)
                    .build(&account_config)?;
                let id_mapper = IdMapper::new(backend.as_ref(), &account_config.name, &folder)?;
                return tpl::handlers::save(
                    &account_config,
                    &mut printer,
                    &id_mapper,
                    backend.as_mut(),
                    &folder,
                    tpl,
                );
            }
            Some(tpl::args::Cmd::Send(tpl)) => {
                let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
                let mut backend = BackendBuilder::new()
                    .disable_cache(disable_cache)
                    .build(&account_config)?;
                let mut sender = SenderBuilder::new().build(&account_config)?;
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
            let mut backend = BackendBuilder::new()
                .disable_cache(disable_cache)
                .build(&account_config)?;
            let mut sender = SenderBuilder::new().build(&account_config)?;
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
