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
