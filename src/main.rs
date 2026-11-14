//! # Himalaya
//!
//! The `himalaya` binary over the library: parses the command line, runs
//! the command over a fresh session and reports its outcome. The crate's
//! architecture is the library's header (src/lib.rs).

use std::process;

use anyhow::Result;
use himalaya::{
    cli::{self, Cli, Command},
    error::ErrorCode,
    session::{Session, SessionAccount},
};
use pimalaya_cli::{
    clap::{args::LogFlags, parse},
    error::{EXIT_FAILURE, ErrorReport},
    log::Logger,
    printer::StdoutPrinter,
    welcome::{is_wanted, shipped_sample},
};
use pimalaya_config::toml::ConfigMissingError;
use simd_cpu::SimdCpu;

fn main() {
    // NOTE: the base64 and quoted-printable codecs read the installed CPU
    // token, the portable baseline until one is installed.
    let _ = SimdCpu::install_detected();

    let cli = parse::<Cli>();
    let mut printer = StdoutPrinter::new(&cli.json);

    let account = SessionAccount {
        config_paths: &cli.config.paths,
        name: cli.account.name.as_deref(),
        backend: cli.backend,
    };

    let result = execute(cli.cmd, &cli.log, account, &mut printer);

    // A person missing a configuration meets the welcome, a script or a
    // JSON caller the error.
    if let Err(err) = &result
        && let Some(missing) = err.downcast_ref::<ConfigMissingError>()
        && is_wanted(&printer)
    {
        let sample = shipped_sample("himalaya");
        let welcome = cli::welcome(&missing.path, account.config_paths, sample.as_deref());
        eprint!("{welcome}");
        process::exit(EXIT_FAILURE);
    }

    ErrorReport::<ErrorCode>::eval_coded(&mut printer, result);
}

/// Runs the subcommand over a fresh session, or meets a bare `himalaya`.
fn execute(
    cmd: Option<Command>,
    log: &LogFlags,
    account: SessionAccount,
    printer: &mut StdoutPrinter,
) -> Result<()> {
    Logger::try_init(log)?;

    let Some(cmd) = cmd else {
        return cli::meet_bare_invocation(account.config_paths);
    };

    #[cfg_attr(
        not(any(backend, feature = "smtp", feature = "sieve")),
        allow(clippy::needless_update)
    )]
    let mut session = Session {
        account,
        ..Default::default()
    };

    cmd.execute(printer, &mut session)
}
