//! # Discovery
//!
//! The half of the wizard deciding what the account is, what becomes of
//! it belonging to [`super::configure`].
//!
//! One prompt takes an email address, a server URL or a local folder
//! path, and its shape orients the setup. An address runs the parallel
//! discovery of [`super::search`], every reachable service becoming one
//! selectable configuration whose advertised authentication is then
//! prompted for.
//!
//! A URL discovers from its host, the scheme narrowing the results, and
//! an existing folder is a local Maildir or m2dir. A Google or Microsoft
//! account collapses to its own set of configurations.
//!
//! The wizard configures what it discovers and nothing else: finding
//! nothing, it stops and points at the sample rather than prompt for a
//! hand-written configuration.
//!
//! Himalaya runs no OAuth 2.0 grant of its own. A grant unlocks the
//! external token brokers behind the API token prompt, and that is all.

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
/// the name it proposes beside the account itself.
///
/// What becomes of that account belongs to [`super::configure`]. This is
/// the discovery half alone.
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

    // NOTE: testing before printing is what stops a bad credential or
    // endpoint from becoming a configuration that cannot connect. The
    // IMAP and SMTP flow already tested each side as it configured them.
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

/// Orients the setup from the input shape, then folds the chosen backend
/// into a fresh [`AccountConfig`].
///
/// The returned flag says whether the flow already validated its
/// connections, so the caller can skip the final test.
///
/// The account is left non-default: whether it claims the default depends
/// on what the configuration already holds, which discovery never reads.
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

    // NOTE: the discovered special-use aliases are what lets a shared
    // command resolve a mailbox without anyone hand-editing ids.
    account.mailbox.aliases = aliases;

    // NOTE: the prompt may already have been answered with an address,
    // so the composers get their `From` without it being typed twice.
    account.email = prompted_email(input).map(ToString::to_string);

    Ok((account, tested))
}

/// Runs the discovery flow for an email, a bare domain or a server URL.
///
/// The services reachable from it are searched, narrowed to what this
/// build supports and what the scheme allows, then one is picked and its
/// backend configured, the authentication method being a second prompt.
///
/// Finding nothing, the wizard stops rather than prompt for a
/// hand-written configuration.
fn configure_discovery(account_name: &str, input: &str) -> Result<Outcome> {
    // NOTE: a URL discovers from its host and narrows on its scheme,
    // where an email or a bare domain discovers from the domain alone.
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

/// Keeps only the discovered entries the URL scheme asked for.
///
/// `imap` and `imaps` keep IMAP and SMTP, `imaps` wanting an
/// implicit-TLS endpoint, and an HTTP-family scheme keeps JMAP. A
/// proprietary entry is dropped, the user having named an open protocol,
/// and an unknown scheme is refused.
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

/// Stops the wizard when discovery found nothing for the input, pointing
/// at the sample a hand-written configuration is seeded from.
///
/// The wizard configures what it discovers and nothing else, so there is
/// no hand-entry flow to drop into.
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
        // NOTE: Gmail and Graph name their special-use mailboxes through
        // platform contracts, so the aliases are pinned without a live
        // listing and the final account test still runs.
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

/// The input read back as an email address, `None` when it names a
/// folder, a server URL or a bare domain instead.
///
/// A server URL may carry a userinfo part and so an `@` of its own, which
/// is a credential rather than an address, hence the scheme check first.
fn prompted_email(input: &str) -> Option<&str> {
    if is_path(input) || input.contains("://") {
        return None;
    }

    let (local, domain) = input.rsplit_once('@')?;

    if local.is_empty() || domain.is_empty() {
        return None;
    }

    Some(input)
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
        assert_eq!(default_account_name("clement.douin@posteo.net"), "posteo");
        assert_eq!(default_account_name("alice@mail.example.co.uk"), "mail");
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
    fn only_an_address_is_kept_as_the_account_email() {
        assert_eq!(
            prompted_email("alice@example.org"),
            Some("alice@example.org")
        );

        // NOTE: a bare domain names no mailbox, and neither does a folder
        // or a server URL, whose `@` would be a credential.
        assert_eq!(prompted_email("@example.org"), None);
        assert_eq!(prompted_email("example.org"), None);
        assert_eq!(prompted_email("~/mail/work"), None);
        assert_eq!(prompted_email("imaps://alice@imap.example.org"), None);
    }

    #[test]
    fn discovered_aliases_render_as_a_mailbox_alias_table() {
        let mut account = AccountConfig {
            email: Some("me@posteo.net".to_string()),
            ..Default::default()
        };
        account
            .mailbox
            .aliases
            .insert("inbox".to_string(), "INBOX".to_string());

        let rendered = account.render("posteo").expect("render the account");

        assert!(rendered.contains("[accounts.posteo]"));
        assert!(rendered.contains("mailbox.alias.inbox = \"INBOX\""));

        // NOTE: the identity says what the account is, so it reads before
        // the mailboxes it names.
        let email = rendered.find("email = ").expect("the address is rendered");
        let alias = rendered
            .find("mailbox.alias")
            .expect("the alias is rendered");
        assert!(email < alias);
    }
}
