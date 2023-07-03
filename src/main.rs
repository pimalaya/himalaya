use anyhow::{anyhow, Context, Result};
use clap::Command;
#[cfg(feature = "imap-backend")]
use pimalaya_email::backend::ImapBackend;
use pimalaya_email::{
    account::{sync::AccountSyncBuilder, DEFAULT_INBOX_FOLDER},
    backend::{BackendBuilder, BackendConfig},
    sender::SenderBuilder,
};
use std::env;
use url::Url;

#[cfg(feature = "imap-backend")]
use himalaya::imap;
use himalaya::{
    account, cache, compl,
    config::{self, DeserializedConfig},
    email, flag, folder, man, output,
    printer::StdoutPrinter,
    tpl, IdMapper,
};

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
#[tokio::main]
async fn main() -> Result<()> {
    let default_env_filter = env_logger::DEFAULT_FILTER_ENV;
    env_logger::init_from_env(env_logger::Env::default().filter_or(default_env_filter, "off"));

    // checks mailto command before app initialization
    let raw_args: Vec<String> = env::args().collect();
    if raw_args.len() > 1 && raw_args[1].starts_with("mailto:") {
        let url = Url::parse(&raw_args[1])?;
        let config = DeserializedConfig::from_opt_path(None)?;
        let account_config = config.to_account_config(None)?;
        let mut backend = BackendBuilder::new(account_config.clone()).build().await?;
        let mut sender = SenderBuilder::new(account_config.clone()).build().await?;
        let mut printer = StdoutPrinter::default();

        email::handlers::mailto(
            &account_config,
            backend.as_mut(),
            sender.as_mut(),
            &mut printer,
            &url,
        )
        .await?;

        return Ok(());
    }

    let app = create_app();
    let m = app.get_matches();

    // check completion command before configs
    // https://github.com/soywod/himalaya/issues/115
    match compl::args::matches(&m)? {
        Some(compl::args::Cmd::Generate(shell)) => {
            return compl::handlers::generate(create_app(), shell);
        }
        _ => (),
    }

    // check also man command before configs
    match man::args::matches(&m)? {
        Some(man::args::Cmd::GenerateAll(dir)) => {
            return man::handlers::generate(dir, create_app());
        }
        _ => (),
    }

    let config = DeserializedConfig::from_opt_path(config::args::parse_arg(&m))?;
    let account_config = config.to_account_config(account::args::parse_arg(&m))?;
    let account_name = account_config.name.clone();
    let folder = folder::args::parse_source_arg(&m);
    let disable_cache = cache::args::parse_disable_cache_flag(&m);

    // FIXME: find why account config cannot be borrowed
    // let backend_builder =
    //     BackendBuilder::new(Cow::Borrowed(&account_config)).with_cache_disabled(disable_cache);
    let backend_builder =
        BackendBuilder::new(account_config.clone()).with_cache_disabled(disable_cache);
    let sender_builder = SenderBuilder::new(account_config.clone());
    let mut printer = StdoutPrinter::try_from(&m)?;

    #[cfg(feature = "imap-backend")]
    if let BackendConfig::Imap(imap_config) = &account_config.backend {
        let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
        match imap::args::matches(&m)? {
            Some(imap::args::Cmd::Notify(keepalive)) => {
                let mut backend =
                    ImapBackend::new(account_config.clone(), imap_config.clone(), None).await?;
                imap::handlers::notify(&mut backend, &folder, keepalive).await?;
                return Ok(());
            }
            Some(imap::args::Cmd::Watch(keepalive)) => {
                let mut backend =
                    ImapBackend::new(account_config.clone(), imap_config.clone(), None).await?;
                imap::handlers::watch(&mut backend, &folder, keepalive).await?;
                return Ok(());
            }
            _ => (),
        }
    }

    match account::args::matches(&m)? {
        Some(account::args::Cmd::List(max_width)) => {
            account::handlers::list(max_width, &account_config, &config, &mut printer)?;
            return Ok(());
        }
        Some(account::args::Cmd::Sync(strategy, dry_run)) => {
            let sync_builder = AccountSyncBuilder::new(account_config, backend_builder)
                .await?
                .with_some_folders_strategy(strategy)
                .with_dry_run(dry_run);
            account::handlers::sync(&mut printer, sync_builder, dry_run).await?;
            return Ok(());
        }
        Some(account::args::Cmd::Configure(reset)) => {
            account::handlers::configure(&account_config, reset).await?;
            return Ok(());
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
            let mut backend = backend_builder.build().await?;
            folder::handlers::create(&mut printer, backend.as_mut(), &folder).await?;
            return Ok(());
        }
        Some(folder::args::Cmd::List(max_width)) => {
            let mut backend = backend_builder.build().await?;
            folder::handlers::list(&account_config, &mut printer, backend.as_mut(), max_width)
                .await?;
            return Ok(());
        }
        Some(folder::args::Cmd::Expunge) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = backend_builder.build().await?;
            folder::handlers::expunge(&mut printer, backend.as_mut(), &folder).await?;
            return Ok(());
        }
        Some(folder::args::Cmd::Delete) => {
            let folder = folder
                .ok_or_else(|| anyhow!("the folder argument is missing"))
                .context("cannot delete folder")?;
            let folder = account_config.folder_alias(folder)?;
            let mut backend = backend_builder.build().await?;
            folder::handlers::delete(&mut printer, backend.as_mut(), &folder).await?;
            return Ok(());
        }
        _ => (),
    }

    // checks email commands
    match email::args::matches(&m)? {
        Some(email::args::Cmd::Attachments(ids)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = backend_builder.clone().into_build().await?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_name, &folder)?;
            email::handlers::attachments(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                &folder,
                ids,
            )
            .await?;
            return Ok(());
        }
        Some(email::args::Cmd::Copy(ids, to_folder)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = backend_builder.clone().into_build().await?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_name, &folder)?;

            email::handlers::copy(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                &folder,
                to_folder,
                ids,
            )
            .await?;

            return Ok(());
        }
        Some(email::args::Cmd::Delete(ids)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = backend_builder.clone().into_build().await?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_name, &folder)?;

            email::handlers::delete(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                &folder,
                ids,
            )
            .await?;

            return Ok(());
        }
        Some(email::args::Cmd::Forward(id, headers, body)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = backend_builder.clone().into_build().await?;
            let mut sender = sender_builder.build().await?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_name, &folder)?;

            email::handlers::forward(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                sender.as_mut(),
                &folder,
                id,
                headers,
                body,
            )
            .await?;

            return Ok(());
        }
        Some(email::args::Cmd::List(max_width, page_size, page)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = backend_builder.clone().into_build().await?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_name, &folder)?;

            email::handlers::list(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                &folder,
                max_width,
                page_size,
                page,
            )
            .await?;

            return Ok(());
        }
        Some(email::args::Cmd::Move(ids, to_folder)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = backend_builder.clone().into_build().await?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_name, &folder)?;

            email::handlers::move_(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                &folder,
                to_folder,
                ids,
            )
            .await?;

            return Ok(());
        }
        Some(email::args::Cmd::Read(ids, text_mime, raw, headers)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = backend_builder.clone().into_build().await?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_name, &folder)?;

            email::handlers::read(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                &folder,
                ids,
                text_mime,
                raw,
                headers,
            )
            .await?;

            return Ok(());
        }
        Some(email::args::Cmd::Reply(id, all, headers, body)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = backend_builder.clone().into_build().await?;
            let mut sender = sender_builder.build().await?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_name, &folder)?;

            email::handlers::reply(
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
            )
            .await?;

            return Ok(());
        }
        Some(email::args::Cmd::Save(raw_email)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = backend_builder.clone().into_build().await?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_name, &folder)?;

            email::handlers::save(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                &folder,
                raw_email,
            )
            .await?;

            return Ok(());
        }
        Some(email::args::Cmd::Search(query, max_width, page_size, page)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = backend_builder.clone().into_build().await?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_name, &folder)?;

            email::handlers::search(
                &account_config,
                &mut printer,
                &id_mapper,
                backend.as_mut(),
                &folder,
                query,
                max_width,
                page_size,
                page,
            )
            .await?;

            return Ok(());
        }
        Some(email::args::Cmd::Sort(criteria, query, max_width, page_size, page)) => {
            let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
            let mut backend = backend_builder.clone().into_build().await?;
            let id_mapper = IdMapper::new(backend.as_ref(), &account_name, &folder)?;

            email::handlers::sort(
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
            )
            .await?;

            return Ok(());
        }
        Some(email::args::Cmd::Send(raw_email)) => {
            let mut backend = backend_builder.build().await?;
            let mut sender = sender_builder.build().await?;
            email::handlers::send(
                &account_config,
                &mut printer,
                backend.as_mut(),
                sender.as_mut(),
                raw_email,
            )
            .await?;

            return Ok(());
        }
        Some(email::args::Cmd::Flag(m)) => match m {
            Some(flag::args::Cmd::Set(ids, ref flags)) => {
                let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
                let mut backend = backend_builder.clone().into_build().await?;
                let id_mapper = IdMapper::new(backend.as_ref(), &account_name, &folder)?;

                flag::handlers::set(
                    &mut printer,
                    &id_mapper,
                    backend.as_mut(),
                    &folder,
                    ids,
                    flags,
                )
                .await?;

                return Ok(());
            }
            Some(flag::args::Cmd::Add(ids, ref flags)) => {
                let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
                let mut backend = backend_builder.clone().into_build().await?;
                let id_mapper = IdMapper::new(backend.as_ref(), &account_name, &folder)?;

                flag::handlers::add(
                    &mut printer,
                    &id_mapper,
                    backend.as_mut(),
                    &folder,
                    ids,
                    flags,
                )
                .await?;

                return Ok(());
            }
            Some(flag::args::Cmd::Remove(ids, ref flags)) => {
                let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
                let mut backend = backend_builder.clone().into_build().await?;
                let id_mapper = IdMapper::new(backend.as_ref(), &account_name, &folder)?;

                flag::handlers::remove(
                    &mut printer,
                    &id_mapper,
                    backend.as_mut(),
                    &folder,
                    ids,
                    flags,
                )
                .await?;

                return Ok(());
            }
            _ => (),
        },
        Some(email::args::Cmd::Tpl(m)) => match m {
            Some(tpl::args::Cmd::Forward(id, headers, body)) => {
                let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
                let mut backend = backend_builder.clone().into_build().await?;
                let id_mapper = IdMapper::new(backend.as_ref(), &account_name, &folder)?;

                tpl::handlers::forward(
                    &account_config,
                    &mut printer,
                    &id_mapper,
                    backend.as_mut(),
                    &folder,
                    id,
                    headers,
                    body,
                )
                .await?;

                return Ok(());
            }
            Some(tpl::args::Cmd::Write(headers, body)) => {
                tpl::handlers::write(&account_config, &mut printer, headers, body).await?;
                return Ok(());
            }
            Some(tpl::args::Cmd::Reply(id, all, headers, body)) => {
                let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
                let mut backend = backend_builder.clone().into_build().await?;
                let id_mapper = IdMapper::new(backend.as_ref(), &account_name, &folder)?;

                tpl::handlers::reply(
                    &account_config,
                    &mut printer,
                    &id_mapper,
                    backend.as_mut(),
                    &folder,
                    id,
                    all,
                    headers,
                    body,
                )
                .await?;

                return Ok(());
            }
            Some(tpl::args::Cmd::Save(tpl)) => {
                let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
                let mut backend = backend_builder.clone().into_build().await?;
                let id_mapper = IdMapper::new(backend.as_ref(), &account_name, &folder)?;

                tpl::handlers::save(
                    &account_config,
                    &mut printer,
                    &id_mapper,
                    backend.as_mut(),
                    &folder,
                    tpl,
                )
                .await?;

                return Ok(());
            }
            Some(tpl::args::Cmd::Send(tpl)) => {
                let folder = account_config.folder_alias(folder.unwrap_or(DEFAULT_INBOX_FOLDER))?;
                let mut backend = backend_builder.clone().into_build().await?;
                let mut sender = sender_builder.build().await?;
                tpl::handlers::send(
                    &account_config,
                    &mut printer,
                    backend.as_mut(),
                    sender.as_mut(),
                    &folder,
                    tpl,
                )
                .await?;

                return Ok(());
            }
            _ => (),
        },
        Some(email::args::Cmd::Write(headers, body)) => {
            let mut backend = backend_builder.build().await?;
            let mut sender = sender_builder.build().await?;
            email::handlers::write(
                &account_config,
                &mut printer,
                backend.as_mut(),
                sender.as_mut(),
                headers,
                body,
            )
            .await?;

            return Ok(());
        }
        _ => (),
    }

    Ok(())
}
