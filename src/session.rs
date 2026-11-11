//! # Session
//!
//! The clients the commands of one run share, each built the first time a
//! command needs it and kept for the commands after it.
//!
//! A one-shot invocation runs a single command over a fresh [`Session`],
//! so it builds exactly the client that command needs. The [`crate::repl`]
//! keeps its session across lines and the [`crate::mcp`] server across
//! tool calls, so each client connects once. A command that fails drops
//! the client it ran on, its connection being possibly dead or left
//! mid-exchange, and the next command builds it again.

use std::path::PathBuf;

use anyhow::Result;
use pimalaya_cli::printer::Printer;

#[cfg(feature = "gmail")]
use crate::gmail::client::GmailClient;
#[cfg(feature = "imap")]
use crate::imap::client::ImapClient;
#[cfg(feature = "jmap")]
use crate::jmap::client::JmapClient;
#[cfg(feature = "m2dir")]
use crate::m2dir::client::M2dirClient;
#[cfg(feature = "maildir")]
use crate::maildir::client::MaildirClient;
#[cfg(feature = "mbox")]
use crate::mbox::client::MboxClient;
#[cfg(feature = "msgraph")]
use crate::msgraph::client::MsgraphClient;
#[cfg(feature = "pimdir")]
use crate::pimdir::client::PimdirClient;
#[cfg(any(backend, feature = "smtp"))]
use crate::shared::client::EmailClient;
#[cfg(feature = "sieve")]
use crate::sieve::client::SieveClient;
#[cfg(feature = "smtp")]
use crate::smtp::client::SmtpClient;
use crate::{
    account::context::Account,
    backend::Backend,
    cli::resolve_account,
    config::{AccountConfig, Config},
};

/// The clients of one run, one optional slot per client kind.
#[derive(Default)]
pub struct Session<'a> {
    /// The account every client of the session is built for.
    pub account: SessionAccount<'a>,
    /// The client of the shared commands.
    #[cfg(any(backend, feature = "smtp"))]
    pub email: Option<(Account, EmailClient)>,
    /// The client of the `imap` commands.
    #[cfg(feature = "imap")]
    pub imap: Option<(Account, ImapClient)>,
    /// The client of the `jmap` commands.
    #[cfg(feature = "jmap")]
    pub jmap: Option<(Account, JmapClient)>,
    /// The client of the `gmail` commands.
    #[cfg(feature = "gmail")]
    pub gmail: Option<(Account, GmailClient)>,
    /// The client of the `msgraph` commands.
    #[cfg(feature = "msgraph")]
    pub msgraph: Option<(Account, MsgraphClient)>,
    /// The client of the `maildir` commands.
    #[cfg(feature = "maildir")]
    pub maildir: Option<(Account, MaildirClient)>,
    /// The client of the `m2dir` commands.
    #[cfg(feature = "m2dir")]
    pub m2dir: Option<(Account, M2dirClient)>,
    /// The client of the `mbox` commands.
    #[cfg(feature = "mbox")]
    pub mbox: Option<(Account, MboxClient)>,
    /// The client of the `pimdir` commands.
    #[cfg(feature = "pimdir")]
    pub pimdir: Option<(Account, PimdirClient)>,
    /// The client of the `smtp` commands.
    #[cfg(feature = "smtp")]
    pub smtp: Option<(Account, SmtpClient)>,
    /// The client of the `sieve` commands.
    #[cfg(feature = "sieve")]
    pub sieve: Option<(Account, SieveClient)>,
}

/// The account a session runs against, as the global flags name it.
#[derive(Clone, Copy, Debug, Default)]
pub struct SessionAccount<'a> {
    /// The configuration paths, `-c` or the canonical ones.
    pub config_paths: &'a [PathBuf],
    /// The account `-a` names, the default one when `None`.
    pub name: Option<&'a str>,
    /// The backend `--backend` allows for the shared commands.
    pub backend: Backend,
}

impl SessionAccount<'_> {
    /// Runs a command over the client in `slot`, building it with `build`
    /// when the slot is empty, and emptying the slot when the command
    /// fails.
    pub fn run<P: Printer, C>(
        self,
        printer: &mut P,
        slot: &mut Option<(Account, C)>,
        build: impl FnOnce(Config, String, AccountConfig) -> Result<(Account, C)>,
        command: impl FnOnce(&mut P, &mut Account, &mut C) -> Result<()>,
    ) -> Result<()> {
        let (account, client) = match slot.take() {
            Some(kept) => slot.insert(kept),
            None => {
                let (config, name, account_config) = resolve_account(self.config_paths, self.name)?;
                slot.insert(build(config, name, account_config)?)
            }
        };

        let result = command(printer, account, client);

        if result.is_err() {
            *slot = None;
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, fs};

    use anyhow::bail;
    use pimalaya_cli::{clap::args::JsonFlag, printer::StdoutPrinter};
    use tempfile::TempDir;

    use crate::{account::context::Account, session::*};

    #[test]
    fn a_kept_client_is_built_once_and_rebuilt_after_a_failure() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, "[accounts.a]\ndefault = true\n").unwrap();
        let config_paths = [config_path];

        let account = SessionAccount {
            config_paths: &config_paths,
            ..Default::default()
        };
        let mut printer = StdoutPrinter::new(&JsonFlag::default());
        let mut slot: Option<(Account, ())> = None;
        let builds = Cell::new(0);
        let build = |config: Config, _name, _account_config| {
            builds.set(builds.get() + 1);
            Ok((Account::from(config), ()))
        };

        account
            .run(&mut printer, &mut slot, build, |_, _, _| Ok(()))
            .unwrap();
        account
            .run(&mut printer, &mut slot, build, |_, _, _| Ok(()))
            .unwrap();
        assert_eq!(builds.get(), 1);

        let failed = account.run(&mut printer, &mut slot, build, |_, _, _| bail!("Dead"));
        assert!(failed.is_err());
        assert!(slot.is_none());

        account
            .run(&mut printer, &mut slot, build, |_, _, _| Ok(()))
            .unwrap();
        assert_eq!(builds.get(), 2);
    }
}
