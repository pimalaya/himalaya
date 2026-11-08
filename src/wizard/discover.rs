//! # Discovery
//!
//! The half of the wizard deciding what the account is, what becomes of
//! it belonging to [`crate::wizard::configure`].
//!
//! One prompt takes an email address, a server URL or a local folder
//! path, and nothing else is asked before discovery. An address looks the
//! account up, and each domain, mail and sending, gets the best option
//! found without a question ([`crate::wizard::plan`]). The review shows
//! the picks in plain words and changes one on demand; the sign-ins
//! follow, grouped by identity ([`crate::wizard::signin`]), then the
//! display name.
//!
//! A URL discovers from its host, the scheme narrowing the options, and
//! an existing folder is a local Maildir or m2dir. Finding nothing asks
//! for a server URL, an empty answer stopping the wizard.

use std::path::Path;

use anyhow::{Context, Result, bail};
use percent_encoding::percent_decode_str;
use pimalaya_cli::{
    prompt,
    spinner::Spinner,
    wizard::{
        keyring::TokenBroker,
        review::{self, ReviewAction, ReviewRow},
    },
};
use url::Url;

#[cfg(any(feature = "maildir", feature = "m2dir"))]
use crate::wizard::local;
use crate::{
    account::check,
    config::{AccountConfig, CONFIG_SAMPLE_URL},
    wizard::{
        plan::{Change, Domain, Endpoint, Pick, Picks, Plan, Protocol, Route, Setup, SignIn},
        search,
        server::TypedServer,
        signin::{Backend, Identity, Outcome, SignIns},
    },
};

/// Discovers one account from a single prompt, signs it in, and hands it
/// back under the name `free_name` makes of the one the input suggests.
///
/// The name is settled before any sign-in, the keyring entries and the
/// Ortie account carrying it. The account is left non-default: whether it
/// claims the default depends on what the configuration already holds,
/// which discovery never reads.
pub fn run(free_name: impl FnOnce(&str) -> String) -> Result<(String, AccountConfig)> {
    let input = prompt::text("Email:", None)?;
    let input = input.trim();
    if input.is_empty() {
        bail!("Empty input: enter an email address, a server URL, or a folder path");
    }

    let name = free_name(&default_account_name(input));

    let mut account = if is_path(input) {
        let account = local_account(input)?;

        let spinner = Spinner::start("Testing account configuration");
        if let Err(err) = check::test_account(&account) {
            spinner.failure("Account configuration test failed");
            return Err(err);
        }
        spinner.success("Account configuration is valid");

        account
    } else {
        remote_account(&name, input)?
    };

    // The prompt may already have been answered with an address, so the
    // composers get their `From` without it being typed twice.
    account.email = prompted_email(input).map(ToString::to_string);

    Ok((name, account))
}

/// What the first prompt was answered with, read for discovery.
struct Target {
    /// The answer, as typed.
    input: String,
    /// What discovery looks up: the address, or `@<domain>`.
    lookup: String,
    /// The domain looked up.
    domain: String,
    /// The typed address.
    address: Option<String>,
    /// The typed server URL.
    url: Option<String>,
    /// The scheme of the typed server URL, narrowing the mail options.
    scheme: Option<String>,
    /// The user name of the typed server URL.
    login: Option<String>,
}

