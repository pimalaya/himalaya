//! # Account capability
//!
//! The `account capability` command, listing what the account serves of
//! the shared API, which backend serves each capability and how well.

use std::{fmt, path::PathBuf};

use anyhow::{Result, anyhow, bail};
use clap::Parser;
use pimalaya_cli::{
    printer::Printer,
    table::{Cell, ContentArrangement, Row, Table, sanitize},
};
use pimalaya_config::toml::TomlConfig;
use schemars::JsonSchema;
use serde::Serialize;

#[cfg(feature = "pimdir")]
use crate::pimdir::client::PimdirClient;
use crate::{
    backend::Backend,
    capability::{self, Resolution, StoreSupport},
    config::{Config, NO_CONFIG_HINT},
    shared::table::{DEFAULT_PRESET, style_from_preset},
};

/// List what the account serves of the shared API.
///
/// One row per capability: the backend it resolves to, the level it is
/// served at (`full`, `partial`, `server` when the server decides once
/// connected, `none`) and what the level leaves out. Read from the
/// account's configuration and the capability tables, nothing connecting;
/// on a pimdir account the writes are read from what the store's sources
/// declare. `--backend` shows the account under that filter.
#[derive(Debug, Parser)]
pub struct AccountCapabilityCommand;

impl AccountCapabilityCommand {
    /// Resolves each capability for the account and prints the table.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config_paths: &[PathBuf],
        account_name: Option<&str>,
        backend: Backend,
    ) -> Result<()> {
        let mut config = match Config::from_paths_or_default(config_paths)? {
            Some(config) => config,
            None => bail!("No configuration found, {NO_CONFIG_HINT}"),
        };

        let (name, account_config) = config
            .take_account(account_name)?
            .ok_or_else(|| anyhow!("Cannot find account"))?;

        let resolution = Resolution::of(&name, &account_config, backend);

        // NOTE: the store is opened as a reader, a local read writing
        // nothing, so the rows say what its gate would refuse.
        #[cfg(feature = "pimdir")]
        let pimdir = match (resolution.storage, account_config.pimdir) {
            (Some(Backend::Pimdir), Some(config)) => Some(PimdirClient::new(config)?),
            _ => None,
        };

        let mut capabilities = Vec::new();

        for capability in capability::all() {
            let backend = resolution.transport(capability);

            #[cfg_attr(not(feature = "pimdir"), allow(unused_mut))]
            let mut support = StoreSupport::from(resolution.support(capability));

            #[cfg(feature = "pimdir")]
            if let (Some(Backend::Pimdir), Some(client)) = (backend, &pimdir)
                && let Some(declared) = client.store_support(capability)?
            {
                support = declared;
            }

            capabilities.push(AccountCapability {
                name: capability.name(),
                backend: backend.map(|backend| backend.to_string()),
                level: support.level.to_string(),
                detail: support.detail,
            });
        }

        let preset = config
            .table
            .preset
            .clone()
            .unwrap_or_else(|| DEFAULT_PRESET.to_string());

        printer.out(AccountCapabilityOutput {
            preset,
            account: name,
            capabilities,
        })
    }
}

/// The `account capability` output.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountCapabilityOutput {
    /// The `comfy_table` preset string the table renders with.
    #[serde(skip)]
    pub preset: String,
    /// The account the capabilities are resolved for.
    pub account: String,
    /// One row per shared capability, in the order the commands reach
    /// them.
    pub capabilities: Vec<AccountCapability>,
}

/// How the account serves one shared capability.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountCapability {
    /// The capability, pimdir's grammar without the domain word
    /// (`message.move`, `envelope.search.body`).
    pub name: String,
    /// The backend it resolves to, `null` when the account configures none
    /// that `--backend` allows.
    pub backend: Option<String>,
    /// `full`, `partial`, `server` or `none`.
    pub level: String,
    /// What the level leaves out, `null` for a full one.
    pub detail: Option<String>,
}

impl fmt::Display for AccountCapabilityOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_style(style_from_preset(&self.preset))
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(Row::from([
                Cell::new("CAPABILITY"),
                Cell::new("BACKEND"),
                Cell::new("LEVEL"),
                Cell::new("DETAIL"),
            ]))
            .add_rows(self.capabilities.iter().map(|capability| {
                Row::from([
                    Cell::new(&capability.name),
                    Cell::new(capability.backend.as_deref().unwrap_or("")),
                    Cell::new(&capability.level),
                    Cell::new(sanitize(capability.detail.as_deref().unwrap_or(""))),
                ])
            }));

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
