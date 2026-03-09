use std::{env::temp_dir, path::PathBuf};

use crate::config::{AccountConfig, Config};
use anyhow::Result;
use comfy_table::presets;
use dirs::download_dir;

#[derive(Debug)]
pub struct Account<B> {
    pub backend: B,
    pub downloads_dir: PathBuf,
    pub table_preset: String,
}

impl<B> Account<B> {
    pub fn new(config: Config, account_config: AccountConfig, backend: B) -> Result<Self> {
        Ok(Self {
            backend,

            downloads_dir: account_config
                .downloads_dir
                .as_ref()
                .and_then(|dir| dir.to_str())
                .and_then(|dir| shellexpand::full(dir).ok())
                .map(|dir| PathBuf::from(dir.to_string()))
                .or(config
                    .downloads_dir
                    .as_ref()
                    .and_then(|dir| dir.to_str())
                    .and_then(|dir| shellexpand::full(dir).ok())
                    .map(|dir| PathBuf::from(dir.to_string())))
                .or(download_dir())
                .unwrap_or_else(temp_dir),

            table_preset: config
                .table_preset
                .or(account_config.table_preset)
                .unwrap_or(presets::UTF8_FULL_CONDENSED.to_string()),
        })
    }
}
