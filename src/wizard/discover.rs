//! First-run configuration wizard.
//!
//! Triggered by `cli::load_or_wizard` when no config file is found.
//!
//! One prompt takes an email address, a server URL, or a local folder
//! path, and its shape orients the setup, mirroring the cardamum-android
//! onboarding:
//!
//! - an email (or bare domain) runs pimconf's parallel discovery (see
//!   [`super::search`]) and every reachable service and authentication
//!   method becomes one selectable configuration; a detected Google or
//!   Microsoft account collapses to its dedicated set;
//! - a `scheme://` URL is a server to configure by hand;
//! - an existing folder is a local Maildir or m2dir.
//!
//! OAuth 2.0 entries appear in the list but cannot be configured here:
//! selecting one prints a note pointing at an external token manager
//! and returns to the list.

use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result, bail};
use pimalaya_cli::{prompt, spinner::Spinner};
use url::Url;

#[cfg(feature = "gmail")]
use crate::config::GmailConfig;
#[cfg(feature = "jmap")]
use crate::config::JmapConfig;
#[cfg(feature = "m2dir")]
use crate::config::M2dirConfig;
#[cfg(feature = "maildir")]
use crate::config::MaildirConfig;
#[cfg(feature = "msgraph")]
use crate::config::MsgraphConfig;
#[cfg(all(feature = "imap", feature = "smtp"))]
use crate::config::{ImapConfig, SmtpConfig};
#[cfg(feature = "gmail")]
use crate::wizard::gmail;
#[cfg(all(feature = "imap", feature = "smtp"))]
use crate::wizard::imap_smtp;
#[cfg(feature = "jmap")]
use crate::wizard::jmap;
#[cfg(any(feature = "maildir", feature = "m2dir"))]
use crate::wizard::local;
#[cfg(feature = "msgraph")]
use crate::wizard::msgraph;
use crate::{
    config::{AccountConfig, Config},
    wizard::search::{self, Discovered, DiscoveredAuth, DiscoveredKind},
};

/// The endpoint prompt label, shared by the create flow.
const ENDPOINT_PROMPT: &str = "Email, server or folder path:";

/// The backend config produced by the chosen flow, folded into a fresh
/// [`AccountConfig`] afterwards.
enum Chosen {
    #[cfg(all(feature = "imap", feature = "smtp"))]
    ImapSmtp(Box<ImapConfig>, Box<SmtpConfig>),
    #[cfg(feature = "jmap")]
    Jmap(Box<JmapConfig>),
    #[cfg(feature = "gmail")]
    Gmail(GmailConfig),
    #[cfg(feature = "msgraph")]
    Msgraph(MsgraphConfig),
    #[cfg(feature = "maildir")]
    Maildir(MaildirConfig),
    #[cfg(feature = "m2dir")]
    M2dir(M2dirConfig),
}

/// Runs the wizard, writing the resulting [`Config`] to `target`.
/// Returns `Ok(None)` when the user declines to create one.
pub fn run(target: &Path) -> Result<Option<Config>> {
    let confirm = format!(
        "No configuration found. Create one at {}?",
        target.display()
    );
    if !prompt::bool(&confirm, true)? {
        return Ok(None);
    }

    let input = prompt::text::<&str>(ENDPOINT_PROMPT, None)?;
    let input = input.trim();
    if input.is_empty() {
        bail!("Empty input: enter an email address, a server URL, or a folder path");
    }

    let account_name = prompt::text("Account name:", Some(&default_account_name(input)))?;
    let account = build_account(&account_name, input)?;

    let config = Config {
        accounts: HashMap::from([(account_name, account)]),
        ..Default::default()
    };

    config.write(target)?;
    println!("Configuration written to {}.", target.display());

    Ok(Some(config))
}

