use clap::Parser;
use color_eyre::{Result, Section};
use himalaya::{
    cli::Cli, config::TomlConfig, envelope::command::list::ListEnvelopesCommand,
    message::command::mailto::MessageMailtoCommand, printer::StdoutPrinter,
};
use std::env;
use tracing::level_filters::LevelFilter;

#[tokio::main]
async fn main() -> Result<()> {
    if env::var("RUST_LOG").is_err() {
        if std::env::args().any(|arg| arg == "--debug") {
            env::set_var("RUST_LOG", "debug");
        }
        if std::env::args().any(|arg| arg == "--trace") {
            env::set_var("RUST_LOG", "trace");
        }
    }
    let cli = Cli::parse();

    let filter = himalaya::tracing::install()?;

    let mut printer = StdoutPrinter::new(cli.output, cli.color);

    // if the first argument starts by "mailto:", execute straight the
    // mailto message command
    let mailto = std::env::args()
        .nth(1)
        .filter(|arg| arg.starts_with("mailto:"));

    if let Some(ref url) = mailto {
        let mut printer = StdoutPrinter::default();
        let config = TomlConfig::from_default_paths().await?;

        return MessageMailtoCommand::new(url)?
            .execute(&mut printer, &config)
            .await;
    }

    let mut res = match cli.command {
        Some(cmd) => cmd.execute(&mut printer, cli.config_paths.as_ref()).await,
        None => {
            let config = TomlConfig::from_paths_or_default(cli.config_paths.as_ref()).await?;
            ListEnvelopesCommand::default()
                .execute(&mut printer, &config)
                .await
        }
    };

    if filter < LevelFilter::DEBUG {
        res = res.note("Run with --debug to enable logs with spantrace.");
    };

    if filter < LevelFilter::TRACE {
        res = res.note("Run with --trace to enable verbose logs with backtrace.")
    };

    res
}
