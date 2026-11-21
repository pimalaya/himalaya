//! # Account list
//!
//! The `account list` command, listing every account a configuration
//! declares.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{
    account::{AccountListOutput, AccountRow},
    printer::Printer,
};
use pimalaya_config::toml::TomlConfig;

use crate::{
    config::{AccountConfig, Config},
    unified::backend::Backend,
};

/// List all accounts declared in the configuration.
///
/// Each row shows the account name, the backends with a config block, a
/// backend this build leaves out marked `(not built)`, and `default` on
/// the default account.
#[derive(Debug, Parser)]
pub struct AccountListCommand;

impl AccountListCommand {
    /// Loads the configuration, taking no account, and prints its
    /// accounts as a list.
    pub fn execute(self, printer: &mut impl Printer, config_paths: &[PathBuf]) -> Result<()> {
        let config = Config::load(config_paths)?;

        let mut accounts: Vec<AccountRow> = config
            .accounts
            .iter()
            .map(|(name, account)| account_row(name, account))
            .collect();
        accounts.sort_by(|a, b| a.name.cmp(&b.name));

        printer.out(AccountListOutput { accounts })
    }
}

/// Reads the row off one account block: the backends it declares a block
/// for, split between those this build serves and those it leaves out.
fn account_row(name: &str, account: &AccountConfig) -> AccountRow {
    let (built, not_built): (Vec<Backend>, Vec<Backend>) = account
        .configured()
        .into_iter()
        .partition(|backend| Backend::COMPILED.contains(backend));

    AccountRow {
        name: name.to_owned(),
        default: account.default,
        backends: built.into_iter().map(Backend::as_str).collect(),
        not_built: not_built.into_iter().map(Backend::as_str).collect(),
    }
}

#[cfg(test)]
mod tests {
    use std::{env, process};

    use pimalaya_cli::{clap::args::JsonFlag, printer::StdoutPrinter};
    use pimalaya_config::toml::ConfigMissingError;

    use crate::account::list::AccountListCommand;

    #[cfg(all(feature = "pimdir", feature = "smtp"))]
    #[test]
    fn a_pimdir_block_is_listed_among_the_backends() {
        use pimalaya_cli::account::AccountListOutput;

        use crate::{account::list::account_row, config::Config};

        let toml = "[accounts.a]\npimdir.root = \"/nonexistent\"\n\
                    smtp.server = \"smtp.example.org\"\n";
        let config: Config = toml::from_str(toml).unwrap();
        let row = account_row("a", &config.accounts["a"]);
        assert_eq!(row.backends, ["pimdir", "smtp"]);
        assert!(row.not_built.is_empty(), "{:?}", row.not_built);

        let output = AccountListOutput {
            accounts: vec![row],
        };
        assert!(output.to_string().contains("  pimdir,smtp  "), "{output}");
    }

    /// Listing takes no account, but a missing configuration still names
    /// the path it looked at.
    #[test]
    fn a_missing_configuration_names_the_path() {
        let dir = env::temp_dir().join(format!("himalaya-account-list-{}", process::id()));
        let paths = [dir.join("config.toml")];
        let mut printer = StdoutPrinter::new(&JsonFlag::default());

        let err = AccountListCommand
            .execute(&mut printer, &paths)
            .unwrap_err();

        let missing = err.downcast_ref::<ConfigMissingError>().unwrap();
        assert_eq!(missing.path, paths[0]);
        assert!(err.to_string().contains(&paths[0].display().to_string()));
    }
}
