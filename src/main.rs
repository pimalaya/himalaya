use anyhow::Result;
use clap::Parser;
use env_logger::{Builder as LoggerBuilder, Env, DEFAULT_FILTER_ENV};
use himalaya::{
    cli::Cli, config::TomlConfig, message::command::mailto::MessageMailtoCommand,
    printer::StdoutPrinter,
};
use log::{debug, warn};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(not(target_os = "windows"))]
    if let Err((_, err)) = coredump::register_panic_handler() {
        warn!("cannot register coredump panic handler: {err}");
        debug!("{err:?}");
    }

    LoggerBuilder::new()
        .parse_env(Env::new().filter_or(DEFAULT_FILTER_ENV, "warn"))
        .format_timestamp(None)
        .init();

    // if the first argument starts by "mailto:", execute straight the
    // mailto message command
    if let Some(ref url) = env::args().nth(1).filter(|arg| arg.starts_with("mailto:")) {
        let mut printer = StdoutPrinter::default();
        let config = TomlConfig::from_default_paths().await?;

        return MessageMailtoCommand::new(url)?
            .execute(&mut printer, &config)
            .await;
    }

    let cli = Cli::parse();
    let mut printer = StdoutPrinter::new(cli.output, cli.color);

    cli.command
        .execute(&mut printer, cli.config_path.as_ref())
        .await
}
