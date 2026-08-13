//! Configuration wizard.
//!
//! Run on bare `himalaya` (no subcommand), and proposed by
//! `cli::resolve_account` when no config file is found. It opens with a
//! welcome banner on stderr, then either saves the resulting account to
//! a config file (offered when writing to a terminal) or prints it as a
//! ready-to-save TOML document on stdout, so `himalaya > <config>` still
//! works as the write-back when stdout is redirected, like ortie.
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
//! - a `scheme://` URL discovers from its host, its scheme narrowing the
//!   results (`imap(s)` to IMAP + SMTP, an HTTP-family scheme to JMAP);
//! - an existing folder is a local Maildir or m2dir.
//!
//! The wizard only configures what it can discover automatically. When
//! discovery finds nothing for the given input it stops and points at the
//! documented sample, rather than prompting for a hand-entered config.
//!
//! Himalaya runs no OAuth 2.0 grant itself: a grant only unlocks the
//! external token brokers (Ortie, pizauth, oama) behind the API token
//! credential prompt (see [`super::secret`]).

use std::{collections::HashMap, fmt, fs, io::IsTerminal, path::Path};

use anyhow::{Context, Result, bail};
#[cfg(all(feature = "imap", feature = "smtp"))]
use io_pim_discovery::compose::config::DiscoverySecurity;
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

/// The documented sample configuration, shown in the welcome banner and
/// pointed at when discovery finds nothing to configure automatically.
const CONFIG_SAMPLE_URL: &str =
    "https://github.com/pimalaya/himalaya/blob/master/config.sample.toml";

/// The backend config produced by the chosen flow, folded into a fresh
/// [`AccountConfig`] afterwards.
enum Chosen {
    #[cfg(all(feature = "imap", feature = "smtp"))]
    ImapSmtp(Box<ImapConfig>, Option<Box<SmtpConfig>>),
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

/// Runs the wizard and either saves the resulting [`Config`] to a file
/// or prints it as a ready-to-save TOML document. Run on bare
/// `himalaya`, and proposed by `cli::resolve_account` on first run.
///
/// A welcome message renders on stderr first (skipped in JSON mode) to
/// frame what Himalaya is and what the wizard does. The generated
/// config is then offered for saving when writing to a terminal; when
/// stdout is redirected (`himalaya > config.toml`) or in JSON mode it is
/// emitted straight to stdout so the redirect / script keeps working.
pub fn run(printer: &mut impl Printer) -> Result<()> {
    if !printer.is_json() {
        print_welcome();
    }

    let input = prompt::text("Email:", None)?;
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

    // JSON mode and a redirected stdout stay non-interactive: emit the
    // document straight to stdout so scripts and `himalaya > config.toml`
    // keep working. Only offer to save when writing to a terminal.
    if printer.is_json() || !std::io::stdout().is_terminal() {
        return printer.out(GeneratedConfig(config));
    }

    save_or_print(printer, config)
}

/// Prints a welcome banner on stderr framing the project and the wizard,
/// so bare `himalaya` explains itself before dropping into prompts. On
/// stderr so it never pollutes a redirected config document.
fn print_welcome() {
    println!();
    eprintln!("Welcome to Himalaya, the CLI to manage emails.");
    eprintln!();
    eprintln!("Himalaya talks to your existing mailbox over IMAP, JMAP, Gmail,");
    eprintln!("Microsoft Graph or a local Maildir. Before you can read or send");
    eprintln!("mail, it needs to know about one account.");
    eprintln!();
    eprintln!("This wizard discovers a provider's settings from your email address");
    eprintln!("(or a server URL, or a local folder path), tests the connection and");
    eprintln!("generates a ready-to-use configuration it can save for you.");
    eprintln!();
    eprintln!("Every field is documented in the sample configuration:");
    eprintln!("  {CONFIG_SAMPLE_URL}");
    eprintln!();
}

/// Offers to save the generated config to a file (default
/// `$XDG_CONFIG_HOME/himalaya/config.toml`), falling back to printing it
/// on stdout when the user declines or an existing file must not be
/// overwritten. Prompts and confirmations render on stderr.
fn save_or_print(printer: &mut impl Printer, config: Config) -> Result<()> {
    if !prompt::bool("Save this configuration to a file, or print it?", true)? {
        return printer.out(GeneratedConfig(config));
    }

    let default = default_config_path();
    let path = prompt::text("Configuration file path:", default.as_deref())?;
    let path = shellexpand::full(path.trim())?.into_owned();
    let path = Path::new(&path);

    // Bare `himalaya` runs the wizard even when a config already exists,
    // so guard the default path: never clobber without confirmation, and
    // fall back to printing so the generated config is never lost.
    if path.exists()
        && !prompt::bool(
            format!("`{}` already exists. Overwrite it?", path.display()),
            false,
        )?
    {
        return printer.out(GeneratedConfig(config));
    }

    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Create config directory `{}`", parent.display()))?;
    }

