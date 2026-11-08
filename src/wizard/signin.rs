//! # Sign-ins
//!
//! Signs in the domains the review settled, once per identity: one OAuth
//! hand-off per authorization server, one password or API token per group
//! of servers of one name.
//!
//! Each group is tested as soon as it is typed. A refusal is reported for
//! its domain and offers a way on, the domains that passed being kept,
//! and a typed secret is stored once a test passed, under one keyring
//! entry for every domain it signs in.
//!
//! Himalaya runs no OAuth 2.0 grant of its own: Ortie signs in when it is
//! installed, and a token broker the user picks otherwise.

use std::fmt;

use anyhow::{Error, Result};
use pimalaya_cli::{
    prompt,
    spinner::Spinner,
    wizard::{
        keyring,
        ortie::{self, OrtieSignInError},
    },
};
use pimalaya_config::secret::Secret;
use secrecy::SecretString;

#[cfg(feature = "gmail")]
use crate::config::GmailConfig;
#[cfg(feature = "imap")]
use crate::config::ImapConfig;
#[cfg(feature = "msgraph")]
use crate::config::MsgraphConfig;
#[cfg(feature = "smtp")]
use crate::config::SmtpConfig;
#[cfg(feature = "jmap")]
use crate::config::{JmapAuthConfig, JmapConfig};
#[cfg(any(feature = "imap", feature = "smtp"))]
use crate::config::{
    SaslConfig, SaslLoginConfig, SaslOauthbearerConfig, SaslPlainConfig, SaslScramSha256Config,
    SaslXoauth2Config,
};
#[cfg(feature = "gmail")]
use crate::wizard::gmail;
#[cfg(feature = "imap")]
use crate::wizard::imap;
#[cfg(feature = "jmap")]
use crate::wizard::jmap;
#[cfg(feature = "msgraph")]
use crate::wizard::msgraph;
#[cfg(feature = "smtp")]
use crate::wizard::smtp;
use crate::wizard::{
    plan::{self, Domain, Pick, Picks, Protocol, Route, Setup, SignIn},
    secret,
};

/// What the sign-ins know of the account being set up.
pub struct Identity {
    /// The account name, naming the keyring entries and the Ortie account.
    pub name: String,
    /// The typed address.
    pub address: Option<String>,
    /// The user name a typed server URL carried.
    pub login: Option<String>,
    /// The domain of the address, flagging a server off it.
    pub domain: Option<String>,
}

/// The sign-ins of one wizard run, kept across returns to the review so a
/// domain that passed is not asked again.
#[derive(Default)]
pub struct SignIns {
    /// The domains signed in, with the route each signed in over.
    settled: Vec<Settled>,
    /// The secrets typed and stored.
    typed: Vec<Typed>,
    /// Whether the token broker line was printed.
    broker_told: bool,
}

/// How the sign-ins ended.
pub enum Outcome {
    /// Every domain set up is signed in, or left out.
    Done(Vec<Backend>),
    /// The user went back to the review.
    Review,
}

/// A signed-in backend, as the account holds it.
#[derive(Clone, Debug)]
pub enum Backend {
    /// The IMAP block.
    #[cfg(feature = "imap")]
    Imap(Box<ImapConfig>),
    /// The SMTP block.
    #[cfg(feature = "smtp")]
    Smtp(Box<SmtpConfig>),
    /// The JMAP block.
    #[cfg(feature = "jmap")]
    Jmap(Box<JmapConfig>),
    /// The Gmail API block.
    #[cfg(feature = "gmail")]
    Gmail(Box<GmailConfig>),
    /// The Microsoft Graph block.
    #[cfg(feature = "msgraph")]
    Msgraph(Box<MsgraphConfig>),
}