impl Target {
    /// Reads an address, a bare domain or a server URL, refusing a scheme
    /// that names no mail server.
    fn parse(input: &str) -> Result<Self> {
        if !input.contains("://") {
            let address = prompted_email(input).map(ToString::to_string);
            let domain = match input.rsplit_once('@') {
                Some((_, domain)) => domain,
                None => input,
            };
            let domain = domain.trim_end_matches('.').to_lowercase();

            return Ok(Self {
                input: input.to_string(),
                lookup: address.clone().unwrap_or_else(|| format!("@{domain}")),
                domain,
                address,
                url: None,
                scheme: None,
                login: None,
            });
        }

        let url = Url::parse(input).with_context(|| format!("Invalid server URL `{input}`"))?;
        let scheme = url.scheme().to_ascii_lowercase();

        if !matches!(
            scheme.as_str(),
            "imap" | "imaps" | "jmap" | "jmaps" | "http" | "https"
        ) {
            bail!("Unsupported server scheme `{scheme}`");
        }

        let domain = url
            .host_str()
            .unwrap_or_default()
            .trim_end_matches('.')
            .to_lowercase();

        let login = Some(
            percent_decode_str(url.username())
                .decode_utf8_lossy()
                .into_owned(),
        )
        .filter(|login| !login.is_empty());

        Ok(Self {
            input: input.to_string(),
            lookup: format!("@{domain}"),
            domain,
            address: None,
            url: Some(input.to_string()),
            scheme: Some(scheme),
            login,
        })
    }

    /// Looks the target up under a spinner, answering the options this
    /// build can set up, the mail ones narrowed to a typed URL's scheme.
    ///
    /// A resolver that does not answer reads as nothing discovered, the
    /// reason shown.
    fn look_up(&self, ortie: bool) -> Plan {
        let spinner = Spinner::start(format!("Looking up {}", self.domain));

        let mut plan = match search::search(&self.lookup) {
            Ok(configs) => Plan::new(&configs, ortie),
            Err(err) => {
                spinner.failure(format!("Cannot look up {}: {err:#}", self.domain));

                return Plan {
                    ortie,
                    ..Default::default()
                };
            }
        };

        plan.mail.retain(|route| route.protocol.compiled());
        plan.sending.retain(|route| route.protocol.compiled());

        match self.scheme.as_deref() {
            Some(scheme @ ("imap" | "imaps")) => plan.mail.retain(|route| {
                route.protocol == Protocol::Imap
                    && (scheme == "imap"
                        || matches!(route.endpoint, Endpoint::Tcp { tls: true, .. }))
            }),
            Some(_) => plan.mail.retain(|route| route.protocol == Protocol::Jmap),
            None => {}
        }

        if plan.mail.is_empty() && plan.sending.is_empty() {
            spinner.failure(format!("Nothing could be discovered for {}", self.domain));
        } else {
            spinner.success(format!("Found settings for {}", self.domain));
        }

        plan
    }

    /// The mail server when discovery found none: the typed URL itself,
    /// else the one asked for, an empty answer stopping the wizard.
    fn mail_server(&self, plan: &Plan) -> Result<Route> {
        match &self.url {
            Some(url) => match TypedServer::parse(url, Domain::Mail) {
                Ok(server) => return Ok(server.route(ask_sign_in(SignIn::Password)?)),
                Err(err) => eprintln!("{err}"),
            },
            None if !plan.sending.is_empty() => {
                eprintln!("No mail server could be discovered for {}", self.domain);
            }
            None => {}
        }

        match ask_server("Server URL:", Domain::Mail, SignIn::Password)? {
            Some(route) => Ok(route),
            None => bail!(
                "Could not automatically discover a configuration for `{}`.\n\n\
                 Write your account configuration by hand instead, starting from the \
                 documented sample:\n  {CONFIG_SAMPLE_URL}",
                self.input
            ),
        }
    }

    /// The domain a server off it is flagged against: the address's, or
    /// the bare domain typed, none for a typed server URL.
    fn flagged_domain(&self) -> Option<String> {
        match self.url {
            Some(_) => None,
            None => Some(self.domain.clone()),
        }
    }
}