/// Orients the setup from the input shape, then folds the chosen
/// backend into a fresh default [`AccountConfig`].
fn build_account(account_name: &str, input: &str) -> Result<AccountConfig> {
    let chosen = if is_path(input) {
        configure_local(input)?
    } else if input.contains("://") {
        configure_server(account_name, input)?
    } else {
        configure_email(account_name, input)?
    };

    let mut account = AccountConfig {
        default: true,
        ..Default::default()
    };

    match chosen {
        #[cfg(all(feature = "imap", feature = "smtp"))]
        Chosen::ImapSmtp(imap, smtp) => {
            account.imap = Some(*imap);
            account.smtp = Some(*smtp);
        }
        #[cfg(feature = "jmap")]
        Chosen::Jmap(jmap) => account.jmap = Some(*jmap),
        #[cfg(feature = "gmail")]
        Chosen::Gmail(gmail) => account.gmail = Some(gmail),
        #[cfg(feature = "msgraph")]
        Chosen::Msgraph(msgraph) => account.msgraph = Some(msgraph),
        #[cfg(feature = "maildir")]
        Chosen::Maildir(maildir) => account.maildir = Some(maildir),
        #[cfg(feature = "m2dir")]
        Chosen::M2dir(m2dir) => account.m2dir = Some(m2dir),
    }

    Ok(account)
}

/// Runs the email-driven discovery flow: search the services reachable
/// from the address, let the user pick one (OAuth entries loop back
/// with a note), and configure its backend. Falls back to manual
/// IMAP+SMTP entry when nothing is discovered.
fn configure_email(account_name: &str, input: &str) -> Result<Chosen> {
    let email = if input.contains('@') {
        input.to_string()
    } else {
        format!("@{input}")
    };

    let spinner = Spinner::start("Searching for email server settings");
    let mut found = search::search(&email)?;
    retain_supported(&mut found);

    if found.is_empty() {
        spinner.failure("No configuration found");
        return manual_fallback(account_name, &email);
    }
    spinner.success(format!("Found {} configuration(s)", found.len()));

    // OAuth entries loop back with a note, but only when a usable
    // configuration remains to fall back on; otherwise the note is
    // shown and the manual flow takes over so the user is not trapped.
    let has_usable = found
        .iter()
        .any(|entry| entry.auth != DiscoveredAuth::OAuth);

    loop {
        let default = found.first().cloned();
        let choice = prompt::item("Choose a configuration:", found.clone(), default)?;

        if choice.auth == DiscoveredAuth::OAuth {
            print_oauth_note();
            if has_usable {
                continue;
            }
            return manual_fallback(account_name, &email);
        }

        return dispatch(&email, choice);
    }
}

/// Configures the backend behind a discovered entry.
#[cfg_attr(
    all(
        feature = "imap",
        feature = "smtp",
        feature = "jmap",
        feature = "gmail",
        feature = "msgraph"
    ),
    allow(unreachable_patterns)
)]
fn dispatch(email: &str, choice: Discovered) -> Result<Chosen> {
    match &choice.kind {
        #[cfg(all(feature = "imap", feature = "smtp"))]
        DiscoveredKind::ImapSmtp { .. } => {
            let (imap, smtp) = imap_smtp::configure_discovered(email, &choice)?;
            Ok(Chosen::ImapSmtp(Box::new(imap), Box::new(smtp)))
        }
        #[cfg(feature = "jmap")]
        DiscoveredKind::Jmap(_) => Ok(Chosen::Jmap(Box::new(jmap::configure_discovered(
            email, &choice,
        )?))),
        #[cfg(feature = "gmail")]
        DiscoveredKind::Gmail => Ok(Chosen::Gmail(gmail::configure()?)),
        #[cfg(feature = "msgraph")]
        DiscoveredKind::Msgraph => Ok(Chosen::Msgraph(msgraph::configure()?)),
        kind => bail!("Configuration `{kind:?}` is not supported by this build"),
    }
}

/// Configures IMAP + SMTP by hand when discovery yields nothing.
#[cfg(all(feature = "imap", feature = "smtp"))]
fn manual_fallback(account_name: &str, email: &str) -> Result<Chosen> {
    let (local_part, domain) = split_email(email);
    let (imap, smtp) = imap_smtp::configure_manual(account_name, local_part, domain, None)?;
    Ok(Chosen::ImapSmtp(Box::new(imap), Box::new(smtp)))
}

#[cfg(not(all(feature = "imap", feature = "smtp")))]
fn manual_fallback(_account_name: &str, email: &str) -> Result<Chosen> {
    bail!("No configuration discovered for `{email}`, and IMAP+SMTP is not compiled in")
}