impl SignIns {
    /// Signs in the domains `setups` resolved to, a domain left out
    /// turning its pick off.
    pub fn run(
        &mut self,
        identity: &Identity,
        setups: &[(Domain, Setup)],
        picks: &mut Picks,
    ) -> Result<Outcome> {
        let routes: Vec<(Domain, &Route)> = setups
            .iter()
            .filter_map(|(domain, setup)| setup.route().map(|route| (*domain, route)))
            .collect();

        let oauth: Vec<&Route> = routes
            .iter()
            .map(|(_, route)| *route)
            .filter(|route| route.sign_in == SignIn::Oauth)
            .collect();
        let suffix = plan::groups(&oauth).len() > 1;

        let mut backends = Vec::new();
        let mut units = Vec::new();

        for (domain, route) in routes {
            let settled = self
                .settled
                .iter()
                .find(|settled| settled.domain == domain && settled.route == *route);

            match settled {
                Some(settled) => backends.push(settled.backend.clone()),
                None => units.push(Unit {
                    domain,
                    login: identity
                        .login
                        .clone()
                        .or_else(|| route.login.clone())
                        .or_else(|| identity.address.clone()),
                    route: route.clone(),
                }),
            }
        }

        let mut alive = backends.len() + units.len();
        let groups = plan::groups(&units.iter().map(|unit| &unit.route).collect::<Vec<_>>());
        let mut units: Vec<Option<Unit>> = units.into_iter().map(Some).collect();

        for group in groups {
            let group = group
                .into_iter()
                .filter_map(|index| units[index].take())
                .collect();

            match self.sign_in(identity, group, suffix, picks, &mut alive)? {
                Some(signed) => backends.extend(signed),
                None => return Ok(Outcome::Review),
            }
        }

        Ok(Outcome::Done(backends))
    }

    /// Signs one group in, answering its backends, or `None` when the user
    /// went back to the review.
    ///
    /// The group's secret is tested against each domain in turn. Retyping
    /// after a refusal concerns the refused domain and the ones after it,
    /// so the domains that passed keep the secret they passed with.
    fn sign_in(
        &mut self,
        identity: &Identity,
        mut pending: Vec<Unit>,
        suffix: bool,
        picks: &mut Picks,
        alive: &mut usize,
    ) -> Result<Option<Vec<Backend>>> {
        let mut backends = Vec::new();
        let mut retry = false;

        while !pending.is_empty() {
            ask_login(&mut pending)?;

            let credential = match self.credential(identity, &pending, suffix, retry)? {
                Ok(credential) => credential,
                Err(err) => {
                    eprintln!("{err:#}");

                    match Way::ask(&pending[0], false, *alive)? {
                        Way::Review => return Ok(None),
                        Way::LeaveOut(domain) => {
                            *picks.get_mut(domain) = Pick::Off;
                            *alive -= 1;
                            pending.remove(0);
                        }
                        Way::TryAgain | Way::ChangeLogin => {}
                    }

                    retry = true;
                    continue;
                }
            };

            let secret = credential.secret();
            let mut passed = Vec::new();
            let mut rest = Vec::new();
            let mut units = pending.into_iter();

            while let Some(unit) = units.next() {
                if let Some(backend) = unit.test(&secret) {
                    passed.push((unit, backend));
                    continue;
                }

                match Way::ask(&unit, unit.route.needs_login(), *alive)? {
                    Way::TryAgain => {
                        rest.push(unit);
                        rest.extend(units);
                        break;
                    }
                    Way::ChangeLogin => {
                        let login = prompt::text("Login:", unit.login.as_deref())?;

                        rest.push(unit);
                        rest.extend(units);

                        for unit in rest.iter_mut().filter(|unit| unit.route.needs_login()) {
                            unit.login = Some(login.clone());
                        }

                        break;
                    }
                    Way::LeaveOut(domain) => {
                        *picks.get_mut(domain) = Pick::Off;
                        *alive -= 1;
                    }
                    Way::Review => return Ok(None),
                }
            }

            if let Some((first, _)) = passed.first() {
                let stored = self.store(identity, credential, first)?;

                for (unit, backend) in passed {
                    let backend = match &stored {
                        Some(stored) => backend.with_secret(stored.clone()),
                        None => backend,
                    };

                    self.settled.push(Settled {
                        domain: unit.domain,
                        route: unit.route,
                        backend: backend.clone(),
                    });

                    backends.push(backend);
                }
            }

            pending = rest;
            retry = true;
        }

        Ok(Some(backends))
    }

