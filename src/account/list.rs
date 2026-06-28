use std::{fmt, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Color, ContentArrangement, Row, Table};
use crossterm::style::Color as CrosstermColor;
use pimalaya_cli::printer::Printer;
use pimalaya_config::toml::TomlConfig;
use serde::Serialize;

use crate::{
    account::context::map_color_or,
    config::{AccountConfig, Config, TableArrangementConfig},
};

/// List all accounts declared in the configuration.
///
/// Each row shows the account name, the backends with a config block,
/// and whether it is the default account.
#[derive(Debug, Parser)]
pub struct AccountListCommand;

impl AccountListCommand {
    pub fn execute(self, printer: &mut impl Printer, config_paths: &[PathBuf]) -> Result<()> {
        let config = load_config(config_paths)?;

        let preset = config
            .table
            .preset
            .clone()
            .unwrap_or_else(|| comfy_table::presets::UTF8_FULL_CONDENSED.to_string());
        let arrangement = config
            .table
            .arrangement
            .clone()
            .unwrap_or(TableArrangementConfig::Dynamic)
            .into();

        let table_cfg = &config.account.list.table;
        let colors = AccountColors {
            // v1.2.0 defaults: name=Green, backends=Blue, default=Reset.
            name: map_color_or(table_cfg.name_color, CrosstermColor::Green),
            backends: map_color_or(table_cfg.backends_color, CrosstermColor::Blue),
            default: map_color_or(table_cfg.default_color, CrosstermColor::Reset),
        };

        let mut accounts: Vec<AccountRow> = config
            .accounts
            .iter()
            .map(|(name, account)| AccountRow::from_account(name, account))
            .collect();
        accounts.sort_by(|a, b| a.name.cmp(&b.name));

        let table = AccountsTable {
            preset,
            arrangement,
            colors,
            accounts,
        };

        printer.out(table)
    }
}

#[derive(Clone, Copy, Debug)]
struct AccountColors {
    name: Color,
    backends: Color,
    default: Color,
}

fn load_config(paths: &[PathBuf]) -> Result<Config> {
    match Config::from_paths_or_default(paths)? {
        Some(config) => Ok(config),
        None => anyhow::bail!(
            "No configuration found. Run `himalaya` once to launch the wizard, \
             or `himalaya account configure <name>` to create one."
        ),
    }
}

/// One account row in the account list: name, backends, default flag.
#[derive(Clone, Debug, Serialize)]
pub struct AccountRow {
    pub name: String,
    pub default: bool,
    pub backends: Vec<&'static str>,
}

impl AccountRow {
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
        if account.smtp.is_some() {
            backends.push("smtp");
        }

        Self {
            name: name.to_owned(),
            default: account.default,
            backends,
        }
    }
}

/// Renderable table for the account list command.
#[derive(Clone, Debug, Serialize)]
pub struct AccountsTable {
    #[serde(skip)]
    pub preset: String,
    #[serde(skip)]
    pub arrangement: ContentArrangement,
    #[serde(skip)]
    colors: AccountColors,
    pub accounts: Vec<AccountRow>,
}

impl fmt::Display for AccountsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_content_arrangement(self.arrangement.clone())
            .set_header(Row::from(vec![
                Cell::new("NAME"),
                Cell::new("BACKENDS"),
                Cell::new("DEFAULT"),
            ]))
            .add_rows(self.accounts.iter().map(|account| {
                let mut row = Row::new();
                row.max_height(1);
                row.add_cell(Cell::new(&account.name).fg(self.colors.name));
                row.add_cell(Cell::new(account.backends.join(", ")).fg(self.colors.backends));
                row.add_cell(
                    Cell::new(if account.default { "yes" } else { "" }).fg(self.colors.default),
                );
                row
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