/// Looks the account up from an address, a bare domain or a server URL,
/// reviews the picks, signs them in, then asks the display name.
fn remote_account(name: &str, input: &str) -> Result<AccountConfig> {
    let target = Target::parse(input)?;
    let plan = target.look_up(TokenBroker::Ortie.installed());
    let mut picks = Picks::default();

    if plan.mail.is_empty() {
        let route = target.mail_server(&plan)?;

        if route.protocol == Protocol::Imap && plan.sending.is_empty() {
            picks.sending = ask_sending(route.sign_in)?;
        }

        picks.mail = Pick::Custom(route);
    }

    let identity = Identity {
        name: name.to_string(),
        address: target.address.clone(),
        login: target.login.clone(),
        domain: target.flagged_domain(),
    };

    let mut sign_ins = SignIns::default();

    let backends = loop {
        let mail = plan.mail(&picks.mail);
        let sending = plan.sending(&picks.sending, &mail);

        let rows = [
            ReviewRow {
                name: Domain::Mail.name().to_string(),
                line: plan.line(&mail, None),
            },
            ReviewRow {
                name: Domain::Sending.name().to_string(),
                line: plan.line(&sending, mail.route()),
            },
        ];

        eprintln!();

        match review::prompt(&rows)? {
            ReviewAction::Continue if mail == Setup::Off && sending == Setup::Off => {
                eprintln!("Set up mail or sending to continue");
            }
            ReviewAction::Continue => {
                let setups = [(Domain::Mail, mail), (Domain::Sending, sending)];

                if let Outcome::Done(backends) = sign_ins.run(&identity, &setups, &mut picks)? {
                    break backends;
                }
            }
            ReviewAction::Change(0) => change(&plan, Domain::Mail, &mut picks)?,
            ReviewAction::Change(_) => change(&plan, Domain::Sending, &mut picks)?,
        }
    };

    let mut account = AccountConfig::default();

    for backend in backends {
        match backend {
            #[cfg(feature = "imap")]
            Backend::Imap(config) => account.imap = Some(*config),
            #[cfg(feature = "smtp")]
            Backend::Smtp(config) => account.smtp = Some(*config),
            #[cfg(feature = "jmap")]
            Backend::Jmap(config) => account.jmap = Some(*config),
            #[cfg(feature = "gmail")]
            Backend::Gmail(config) => account.gmail = Some(*config),
            #[cfg(feature = "msgraph")]
            Backend::Msgraph(config) => account.msgraph = Some(*config),
        }
    }

    let display_name = prompt::text("Your name (optional):", None)?;
    account.display_name = Some(display_name.trim().to_string()).filter(|name| !name.is_empty());

    Ok(account)
}

/// Changes the pick of `domain` among the entries the plan offers, the
/// current one selected.
fn change(plan: &Plan, domain: Domain, picks: &mut Picks) -> Result<()> {
    let items = plan.changes(domain, picks);
    let current = Change::Pick(picks.get(domain).clone());
    let default = items.iter().find(|item| item.change == current).cloned();

    let item = prompt::item(format!("{}:", domain.name()), items, default)?;

    let route = match item.change {
        Change::Pick(pick) => {
            *picks.get_mut(domain) = pick;
            return Ok(());
        }
        Change::Custom => {
            let (label, sign_in) = match domain {
                Domain::Mail => ("Server URL:", SignIn::Password),
                Domain::Sending => {
                    let mail = plan.mail(&picks.mail);
                    let sign_in = mail.route().map_or(SignIn::Password, |route| route.sign_in);
                    ("Sending server URL:", sign_in)
                }
            };

            match ask_server(label, domain, sign_in)? {
                Some(route) => route,
                None => return Ok(()),
            }
        }
    };

    let ask = domain == Domain::Mail
        && route.protocol == Protocol::Imap
        && plan.sending.is_empty()
        && !matches!(picks.sending, Pick::Custom(_));

    if ask {
        picks.sending = ask_sending(route.sign_in)?;
    }

    *picks.get_mut(domain) = Pick::Custom(route);

    Ok(())
}

