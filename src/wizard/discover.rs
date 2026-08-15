//! Account discovery, the half of the wizard that decides what the
//! account is.
//!
//! What becomes of the discovered account, a file to create, a block to
//! append or a document on stdout, belongs to [`super::configure`],
//! which is also where the welcome and the prompts around this one live.
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

use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result, bail};
#[cfg(all(feature = "imap", feature = "smtp"))]
use io_pim_discovery::compose::config::DiscoverySecurity;
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
#[cfg(any(feature = "gmail", feature = "msgraph"))]
use crate::wizard::mailbox;
#[cfg(feature = "msgraph")]
use crate::wizard::msgraph;
use crate::{
    account::check,
    config::AccountConfig,
    wizard::search::{self, Discovered, DiscoveredKind},
};

/// The documented sample configuration, shown in the welcome banner and
/// pointed at when discovery finds nothing to configure automatically.
pub const CONFIG_SAMPLE_URL: &str =
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

/// Discovers one account from a single prompt, tests it, and hands back
/// the name it proposes with the account itself.
///
/// What happens to that account, written to a file, appended to one or
/// printed, belongs to [`super::configure`], which is also where the
/// welcome lives: this is the discovery half alone.
pub fn run() -> Result<(String, AccountConfig)> {
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

    Ok((account_name, account))
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
/// The account is left non-default here. Whether it claims the default
/// depends on what the configuration already holds, which discovery does
/// not read, so [`super::configure`] decides it.
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

        let rendered = account.render("posteo").expect("render the account");

        assert!(rendered.contains("[accounts.posteo]"));
        assert!(rendered.contains("mailbox.alias.inbox = \"INBOX\""));
    }
}
