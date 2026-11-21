//! # Account capability
//!
//! The `account capability` command, listing what the account serves of
//! the unified API, which backend serves each capability and how well.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{
    capability::{AccountCapability, AccountCapabilityOutput},
    printer::Printer,
};

#[cfg(feature = "pimdir")]
use crate::pimdir::client::PimdirClient;
use crate::{
    cli::resolve_account,
    unified::{
        backend::Backend,
        capability::{self, Resolution, StoreSupport},
    },
};

/// List what the account serves of the unified API.
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
    /// Resolves each capability for the account and lists them.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        config_paths: &[PathBuf],
        account_name: Option<&str>,
        backend: Backend,
    ) -> Result<()> {
        let (_, name, account_config) = resolve_account(config_paths, account_name)?;
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
                level: support.level,
                detail: support.detail,
            });
        }

        printer.out(AccountCapabilityOutput {
            account: name,
            capabilities,
        })
    }
}