    /// The secret `pending` signs in with: issued through OAuth, reused,
    /// or typed. The outer error is a prompt failing, the inner one a
    /// sign-in the user can retry.
    ///
    /// A secret typed earlier for a server of the same name is reused
    /// silently, one for another server only when the user says so. A
    /// retry always asks again.
    fn credential(
        &mut self,
        identity: &Identity,
        pending: &[Unit],
        suffix: bool,
        retry: bool,
    ) -> Result<Result<Credential, Error>> {
        let route = &pending[0].route;

        if route.sign_in == SignIn::Oauth {
            return self.oauth(identity, pending, suffix);
        }

        let host = route.endpoint.host();

        if !retry {
            let earlier = || {
                self.typed
                    .iter()
                    .rev()
                    .filter(|typed| typed.sign_in == route.sign_in)
            };

            if let Some(typed) = earlier().find(|typed| plan::same_host(&typed.host, &host)) {
                return Ok(Ok(typed.reused()));
            }

            if let Some(typed) = earlier().next() {
                let noun = match route.sign_in {
                    SignIn::Token => "API token",
                    _ => "password",
                };

                if prompt::bool(format!("Use the same {noun} for {host}?"), false)? {
                    return Ok(Ok(typed.reused()));
                }
            }
        }

        let urls: Vec<String> = pending.iter().map(|unit| unit.route.url()).collect();
        let urls: Vec<&str> = urls.iter().map(String::as_str).collect();
        let secret =
            keyring::prompt_password(route.sign_in.label(), &urls, identity.domain.as_deref())?;

        Ok(Ok(Credential::Typed(secret)))
    }

    /// Signs `pending` in through Ortie, one account covering every
    /// service of the group, or through the token broker the user picks
    /// when Ortie is not installed.
    fn oauth(
        &mut self,
        identity: &Identity,
        pending: &[Unit],
        suffix: bool,
    ) -> Result<Result<Credential, Error>> {
        let mut services: Vec<&str> = Vec::new();

        for unit in pending {
            let service = unit.route.protocol.service();

            if !services.contains(&service) {
                services.push(service);
            }
        }

        let account = if suffix {
            format!("himalaya-{}-{}", identity.name, services[0])
        } else {
            format!("himalaya-{}", identity.name)
        };

        let input = identity
            .address
            .clone()
            .or_else(|| identity.domain.clone())
            .unwrap_or_else(|| pending[0].route.endpoint.host());

        match ortie::sign_in(&input, &account, &services) {
            Ok(argv) => Ok(Ok(Credential::Read(secret::command_secret(argv)?))),
            Err(OrtieSignInError::NotInstalled) => {
                if !self.broker_told {
                    eprintln!(
                        "A token broker issues the access token: \
                         install Ortie and the wizard signs you in, \
                         or pick the broker you use"
                    );
                    self.broker_told = true;
                }

                let token = secret::configure_token(SignIn::Oauth.label(), &account)?;

                Ok(Ok(Credential::Read(token)))
            }
            Err(err) => Ok(Err(err.into())),
        }
    }

    /// Stores a typed secret once a test passed, under the entry of the
    /// first domain it signed in, answering what the configuration reads
    /// instead, `None` when it reads the tested secret already.
    fn store(
        &mut self,
        identity: &Identity,
        credential: Credential,
        first: &Unit,
    ) -> Result<Option<Secret>> {
        match credential {
            Credential::Read(_) => Ok(None),
            Credential::Reused { stored, .. } => Ok(Some(stored)),
            Credential::Typed(secret) => {
                let sign_in = first.route.sign_in;
                let key = format!(
                    "himalaya-{}-{}",
                    identity.name,
                    first.route.protocol.service()
                );

                let choice = keyring::prompt_store(sign_in.label(), &key, &secret)?;
                let stored = secret::to_secret(choice)?;

                self.typed.push(Typed {
                    sign_in,
                    host: first.route.endpoint.host(),
                    secret,
                    stored: stored.clone(),
                });

                Ok(Some(stored))
            }
        }
    }
}

impl Backend {
    /// The backend reading `secret` where it read the tested one.
    #[cfg_attr(
        not(any(
            feature = "imap",
            feature = "smtp",
            feature = "jmap",
            feature = "gmail",
            feature = "msgraph"
        )),
        allow(unused_variables)
    )]
    fn with_secret(self, secret: Secret) -> Self {
        match self {
            #[cfg(feature = "imap")]
            Self::Imap(mut config) => {
                config.sasl = config.sasl.map(|sasl| sasl_with_secret(sasl, secret));
                Self::Imap(config)
            }
            #[cfg(feature = "smtp")]
            Self::Smtp(mut config) => {
                config.sasl = config.sasl.map(|sasl| sasl_with_secret(sasl, secret));
                Self::Smtp(config)
            }
            #[cfg(feature = "jmap")]
            Self::Jmap(mut config) => {
                config.auth = match config.auth {
                    JmapAuthConfig::Basic { username, .. } => JmapAuthConfig::Basic {
                        username,
                        password: secret,
                    },
                    JmapAuthConfig::Bearer { .. } => JmapAuthConfig::Bearer { token: secret },
                    JmapAuthConfig::Header(_) => JmapAuthConfig::Header(secret),
                };
                Self::Jmap(config)
            }
            #[cfg(feature = "gmail")]
            Self::Gmail(mut config) => {
                config.auth.token = secret;
                Self::Gmail(config)
            }
            #[cfg(feature = "msgraph")]
            Self::Msgraph(mut config) => {
                config.auth.token = secret;
                Self::Msgraph(config)
            }
        }
    }
}

