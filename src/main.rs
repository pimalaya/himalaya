use clap::Parser;
use himalaya::cli::HimalayaCli;
use pimalaya_toolbox::terminal::{error::ErrorReport, log::Logger, printer::StdoutPrinter};

fn main() {
    let cli = HimalayaCli::parse();

    Logger::init(&cli.log);

    let mut printer = StdoutPrinter::new(&cli.json);
    let config_paths = cli.config_paths.as_ref();
    let account_name = cli.account.name.as_deref();

    let result = cli
        .command
        .execute(&mut printer, config_paths, account_name);

    ErrorReport::eval(&mut printer, result)
}

// fn main() {
//     let tracing = tracing::install()?;

//     #[cfg(feature = "keyring")]
//     secret::keyring::set_global_service_name("himalaya-cli");

//     // if the first argument starts by "mailto:", execute straight the
//     // mailto message command
//     let mailto = std::env::args()
//         .nth(1)
//         .filter(|arg| arg.starts_with("mailto:"));

//     if let Some(ref url) = mailto {
//         let mut printer = StdoutPrinter::default();
//         let config = TomlConfig::from_default_paths().await?;

//         return MessageMailtoCommand::new(url)?
//             .execute(&mut printer, &config)
//             .await;
//     }

//     let cli = Cli::parse();
//     let mut printer = StdoutPrinter::new(cli.output);
//     let res = match cli.command {
//         Some(cmd) => cmd.execute(&mut printer, cli.config_paths.as_ref()).await,
//         None => {
//             let config = TomlConfig::from_paths_or_default(cli.config_paths.as_ref()).await?;
//             EnvelopeListCommand::default()
//                 .execute(&mut printer, &config)
//                 .await
//         }
//     };

//     tracing.with_debug_and_trace_notes(res)
// }