/// Asks for a server URL for `domain` until one reads, then its sign-in
/// method, `sign_in` selected first; `None` on an empty answer.
fn ask_server(label: &str, domain: Domain, sign_in: SignIn) -> Result<Option<Route>> {
    loop {
        let input = prompt::text(label, None)?;
        let input = input.trim();

        if input.is_empty() {
            return Ok(None);
        }

        match TypedServer::parse(input, domain) {
            Ok(server) => return Ok(Some(server.route(ask_sign_in(sign_in)?))),
            Err(err) => eprintln!("{err}"),
        }
    }
}

/// Asks for the sending server after a typed IMAP one, an empty answer
/// leaving sending out.
fn ask_sending(sign_in: SignIn) -> Result<Pick> {
    let route = ask_server(
        "Sending server URL (empty for none):",
        Domain::Sending,
        sign_in,
    )?;

    Ok(match route {
        Some(route) => Pick::Custom(route),
        None => Pick::Off,
    })
}

/// Asks how a typed server signs in, a password or an API token,
/// `default` selected first.
fn ask_sign_in(default: SignIn) -> Result<SignIn> {
    let default = match default {
        SignIn::Oauth => SignIn::Password,
        default => default,
    };

    Ok(prompt::item(
        "Sign in with:",
        [SignIn::Password, SignIn::Token],
        Some(default),
    )?)
}

/// Configures a local backend from a typed folder path.
#[cfg(any(feature = "maildir", feature = "m2dir"))]
fn local_account(input: &str) -> Result<AccountConfig> {
    let raw = input.strip_prefix("file://").unwrap_or(input);
    let root = shellexpand::tilde(raw).into_owned();
    if !Path::new(&root).is_dir() {
        bail!("No such folder `{raw}`");
    }

    let mut account = AccountConfig::default();

    match local::configure(root.into())? {
        #[cfg(feature = "maildir")]
        local::Local::Maildir(config) => account.maildir = Some(config),
        #[cfg(feature = "m2dir")]
        local::Local::M2dir(config) => account.m2dir = Some(config),
    }

    Ok(account)
}

/// Refuses a folder path, no local backend being compiled in.
#[cfg(not(any(feature = "maildir", feature = "m2dir")))]
fn local_account(input: &str) -> Result<AccountConfig> {
    bail!("`{input}` looks like a folder path, but no local backend is compiled in")
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
    use crate::{config::AccountConfig, wizard::discover::*};

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

        // A bare domain names no mailbox, and neither does a folder or a
        // server URL, whose `@` would be a credential.
        assert_eq!(prompted_email("@example.org"), None);
        assert_eq!(prompted_email("example.org"), None);
        assert_eq!(prompted_email("~/mail/work"), None);
        assert_eq!(prompted_email("imaps://alice@imap.example.org"), None);
    }

    #[test]
    fn the_input_says_what_discovery_looks_up() {
        let address = Target::parse("jane@Example.org").expect("an address");
        assert_eq!(address.lookup, "jane@Example.org");
        assert_eq!(address.domain, "example.org");
        assert_eq!(address.flagged_domain().as_deref(), Some("example.org"));

        let domain = Target::parse("example.org").expect("a bare domain");
        assert_eq!(domain.lookup, "@example.org");
        assert_eq!(domain.address, None);

        let url = Target::parse("imaps://jane@Mail.Example.org:993").expect("a server URL");
        assert_eq!(url.lookup, "@mail.example.org");
        assert_eq!(url.scheme.as_deref(), Some("imaps"));
        assert_eq!(url.login.as_deref(), Some("jane"));
        assert_eq!(url.flagged_domain(), None);

        assert!(Target::parse("pop3s://pop.example.org").is_err());
    }

    #[test]
    fn mailbox_aliases_render_as_a_mailbox_alias_table() {
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

        // The identity says what the account is, so it reads before the
        // mailboxes it names.
        let email = rendered.find("email = ").expect("the address is rendered");
        let alias = rendered
            .find("mailbox.alias")
            .expect("the alias is rendered");
        assert!(email < alias);
    }
}
