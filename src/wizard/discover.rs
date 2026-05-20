// This file is part of Himalaya, a CLI to manage emails.
//
// Copyright (C) 2022-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Interactive configuration wizard with discovery-based defaults.
//!
//! Triggered by `cli::load_or_wizard` when no config file is found
//! ([`pimalaya_config::toml::TomlConfig::from_paths_or_default`]
//! returned `Ok(None)`).
//!
//! Flow:
//!
//! 1. Confirm with the user. Exit if they decline.
//! 2. Ask for an account name and email address.
//! 3. Try PACC, then Autoconfig (ISP main / fallback / ISPDB, secure
//!    variants only), then RFC 6186 SRV; each probe owns its own
//!    spinner and first hit wins.
//! 4. If PACC returned a JMAP endpoint, ask the user whether to use
//!    it instead of IMAP+SMTP and run the matching protocol wizard(s).
//! 5. Build a [`Config`], write it to `target`, return it.

use std::{collections::HashMap, path::Path, process::exit};

use anyhow::{anyhow, Result};
use log::info;
use pimalaya_cli::{
    prompt,
    wizard::{
        imap::{self as imap_wizard, WizardImapConfig},
        jmap::{self as jmap_wizard, WizardJmapConfig},
        smtp::{self as smtp_wizard, WizardSmtpConfig},
    },
};
use pimalaya_stream::tls::Tls;
use url::Url;

use crate::{
    config::{AccountConfig, Config},
    wizard::{
        account::{imap_to_config, jmap_to_config, smtp_to_config},
        autoconfig, pacc, srv,
    },
};

/// DNS resolver used by PACC, Autoconfig, and SRV discovery.
/// Cloudflare's `1.1.1.1` is a reasonable default; we'll make this
/// configurable later.
const DEFAULT_RESOLVER: &str = "tcp://1.1.1.1:53";

/// Parses [`DEFAULT_RESOLVER`] into a [`Url`]. The const is fixed at
/// build time, so a parse failure is a static bug.
pub fn discovery_resolver() -> Url {
    DEFAULT_RESOLVER
        .parse()
        .expect("DEFAULT_RESOLVER must be a valid URL")
}

/// Builds the [`Tls`] profile passed to the per-mechanism discovery
/// clients via `with_tls`. Discovery only speaks HTTPS to `_well-known`
/// endpoints, so `http/1.1` is the only ALPN protocol we offer.
pub fn discovery_tls() -> Tls {
    let mut tls = Tls::default();
    tls.rustls.alpn = vec!["http/1.1".into()];
    tls
}

#[derive(Default)]
pub struct DiscoveryResult {
    pub jmap: Option<WizardJmapConfig>,
    pub imap: Option<WizardImapConfig>,
    pub smtp: Option<WizardSmtpConfig>,
}

impl DiscoveryResult {
    pub fn is_empty(&self) -> bool {
        self.imap.is_none() && self.smtp.is_none() && self.jmap.is_none()
    }
}

pub fn run_or_exit(target: &Path) -> Result<Config> {
    let prompt = format!(
        "No configuration found. Create one at {}?",
        target.display(),
    );

    if !prompt::bool(&prompt, true)? {
        exit(0);
    }

    let account_name = prompt::text("Account name:", Some("default"))?;
    let email = prompt::text::<&str>("Email address:", None)?;

    let (local_part, domain) = email
        .rsplit_once('@')
        .ok_or_else(|| anyhow!("Invalid email address `{email}`: missing `@`"))?;

    let discovery = discover(local_part, domain);

    let account = build_account_from_discovery(&account_name, local_part, domain, discovery)?;

    let config = Config {
        downloads_dir: None,
        table_preset: None,
        table_arrangement: None,
        envelope: Default::default(),
        mailbox: Default::default(),
        message: Default::default(),
        accounts: HashMap::from([(account_name, account)]),
    };

    config.write(target)?;
    info!("Configuration written to {}.", target.display());

    Ok(config)
}

/// Runs PACC, then Autoconfig, then SRV in series; first non-empty
/// `DiscoveryResult` wins. Each mechanism reports its own spinner
/// line. Returns an empty `DiscoveryResult` when every mechanism
/// failed; the caller falls back to pure manual entry in that case.
fn discover(local_part: &str, domain: &str) -> DiscoveryResult {
    if let Some(result) = pacc::run(domain)
        .map(|c| pacc::defaults(&c))
        .filter(|r| !r.is_empty())
    {
        return result;
    }

    if let Some(result) = autoconfig::run(local_part, domain)
        .map(|c| autoconfig::defaults(&c))
        .filter(|r| !r.is_empty())
    {
        return result;
    }

    if let Some(result) = srv::run(domain)
        .map(|r| srv::defaults(&r))
        .filter(|r| !r.is_empty())
    {
        return result;
    }

    DiscoveryResult::default()
}

/// Decides whether to run the JMAP wizard or the IMAP+SMTP wizard
/// pair and builds an [`AccountConfig`] from the answers. The JMAP
/// branch fires when PACC discovered a JMAP endpoint and either the
/// user opted into it (when IMAP+SMTP defaults were also present) or
/// nothing else is available.
fn build_account_from_discovery(
    account_name: &str,
    local_part: &str,
    domain: &str,
    discovery: DiscoveryResult,
) -> Result<AccountConfig> {
    let DiscoveryResult { imap, smtp, jmap } = discovery;

    let prefer_jmap = match (&jmap, imap.is_some() || smtp.is_some()) {
        (Some(_), true) => prompt::bool(
            "A JMAP server was discovered. Use it instead of IMAP+SMTP?",
            true,
        )?,
        (Some(_), false) => true,
        (None, _) => false,
    };

    if prefer_jmap {
        let jmap_defaults = jmap.as_ref();
        let jmap = jmap_wizard::run(account_name, local_part, domain, jmap_defaults)?;

        Ok(AccountConfig {
            default: true,
            downloads_dir: None,
            table_preset: None,
            table_arrangement: None,
            envelope: Default::default(),
            mailbox: Default::default(),
            imap: None,
            jmap: Some(jmap_to_config(jmap)?),
            maildir: None,
            smtp: None,
        })
    } else {
        let imap = imap_wizard::run(account_name, local_part, domain, imap.as_ref())?;
        let smtp = smtp_wizard::run(account_name, local_part, domain, smtp.as_ref())?;

        Ok(AccountConfig {
            default: true,
            downloads_dir: None,
            table_preset: None,
            table_arrangement: None,
            envelope: Default::default(),
            mailbox: Default::default(),
            imap: Some(imap_to_config(imap)?),
            jmap: None,
            maildir: None,
            smtp: Some(smtp_to_config(smtp)?),
        })
    }
}
