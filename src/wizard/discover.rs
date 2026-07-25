//! Configuration wizard.
//!
//! Run on bare `himalaya` (no subcommand), and proposed by
//! `cli::resolve_account` when no config file is found. It writes
//! nothing to disk: the resulting account is printed as a ready-to-save
//! TOML document on stdout (prompts render on stderr), so
//! `himalaya > <config>` is the write-back, exactly like ortie.
//!
//! One prompt takes an email address, a server URL, or a local folder
//! path, and its shape orients the setup, mirroring the cardamum-android
//! onboarding:
//!
//! - an email (or bare domain) runs io-pim-discovery's parallel
//!   discovery (see [`super::search`]) and every reachable service
//!   becomes one selectable configuration; picking one then prompts its
//!   authentication method (SASL mechanism or HTTP scheme) among those
//!   advertised; a detected Google or Microsoft account collapses to its
//!   dedicated set;
//! - a `scheme://` URL is a server to configure by hand;
//! - an existing folder is a local Maildir or m2dir.
//!
//! Himalaya runs no OAuth 2.0 grant itself: a grant only unlocks the
//! external token brokers (Ortie, pizauth, oama) behind the API token
//! credential prompt (see [`super::secret`]).

use std::{collections::HashMap, fmt, path::Path};

use anyhow::{Context, Result, bail};
use pimalaya_cli::{printer::Printer, prompt, spinner::Spinner};
use pimalaya_config::toml as config_toml;
use serde::{Serialize, Serializer};
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
#[cfg(any(feature = "gmail", feature = "msgraph"))]
use crate::wizard::mailbox;
#[cfg(feature = "msgraph")]
use crate::wizard::msgraph;
use crate::{
    account::check,
    config::{AccountConfig, Config},
    wizard::search::{self, Discovered, DiscoveredKind},
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

/// Runs the wizard and prints the resulting [`Config`] as a
/// ready-to-save TOML document, writing nothing to disk. Run on bare
/// `himalaya`, and proposed by `cli::resolve_account` on first run.
pub fn run(printer: &mut impl Printer) -> Result<()> {
    let input = prompt::text::<&str>(ENDPOINT_PROMPT, None)?;
    let input = input.trim();
    if input.is_empty() {
        bail!("Empty input: enter an email address, a server URL, or a folder path");
    }

    // NOTE: the account name is just the TOML table key, so it is derived
    // from the input rather than prompted; the user renames it by hand.
    let account_name = default_account_name(input);
    let (account, tested) = build_account(&account_name, input)?;

    // Test the account before printing it: a bad credential or endpoint
    // fails here and stops the process, like any other error, rather
    // than emitting a config that cannot connect. The IMAP+SMTP flow
    // already tests each protocol as it configures them, so skip the
    // redundant round-trip in that case.
    if !tested {
        let spinner = Spinner::start("Testing account configuration");
        if let Err(err) = check::test_account(&account) {
            spinner.failure("Account configuration test failed");
            return Err(err);
        }
        spinner.success("Account configuration is valid");
    }

    let config = Config {
        accounts: HashMap::from([(account_name, account)]),
        ..Default::default()
    };

    printer.out(GeneratedConfig(config))
}

/// The account produced by the wizard, printed as a ready-to-save TOML
/// document on stdout with its guidance embedded as comments, or the
/// same config serialized as an object in JSON mode. The wizard writes
/// nothing itself: the user redirects the output into their config file
/// (e.g. `himalaya > <config>`), so prompts go to stderr and only this
/// lands on stdout.
struct GeneratedConfig(Config);

impl fmt::Display for GeneratedConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let toml = config_toml::to_string(&self.0).map_err(|_| fmt::Error)?;

        writeln!(f, "# Configuration generated by the himalaya wizard.")?;
        writeln!(f, "#")?;
        writeln!(
            f,
            "# Nothing was written to disk: save this into your config"
        )?;
        writeln!(f, "# file, one of:")?;
        writeln!(f, "#   $XDG_CONFIG_HOME/himalaya/config.toml")?;
        writeln!(f, "#   $HOME/.config/himalaya/config.toml")?;
        writeln!(f, "#   $HOME/.himalayarc")?;
        writeln!(f, "#")?;
        writeln!(
            f,
            "# Prompts render on stderr, so redirecting works directly:"
        )?;
        writeln!(f, "#   himalaya > ~/.config/himalaya/config.toml")?;
        writeln!(f, "#")?;
        writeln!(
            f,
            "# The account name (the [accounts.*] table key) is derived"
        )?;
        writeln!(f, "# from your input; rename it to anything you like.")?;
        writeln!(f, "#")?;
        writeln!(f, "# Every field is documented in the sample config:")?;
        writeln!(
            f,
            "# https://github.com/pimalaya/himalaya/blob/master/config.sample.toml"
        )?;
        writeln!(f)?;
        write!(f, "{toml}")
    }
}

