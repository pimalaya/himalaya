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

use crate::config::{AccountConfig, Config, NO_CONFIG_HINT};

/// List all accounts declared in the configuration.
///
/// Each row shows the account name, the backends with a config block,
/// and whether it is the default account.
#[derive(Debug, Parser)]
pub struct AccountListCommand;

impl AccountListCommand {
    /// Loads the configuration and prints its accounts as a list.
    pub fn execute(self, printer: &mut impl Printer, config_paths: &[PathBuf]) -> Result<()> {
        let config = load_config(config_paths)?;

        let mut accounts: Vec<AccountRow> = config
            .accounts
            .iter()
            .map(|(name, account)| AccountRow::from_account(name, account))
            .collect();
        accounts.sort_by(|a, b| a.name.cmp(&b.name));

        printer.out(AccountListOutput { accounts })
    }
}

/// Loads the merged configuration, pointing at the sample when there is
/// none to load.
fn load_config(paths: &[PathBuf]) -> Result<Config> {
    match Config::from_paths_or_default(paths)? {
        Some(config) => Ok(config),
        None => anyhow::bail!("No configuration found, {NO_CONFIG_HINT}"),
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