    fs::write(path, GeneratedConfig(config).to_string())
        .with_context(|| format!("Write config file `{}`", path.display()))?;

    eprintln!();
    eprintln!("Configuration saved to {}.", path.display());
    eprintln!("Run `himalaya envelope list` to read your mailbox.");
    Ok(())
}

/// The default config path (`$XDG_CONFIG_HOME/himalaya/config.toml`),
/// used to seed the save prompt; `None` when no config dir resolves.
fn default_config_path() -> Option<String> {
    let path = dirs::config_dir()?
        .join(env!("CARGO_PKG_NAME"))
        .join("config.toml");
    Some(path.to_string_lossy().into_owned())
}

/// The account produced by the wizard, rendered as a ready-to-save TOML
/// document (for a file write or stdout), or serialized as an object in
/// JSON mode. The framing that used to head this document as comments
/// now lives in the stderr welcome banner, so what lands here is the
/// bare config, whether it is saved to a file or redirected on stdout.
struct GeneratedConfig(Config);

impl fmt::Display for GeneratedConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let toml = config_toml::to_string(&self.0).map_err(|_| fmt::Error)?;
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
    } else {
        configure_discovery(account_name, input)?
    };

    let mut account = AccountConfig {
        default: false,
        ..Default::default()
    };

    match chosen {
        #[cfg(all(feature = "imap", feature = "smtp"))]
        Chosen::ImapSmtp(imap, smtp) => {
            account.imap = Some(*imap);
            account.smtp = smtp.map(|smtp| *smtp);
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

/// Runs the discovery flow for an email, a bare domain, or a
/// `scheme://` server URL: search the services reachable from it, keep
/// only those supported by this build (and matching the URL scheme when
/// one was given), let the user pick one, then configure its backend
/// (the authentication method is picked in a second, service-specific
/// prompt). When nothing is discovered the wizard stops rather than
/// prompting for a hand-entered config (see [`stop_undiscovered`]).
fn configure_discovery(account_name: &str, input: &str) -> Result<Outcome> {
    // A `scheme://host` URL discovers from its host, and its scheme
    // narrows the results; an email or bare domain discovers from the
    // domain with no scheme filter.
    let (email, scheme) = if input.contains("://") {
        let url = Url::parse(input).with_context(|| format!("Invalid server URL `{input}`"))?;
        let host = url.host_str().unwrap_or_default().to_string();
        (format!("@{host}"), Some(url.scheme().to_string()))
    } else if input.contains('@') {
        (input.to_string(), None)
    } else {
        (format!("@{input}"), None)
    };

    let spinner = Spinner::start("Searching for server settings");
    let mut found = search::search(&email)?;
    retain_supported(&mut found);
    if let Some(scheme) = &scheme {
        retain_scheme(&mut found, scheme)?;
    }

    if found.is_empty() {
        spinner.failure("No configuration found");
        return stop_undiscovered(input);
    }
    spinner.success(format!("Found {} configuration(s)", found.len()));

    let default = found.first().cloned();
    let choice = prompt::item("Choose a configuration:", found, default)?;

    dispatch(account_name, &email, choice)
}

/// Keeps only the discovered entries a `scheme://` URL asked for: `imap`
/// and `imaps` keep IMAP + SMTP (with `imaps` requiring an implicit-TLS
/// IMAP endpoint), and the HTTP-family schemes keep JMAP. A proprietary
/// entry (Gmail, Graph) is dropped, since the user named an open
/// protocol. An unknown scheme is rejected outright.
fn retain_scheme(found: &mut Vec<Discovered>, scheme: &str) -> Result<()> {
    match scheme {
        #[cfg(all(feature = "imap", feature = "smtp"))]
        "imap" | "imaps" => {
            let tls_only = scheme == "imaps";
            found.retain(|entry| match &entry.kind {
                DiscoveredKind::ImapSmtp { imap, .. } => {
                    !tls_only || imap.security == DiscoverySecurity::Tls
                }
                _ => false,
            });
        }
        "jmap" | "jmaps" | "http" | "https" => {
            found.retain(|entry| matches!(entry.kind, DiscoveredKind::Jmap(_)));
        }
        other => bail!("Unsupported server scheme `{other}`"),
    }

    Ok(())
}

/// Stops the wizard when discovery found nothing to configure for
/// `input`: it prints where to go next (a hand-written config, seeded
/// from the documented sample) and errors out, rather than dropping into
/// a hand-entry flow. Himalaya's wizard only ever configures what it can
/// discover automatically.
fn stop_undiscovered(input: &str) -> Result<Outcome> {
    bail!(
        "Could not automatically discover a configuration for `{input}`.\n\n\
         Write your account configuration by hand instead, starting from the \
         documented sample:\n  {CONFIG_SAMPLE_URL}"
    )
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
                chosen: Chosen::ImapSmtp(Box::new(imap), smtp.map(Box::new)),
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