/// A domain signed in during the run, and the route it signed in over.
struct Settled {
    domain: Domain,
    route: Route,
    backend: Backend,
}

/// A secret typed during the run, and what the configuration reads it
/// with.
struct Typed {
    sign_in: SignIn,
    host: String,
    secret: SecretString,
    stored: Secret,
}

/// A domain to sign in.
struct Unit {
    domain: Domain,
    route: Route,
    login: Option<String>,
}

/// The secret a group signs in with.
enum Credential {
    /// Typed now: tested raw, stored once a test passed.
    Typed(SecretString),
    /// Typed earlier in the run, and stored already.
    Reused {
        secret: SecretString,
        stored: Secret,
    },
    /// Read by a command, kept as it is.
    Read(Secret),
}

/// How to go on after a refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Way {
    /// Retype the secret, or sign in again.
    TryAgain,
    /// Ask the login, then retype.
    ChangeLogin,
    /// Save the account without the domain.
    LeaveOut(Domain),
    /// Return to the review.
    Review,
}

impl Typed {
    /// The secret, signing another group in.
    fn reused(&self) -> Credential {
        Credential::Reused {
            secret: self.secret.clone(),
            stored: self.stored.clone(),
        }
    }
}

impl Unit {
    /// Tests the route signing in with `secret` under a spinner, answering
    /// the backend that connected, or `None` once the failure is shown.
    fn test(&self, secret: &Secret) -> Option<Backend> {
        let name = self.route.protocol.name();
        let spinner = Spinner::start(format!("Testing {name} connection"));

        match self.connect(secret) {
            Ok(backend) => {
                spinner.success(format!("{name} connection succeeded"));
                Some(backend)
            }
            Err(err) => {
                spinner.failure(format!("{name} connection failed: {err:#}"));
                None
            }
        }
    }

    /// Connects the route's backend with `secret`.
    #[cfg_attr(
        not(any(feature = "imap", feature = "smtp", feature = "jmap")),
        allow(unused_variables)
    )]
    fn connect(&self, secret: &Secret) -> Result<Backend> {
        let route = &self.route;
        let login = self.login.as_deref().unwrap_or_default();

        match route.protocol {
            #[cfg(feature = "imap")]
            Protocol::Imap => imap::connect(&route.endpoint, route.sign_in, login, secret.clone())
                .map(|config| Backend::Imap(Box::new(config))),
            #[cfg(feature = "smtp")]
            Protocol::Smtp => smtp::connect(&route.endpoint, route.sign_in, login, secret.clone())
                .map(|config| Backend::Smtp(Box::new(config))),
            #[cfg(feature = "jmap")]
            Protocol::Jmap => jmap::connect(&route.endpoint, route.sign_in, login, secret.clone())
                .map(|config| Backend::Jmap(Box::new(config))),
            #[cfg(feature = "gmail")]
            Protocol::Gmail => {
                gmail::connect(secret.clone()).map(|config| Backend::Gmail(Box::new(config)))
            }
            #[cfg(feature = "msgraph")]
            Protocol::Msgraph => {
                msgraph::connect(secret.clone()).map(|config| Backend::Msgraph(Box::new(config)))
            }
            #[cfg(not(all(
                feature = "imap",
                feature = "smtp",
                feature = "jmap",
                feature = "gmail",
                feature = "msgraph"
            )))]
            protocol => Err(Error::msg(format!(
                "{} is not compiled in",
                protocol.name()
            ))),
        }
    }
}

impl Credential {
    /// The secret the tests read.
    fn secret(&self) -> Secret {
        match self {
            Self::Typed(secret) | Self::Reused { secret, .. } => Secret::Raw(secret.clone()),
            Self::Read(secret) => secret.clone(),
        }
    }
}

impl Way {
    /// Asks how to go on after `unit` was refused, offering to change the
    /// login when `login`, and to leave the domain out when another one is
    /// set up or still to come.
    fn ask(unit: &Unit, login: bool, alive: usize) -> Result<Self> {
        let mut ways = vec![Self::TryAgain];

        if login {
            ways.push(Self::ChangeLogin);
        }

        if alive > 1 {
            ways.push(Self::LeaveOut(unit.domain));
        }

        ways.push(Self::Review);

        Ok(prompt::item("What next?", ways, None)?)
    }
}