impl Serialize for GeneratedConfig {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

/// The result of a configure flow: the chosen backend, whether it
/// already validated its connections (so the caller skips the final
/// account test), and any `mailbox.alias.*` entries discovered from the
/// server.
struct Outcome {
    chosen: Chosen,
    tested: bool,
    aliases: HashMap<String, String>,
}

impl Outcome {
    /// A not-yet-tested outcome with no discovered aliases, for the flows
    /// that defer validation to the final account test (manual entry, the
    /// proprietary APIs, local backends).
    fn untested(chosen: Chosen) -> Self {
        Self {
            chosen,
            tested: false,
            aliases: HashMap::new(),
        }
    }
}

/// Orients the setup from the input shape, then folds the chosen
/// backend into a fresh [`AccountConfig`]. The returned flag reports
/// whether the flow already validated its connections (the IMAP+SMTP and
/// JMAP paths do), so the caller can skip the final account test.
///
/// The account is left non-default so it does not hijack the default
/// when the wizard's output is merged into a config that already has
/// one. Being false, `default` is omitted from the printed TOML; the
/// user marks their choice with `default = true`.
fn build_account(account_name: &str, input: &str) -> Result<(AccountConfig, bool)> {
    let Outcome {
        chosen,
        tested,
        aliases,
    } = if is_path(input) {
        Outcome::untested(configure_local(input)?)
    } else if input.contains("://") {
        Outcome::untested(configure_server(account_name, input)?)
    } else {
        configure_email(account_name, input)?
    };

    let mut account = AccountConfig {
        default: false,
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

    // NOTE: the discovered special-use aliases (e.g. the default `inbox`)
    // let shared commands resolve a mailbox without hand-editing ids;
    // empty for the flows that discover none.
    account.mailbox.aliases = aliases;

    Ok((account, tested))
}

/// Runs the email-driven discovery flow: search the services reachable
/// from the address, let the user pick one, then configure its backend
/// (the authentication method is picked in a second, service-specific
/// prompt). Falls back to manual IMAP+SMTP entry when nothing is found.
fn configure_email(account_name: &str, input: &str) -> Result<Outcome> {
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
        return Ok(Outcome::untested(manual_fallback(account_name, &email)?));
    }
    spinner.success(format!("Found {} configuration(s)", found.len()));

    let default = found.first().cloned();
    let choice = prompt::item("Choose a configuration:", found, default)?;

    dispatch(account_name, &email, choice)
}

/// Configures the backend behind a discovered entry. The IMAP+SMTP and
/// JMAP flows test their connections inline (marking the outcome tested)
/// and discover their `mailbox.alias.*` on the same session; the others
/// defer to the final account test and discover no aliases.
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
fn dispatch(account_name: &str, email: &str, choice: Discovered) -> Result<Outcome> {
    match &choice.kind {
        #[cfg(all(feature = "imap", feature = "smtp"))]
        DiscoveredKind::ImapSmtp { .. } => {
            let (imap, smtp, aliases) =
                imap_smtp::configure_discovered(account_name, email, &choice)?;
            Ok(Outcome {
                chosen: Chosen::ImapSmtp(Box::new(imap), Box::new(smtp)),
                tested: true,
                aliases,
            })
        }
        #[cfg(feature = "jmap")]
        DiscoveredKind::Jmap(_) => {
            let (jmap, aliases) = jmap::configure_discovered(account_name, email, &choice)?;
            Ok(Outcome {
                chosen: Chosen::Jmap(Box::new(jmap)),
                tested: true,
                aliases,
            })
        }
        // NOTE: Gmail and Graph expose special-use mailboxes through fixed
        // platform contracts (system-label ids / well-known names), so
        // their aliases are pinned without a live listing; the connection
        // is still validated by the final account test.
        #[cfg(feature = "gmail")]
        DiscoveredKind::Gmail => Ok(Outcome {
            chosen: Chosen::Gmail(gmail::configure(account_name)?),
            tested: false,
            aliases: mailbox::gmail_aliases(),
        }),
        #[cfg(feature = "msgraph")]
        DiscoveredKind::Msgraph => Ok(Outcome {
            chosen: Chosen::Msgraph(msgraph::configure(account_name)?),
            tested: false,
            aliases: mailbox::msgraph_aliases(),
        }),
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
        "http" | "https" | "jmap" | "jmaps" => Ok(Chosen::Jmap(Box::new(jmap::configure_manual(
            account_name,
            input,
            None,
        )?))),
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

/// Proposes a default account name from the input shape: the first
/// label of the domain (of an email, host, or bare domain), or the
/// folder name of a local path.
fn default_account_name(input: &str) -> String {
    if is_path(input) {
        let raw = input.strip_prefix("file://").unwrap_or(input);
        return Path::new(raw)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("personal")
            .to_string();
    }

    if let Ok(url) = Url::parse(input)
        && let Some(host) = url.host_str()
    {
        return first_label(host);
    }

    match input.rsplit_once('@') {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_name_defaults_to_the_first_domain_label() {
        // Email: the domain's first label, never the local part.
        assert_eq!(default_account_name("clement.douin@posteo.net"), "posteo");
        assert_eq!(default_account_name("alice@mail.example.co.uk"), "mail");
        // Bare domain (as discovery synthesizes it) and plain domain.
        assert_eq!(default_account_name("@posteo.net"), "posteo");
        assert_eq!(default_account_name("posteo.net"), "posteo");
    }

    #[test]
    fn account_name_defaults_to_the_last_path_component() {
        assert_eq!(
            default_account_name("/home/alice/mail/personal"),
            "personal"
        );
        assert_eq!(default_account_name("~/mail/work"), "work");
        assert_eq!(default_account_name("file:///var/mail/archive"), "archive");
    }

    #[test]
    fn discovered_aliases_render_as_a_mailbox_alias_table() {
        let mut account = AccountConfig::default();
        account
            .mailbox
            .aliases
            .insert("inbox".to_string(), "INBOX".to_string());

        let config = Config {
            accounts: HashMap::from([("posteo".to_string(), account)]),
            ..Default::default()
        };
        let rendered = GeneratedConfig(config).to_string();

        assert!(rendered.contains("[accounts.posteo]"));
        assert!(rendered.contains("mailbox.alias.inbox = \"INBOX\""));
    }
}
