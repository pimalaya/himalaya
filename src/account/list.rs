//! # Account list
//!
//! The `account list` command, listing every account a configuration
//! declares.

use std::{fmt, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{columns::Columns, printer::Printer};
use pimalaya_config::toml::TomlConfig;
use schemars::JsonSchema;
use serde::Serialize;

use crate::config::{AccountConfig, Config};

/// List all accounts declared in the configuration.
///
/// Each row shows the account name, the backends with a config block,
/// and whether it is the default account.
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
            .map(|(name, account)| AccountRow::from_account(name, account))
            .collect();
        accounts.sort_by(|a, b| a.name.cmp(&b.name));

        printer.out(AccountListOutput { accounts })
    }
}

/// One account of the list.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountRow {
    /// The name of the `[accounts.<name>]` block.
    pub name: String,
    /// Whether a command with no `-a/--account` runs against it.
    pub default: bool,
    /// The backends it declares a block for.
    pub backends: Vec<&'static str>,
}

impl AccountRow {
    /// Reads the row off one account block.
    fn from_account(name: &str, account: &AccountConfig) -> Self {
        let mut backends = Vec::new();
        if account.imap.is_some() {
            backends.push("imap");
        }
        if account.jmap.is_some() {
            backends.push("jmap");
        }
        if account.gmail.is_some() {
            backends.push("gmail");
        }
        if account.msgraph.is_some() {
            backends.push("msgraph");
        }
        if account.maildir.is_some() {
            backends.push("maildir");
        }
        if account.m2dir.is_some() {
            backends.push("m2dir");
        }
        if account.mbox.is_some() {
            backends.push("mbox");
        }
        if account.pimdir.is_some() {
            backends.push("pimdir");
        }
        if account.smtp.is_some() {
            backends.push("smtp");
        }
        if account.sieve.is_some() {
            backends.push("sieve");
        }

        Self {
            name: name.to_owned(),
            default: account.default,
            backends,
        }
    }
}

/// The `account list` output, the accounts the configuration declares.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountListOutput {
    /// The accounts, sorted by name.
    pub accounts: Vec<AccountRow>,
}

impl fmt::Display for AccountListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut columns = Columns::new(["NAME", "BACKENDS", "DEFAULT"]);

        for account in &self.accounts {
            columns.push([
                account.name.clone(),
                account.backends.join(","),
                String::from(if account.default { "yes" } else { "" }),
            ]);
        }

        write!(f, "{columns}")
    }
}

#[cfg(test)]
mod tests {
    use std::{env, process};

    use pimalaya_cli::{clap::args::JsonFlag, printer::StdoutPrinter};
    use pimalaya_config::toml::ConfigMissingError;

    use crate::{account::list::*, config::Config};

    #[test]
    fn a_pimdir_block_is_listed_among_the_backends() {
        let toml = "[accounts.a]\npimdir.root = \"/nonexistent\"\n\
                    smtp.server = \"smtp.example.org\"\n";
        let config: Config = toml::from_str(toml).unwrap();
        let row = AccountRow::from_account("a", &config.accounts["a"]);
        assert_eq!(row.backends, ["pimdir", "smtp"]);

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
