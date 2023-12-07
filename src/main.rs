use anyhow::Result;
use clap::Parser;
use env_logger::{Builder as LoggerBuilder, Env, DEFAULT_FILTER_ENV};
use himalaya::{cli::Cli, config::TomlConfig, printer::StdoutPrinter};
use log::{debug, warn};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(not(target_os = "windows"))]
    if let Err((_, err)) = coredump::register_panic_handler() {
        warn!("cannot register custom panic handler: {err}");
        debug!("{err:?}");
    }

    LoggerBuilder::new()
        .parse_env(Env::new().filter_or(DEFAULT_FILTER_ENV, "warn"))
        .format_timestamp(None)
        .init();

    let raw_args: Vec<String> = env::args().collect();
    if raw_args.len() > 1 && raw_args[1].starts_with("mailto:") {
        // TODO
        // let cmd = MessageMailtoCommand::command()
        //     .no_binary_name(true)
        //     .try_get_matches_from([&raw_args[1]]);
        // match cmd {
        //     Ok(m) => m.exec
        // }
    }

    let cli = Cli::parse();

    let mut printer = StdoutPrinter::new(cli.output, cli.color);
    let config = TomlConfig::from_some_path_or_default(cli.config.as_ref()).await?;

    cli.command.execute(&mut printer, &config).await
}

// fn create_app() -> clap::Command {
//     clap::Command::new(env!("CARGO_PKG_NAME"))
//         .subcommand(message::args::subcmd())
//         .subcommand(template::args::subcmd())
// }

// #[tokio::main]
// async fn main() -> Result<()> {
//     // check mailto command before app initialization
//     let raw_args: Vec<String> = env::args().collect();
//     if raw_args.len() > 1 && raw_args[1].starts_with("mailto:") {
//         let url = Url::parse(&raw_args[1])?;
//         let (toml_account_config, account_config) = TomlConfig::from_default_paths()
//             .await?
//             .into_account_configs(None, false)?;
//         let backend_builder =
//             BackendBuilder::new(toml_account_config, account_config.clone(), true).await?;
//         let backend = backend_builder.build().await?;
//         let mut printer = StdoutPrinter::default();

//         return message::handlers::mailto(&account_config, &backend, &mut printer, &url).await;
//     }

//     match message::args::matches(&m)? {
//         Some(message::args::Cmd::Attachments(ids)) => {
//             let folder = folder.unwrap_or(DEFAULT_INBOX_FOLDER);
//             let backend = Backend::new(toml_account_config, account_config.clone(), false).await?;
//             return message::handlers::attachments(
//                 &account_config,
//                 &mut printer,
//                 &backend,
//                 &folder,
//                 ids,
//             )
//             .await;
//         }
//         _ => (),
//     }

//     match template::args::matches(&m)? {
//         Some(template::args::Cmd::Forward(id, headers, body)) => {
//             let folder = folder.unwrap_or(DEFAULT_INBOX_FOLDER);
//             let backend = Backend::new(toml_account_config, account_config.clone(), false).await?;
//             return template::handlers::forward(
//                 &account_config,
//                 &mut printer,
//                 &backend,
//                 &folder,
//                 id,
//                 headers,
//                 body,
//             )
//             .await;
//         }
//         Some(template::args::Cmd::Write(headers, body)) => {
//             return template::handlers::write(&account_config, &mut printer, headers, body).await;
//         }
//         Some(template::args::Cmd::Reply(id, all, headers, body)) => {
//             let folder = folder.unwrap_or(DEFAULT_INBOX_FOLDER);
//             let backend = Backend::new(toml_account_config, account_config.clone(), false).await?;
//             return template::handlers::reply(
//                 &account_config,
//                 &mut printer,
//                 &backend,
//                 &folder,
//                 id,
//                 all,
//                 headers,
//                 body,
//             )
//             .await;
//         }
//         Some(template::args::Cmd::Save(template)) => {
//             let folder = folder.unwrap_or(DEFAULT_INBOX_FOLDER);
//             let backend = Backend::new(toml_account_config, account_config.clone(), false).await?;
//             return template::handlers::save(
//                 &account_config,
//                 &mut printer,
//                 &backend,
//                 &folder,
//                 template,
//             )
//             .await;
//         }
//         Some(template::args::Cmd::Send(template)) => {
//             let backend = Backend::new(toml_account_config, account_config.clone(), true).await?;
//             return template::handlers::send(&account_config, &mut printer, &backend, template)
//                 .await;
//         }
//         _ => (),
//     }
// }
