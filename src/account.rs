use std::{env::temp_dir, path::PathBuf};

use crate::config::{AccountConfig, Config};
use anyhow::Result;
use comfy_table::presets;
use dirs::download_dir;

#[derive(Debug)]
pub struct Account<B> {
    pub backend: B,

    pub email: String,
    pub display_name: Option<String>,
    pub signature: String,
    pub downloads_dir: PathBuf,

    pub table_preset: &'static str,
}

impl<B> Account<B> {
    pub fn new(config: Config, account_config: AccountConfig, backend: B) -> Result<Self> {
        Ok(Self {
            backend,
            email: account_config.email,
            display_name: account_config.display_name.or(config.display_name),
            signature: match account_config.signature.or(config.signature) {
                None => String::new(),
                Some(ref signature) => {
                    account_config
                        .signature_delim
                        .or(config.signature_delim)
                        .unwrap_or(String::from("-- \n"))
                        + signature
                }
            },
            downloads_dir: match account_config
                .downloads_dir
                .as_ref()
                .and_then(|dir| dir.to_str())
            {
                Some(dir) => PathBuf::from(shellexpand::full(dir)?.to_string()),
                None => download_dir().unwrap_or_else(temp_dir),
            },

            table_preset: presets::UTF8_FULL_CONDENSED,
        })
    }
}