/// Configures a server the user typed as a `scheme://` URL, routing by
/// scheme: `imap`/`imaps` to IMAP+SMTP, an HTTP-family scheme to JMAP.
#[cfg_attr(
    not(any(all(feature = "imap", feature = "smtp"), feature = "jmap")),
    allow(unused_variables)
)]
fn configure_server(account_name: &str, input: &str) -> Result<Chosen> {
    let url = Url::parse(input).with_context(|| format!("Invalid server URL `{input}`"))?;
    let domain = url.host_str().unwrap_or_default().to_string();

    match url.scheme() {
        #[cfg(all(feature = "imap", feature = "smtp"))]
        "imap" | "imaps" => {
            let (imap, smtp) = imap_smtp::configure_manual(account_name, "", &domain, Some(&url))?;
            Ok(Chosen::ImapSmtp(Box::new(imap), Box::new(smtp)))
        }
        #[cfg(feature = "jmap")]
        "http" | "https" | "jmap" | "jmaps" => {
            Ok(Chosen::Jmap(Box::new(jmap::configure_manual(input, None)?)))
        }
        other => bail!("Unsupported server scheme `{other}`"),
    }
}

/// Configures a local backend from a typed folder path.
#[cfg(any(feature = "maildir", feature = "m2dir"))]
fn configure_local(input: &str) -> Result<Chosen> {
    let raw = input.strip_prefix("file://").unwrap_or(input);
    let root = shellexpand::tilde(raw).into_owned();
    if !Path::new(&root).is_dir() {
        bail!("No such folder `{raw}`");
    }

    Ok(match local::configure(root.into())? {
        #[cfg(feature = "maildir")]
        local::Local::Maildir(config) => Chosen::Maildir(config),
        #[cfg(feature = "m2dir")]
        local::Local::M2dir(config) => Chosen::M2dir(config),
    })
}

#[cfg(not(any(feature = "maildir", feature = "m2dir")))]
fn configure_local(input: &str) -> Result<Chosen> {
    bail!("`{input}` looks like a folder path, but no local backend is compiled in")
}

/// Drops the discovered entries whose backend is not compiled in.
fn retain_supported(found: &mut Vec<Discovered>) {
    found.retain(|entry| match entry.kind {
        DiscoveredKind::ImapSmtp { .. } => cfg!(all(feature = "imap", feature = "smtp")),
        DiscoveredKind::Jmap(_) => cfg!(feature = "jmap"),
        DiscoveredKind::Gmail => cfg!(feature = "gmail"),
        DiscoveredKind::Msgraph => cfg!(feature = "msgraph"),
    });
}

/// Prints the note shown when an OAuth 2.0 entry is selected, before
/// returning to the configuration list.
fn print_oauth_note() {
    println!(
        "OAuth 2.0 is not configured directly by Himalaya; install an external token manager such as Ortie, then choose the \"API token\" configuration"
    );
}

/// Proposes a default account name from the input shape: the local part
/// of an email, the first label of a domain or host, or the folder name
/// of a local path.
fn default_account_name(input: &str) -> String {
    if is_path(input) {
        let raw = input.strip_prefix("file://").unwrap_or(input);
        return Path::new(raw)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("personal")
            .to_string();
    }

    if let Ok(url) = Url::parse(input) {
        if let Some(host) = url.host_str() {
            return first_label(host);
        }
    }

    match input.rsplit_once('@') {
        Some((local, _)) if !local.is_empty() => local.to_string(),
        Some((_, domain)) => first_label(domain),
        None => first_label(input),
    }
}

/// The first dot-separated label of a host or domain.
fn first_label(host: &str) -> String {
    host.split('.').next().unwrap_or(host).to_string()
}

/// Whether the input names a filesystem path (absolute, home-relative,
/// explicitly relative, or a `file://` URL) rather than a network
/// endpoint.
fn is_path(input: &str) -> bool {
    input.starts_with("file://")
        || input.starts_with('/')
        || input.starts_with('~')
        || input.starts_with("./")
        || input.starts_with("../")
}

/// Splits an email into its local part and domain, tolerating the
/// bare-domain form (`@domain`) discovery synthesizes.
#[cfg(all(feature = "imap", feature = "smtp"))]
fn split_email(email: &str) -> (&str, &str) {
    email.rsplit_once('@').unwrap_or(("", email))
}