impl fmt::Display for Way {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TryAgain => f.write_str("Try again"),
            Self::ChangeLogin => f.write_str("Change the login"),
            Self::LeaveOut(domain) => write!(f, "Leave {} out", domain.name().to_lowercase()),
            Self::Review => f.write_str("Back to the review"),
        }
    }
}

/// Asks the login once for the domains needing one nobody knows.
fn ask_login(pending: &mut [Unit]) -> Result<()> {
    let mut missing = pending
        .iter_mut()
        .filter(|unit| unit.route.needs_login() && unit.login.is_none())
        .peekable();

    if missing.peek().is_none() {
        return Ok(());
    }

    let login = prompt::text("Login:", None)?;

    for unit in missing {
        unit.login = Some(login.clone());
    }

    Ok(())
}

/// The SASL block reading `secret` where it read the tested one.
#[cfg(any(feature = "imap", feature = "smtp"))]
fn sasl_with_secret(sasl: SaslConfig, secret: Secret) -> SaslConfig {
    match sasl {
        SaslConfig::Anonymous(config) => SaslConfig::Anonymous(config),
        SaslConfig::Login(config) => SaslConfig::Login(SaslLoginConfig {
            password: secret,
            ..config
        }),
        SaslConfig::Plain(config) => SaslConfig::Plain(SaslPlainConfig {
            passwd: secret,
            ..config
        }),
        SaslConfig::Oauthbearer(config) => SaslConfig::Oauthbearer(SaslOauthbearerConfig {
            token: secret,
            ..config
        }),
        SaslConfig::Xoauth2(config) => SaslConfig::Xoauth2(SaslXoauth2Config {
            token: secret,
            ..config
        }),
        SaslConfig::ScramSha256(config) => SaslConfig::ScramSha256(SaslScramSha256Config {
            password: secret,
            ..config
        }),
    }
}

#[cfg(all(test, feature = "imap", feature = "jmap"))]
mod tests {
    use pimalaya_config::{command::CommandConfig, secret::Secret};
    use secrecy::SecretString;

    use crate::{
        config::{ImapConfig, JmapAuthConfig, JmapConfig, SaslConfig, SaslPlainConfig},
        wizard::signin::*,
    };

    fn stored() -> Secret {
        Secret::Command(CommandConfig::Argv {
            program: String::from("pass"),
            args: vec![String::from("show"), String::from("himalaya-perso-imap")],
        })
    }

    fn typed() -> Secret {
        Secret::Raw(SecretString::from("typed"))
    }

    #[test]
    fn the_stored_secret_replaces_the_typed_one() {
        let imap = Backend::Imap(Box::new(ImapConfig {
            server: String::from("imaps://imap.example.org:993"),
            tls: Default::default(),
            proxy: None,
            starttls: false,
            alpn: Vec::new(),
            sasl: Some(SaslConfig::Plain(SaslPlainConfig {
                authzid: None,
                authcid: String::from("jane@example.org"),
                passwd: typed(),
            })),
            sasl_ir: None,
            id: Default::default(),
            sort: Default::default(),
        }));

        let Backend::Imap(imap) = imap.with_secret(stored()) else {
            panic!("expected an IMAP block");
        };
        let Some(SaslConfig::Plain(plain)) = imap.sasl else {
            panic!("expected a PLAIN block");
        };
        assert_eq!(plain.authcid, "jane@example.org");
        assert!(matches!(plain.passwd, Secret::Command(_)));

        let jmap = Backend::Jmap(Box::new(JmapConfig {
            server: String::from("https://api.example.org/"),
            tls: Default::default(),
            proxy: None,
            alpn: Vec::new(),
            auth: JmapAuthConfig::Basic {
                username: String::from("jane@example.org"),
                password: typed(),
            },
            identity_id: None,
            drafts_mailbox_id: None,
            sent_mailbox_id: None,
        }));

        let Backend::Jmap(jmap) = jmap.with_secret(stored()) else {
            panic!("expected a JMAP block");
        };
        let JmapAuthConfig::Basic { username, password } = jmap.auth else {
            panic!("expected Basic");
        };
        assert_eq!(username, "jane@example.org");
        assert!(matches!(password, Secret::Command(_)));
    }
}
