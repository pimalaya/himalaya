use anyhow::Result;
use clap::Parser;
use env_logger::{Builder as LoggerBuilder, Env, DEFAULT_FILTER_ENV};
#[cfg(any(feature = "envelope-list", feature = "message-mailto"))]
use himalaya::config::TomlConfig;
#[cfg(feature = "envelope-list")]
use himalaya::envelope::command::list::ListEnvelopesCommand;
#[cfg(feature = "message-mailto")]
use himalaya::message::command::mailto::MessageMailtoCommand;
use himalaya::{cli::Cli, printer::StdoutPrinter};
use log::{debug, warn};

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

    #[cfg(feature = "message-mailto")]
    // if the first argument starts by "mailto:", execute straight the
    // mailto message command
    if let Some(ref url) = std::env::args()
        .nth(1)
        .filter(|arg| arg.starts_with("mailto:"))
    {
        let mut printer = StdoutPrinter::default();
        let config = TomlConfig::from_default_paths().await?;

        return MessageMailtoCommand::new(url)?
            .execute(&mut printer, &config)
            .await;
    }

    let cli = Cli::parse();
    let mut printer = StdoutPrinter::new(cli.output, cli.color);

    #[cfg(feature = "envelope-list")]
    match cli.command {
        Some(cmd) => return cmd.execute(&mut printer, cli.config_path.as_ref()).await,
        None => {
            let config = TomlConfig::from_some_path_or_default(cli.config_path.as_ref()).await?;
            return ListEnvelopesCommand::default()
                .execute(&mut printer, &config)
                .await;
        }
    }

    #[cfg(not(feature = "envelope-list"))]
    return cli
        .command
        .execute(&mut printer, cli.config_path.as_ref())
        .await;
}
