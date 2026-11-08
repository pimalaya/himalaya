//! # Plan
//!
//! The options discovery found for each domain of an account, mail and
//! sending, ranked so the first one, Auto, is picked without a question,
//! and the words the review shows them in.
//!
//! An option is a [`Route`]: a protocol, a server and a sign-in method.
//! Nothing here prompts or connects, the interactive half belonging to
//! [`crate::wizard::discover`] and [`crate::wizard::signin`].

use std::{fmt, net::IpAddr};

use io_pim_discovery::compose::config::{
    DiscoveryAuthMethod, DiscoveryEndpoint, DiscoverySecurity, DiscoveryService,
    DiscoveryServiceConfig,
};
use url::Url;

/// A domain of the account, as the review rows name it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Domain {
    /// Reading mail.
    Mail,
    /// Sending mail.
    Sending,
}

/// A protocol a route reaches its domain over.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Protocol {
    /// IMAP, reading mail.
    Imap,
    /// SMTP, sending mail.
    Smtp,
    /// JMAP, reading and sending mail over one session.
    Jmap,
    /// The Gmail API, reading and sending mail.
    Gmail,
    /// Microsoft Graph, reading and sending mail.
    Msgraph,
}

/// How a route signs in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignIn {
    /// A password, often an app password.
    Password,
    /// A static API token, sent as a bearer.
    Token,
    /// An OAuth 2.0 access token a broker issues.
    Oauth,
}

/// Where a route reaches its server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Endpoint {
    /// A TCP server, over implicit TLS or STARTTLS.
    Tcp { host: String, port: u16, tls: bool },
    /// An https URL.
    Http(String),
}

/// One way to set a domain up: a protocol, a server and a sign-in method.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Route {
    /// The protocol.
    pub protocol: Protocol,
    /// The server.
    pub endpoint: Endpoint,
    /// The sign-in method.
    pub sign_in: SignIn,
    /// The login the server is known to take.
    pub login: Option<String>,
    /// The authorization server of an OAuth sign-in.
    pub authority: Option<String>,
}

/// The options discovery found for each domain, best first.
#[derive(Clone, Debug, Default)]
pub struct Plan {
    /// The mail routes.
    pub mail: Vec<Route>,
    /// The SMTP routes.
    pub sending: Vec<Route>,
    /// Whether `ortie` is on the `PATH`, which ranks OAuth first and words
    /// it as a browser sign-in.
    pub ortie: bool,
}

/// What the user picked for a domain.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Pick {
    /// The first ranked route, resolved against the other picks.
    #[default]
    Auto,
    /// The route found at this index.
    Found(usize),
    /// A typed server.
    Custom(Route),
    /// Sending over mail's own session.
    ThroughMail,
    /// Nothing.
    Off,
}

/// The picks of both domains.
#[derive(Clone, Debug, Default)]
pub struct Picks {
    /// The mail pick.
    pub mail: Pick,
    /// The sending pick.
    pub sending: Pick,
}

/// What a domain's pick resolves to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Setup {
    /// A route to sign in.
    Route(Route),
    /// Sending over mail's own session.
    ThroughMail,
    /// Nothing.
    Off,
}

/// An entry of the list a change offers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChangeItem {
    /// What the entry reads.
    pub label: String,
    /// What picking it does.
    pub change: Change,
}

/// What picking a change entry does.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Change {
    /// Sets the domain's pick.
    Pick(Pick),
    /// Asks for a server URL.
    Custom,
}

impl Domain {
    /// The domain's name, as the review shows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Mail => "Mail",
            Self::Sending => "Sending",
        }
    }
}

impl Protocol {
    /// The protocol's name, as the review shows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Imap => "IMAP",
            Self::Smtp => "SMTP",
            Self::Jmap => "JMAP",
            Self::Gmail => "Gmail API",
            Self::Msgraph => "Microsoft Graph",
        }
    }

    /// The service name Ortie and the keyring entries take.
    pub fn service(self) -> &'static str {
        match self {
            Self::Imap => "imap",
            Self::Smtp => "smtp",
            Self::Jmap => "jmap",
            Self::Gmail => "gmail",
            Self::Msgraph => "msgraph",
        }
    }

    /// Whether this build carries the protocol's backend.
    pub fn compiled(self) -> bool {
        match self {
            Self::Imap => cfg!(feature = "imap"),
            Self::Smtp => cfg!(feature = "smtp"),
            Self::Jmap => cfg!(feature = "jmap"),
            Self::Gmail => cfg!(feature = "gmail"),
            Self::Msgraph => cfg!(feature = "msgraph"),
        }
    }

    /// Whether mail read over the protocol sends over the same session.
    pub fn sends(self) -> bool {
        matches!(self, Self::Jmap | Self::Gmail | Self::Msgraph)
    }

    /// The provider's own API first, then JMAP, then the rest.
    fn rank(self) -> u8 {
        match self {
            Self::Gmail | Self::Msgraph => 0,
            Self::Jmap => 1,
            Self::Imap | Self::Smtp => 2,
        }
    }
}

impl SignIn {
    /// The secret's name, as a prompt asks for it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Password => "Password",
            Self::Token => "API token",
            Self::Oauth => "Access token",
        }
    }

    /// OAuth first when Ortie can run it, last otherwise.
    fn rank(self, ortie: bool) -> u8 {
        match (self, ortie) {
            (Self::Oauth, true) | (Self::Password, false) => 0,
            (Self::Token, _) => 1,
            (Self::Password, true) | (Self::Oauth, false) => 2,
        }
    }
}

impl fmt::Display for SignIn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Password => "Password",
            Self::Token => "API token",
            Self::Oauth => "OAuth 2.0",
        })
    }
}

impl Endpoint {
    /// The server's host name, lowercased.
    pub fn host(&self) -> String {
        match self {
            Self::Tcp { host, .. } => host.clone(),
            Self::Http(url) => Url::parse(url)
                .ok()
                .and_then(|url| url.host_str().map(str::to_lowercase))
                .unwrap_or_default(),
        }
    }

    /// Implicit TLS before STARTTLS.
    fn rank(&self) -> u8 {
        match self {
            Self::Tcp { tls: false, .. } => 1,
            _ => 0,
        }
    }
}

impl Route {
    /// The server URL, as a password prompt names it.
    pub fn url(&self) -> String {
        match &self.endpoint {
            Endpoint::Tcp { host, port, tls } => {
                let scheme = match (self.protocol, tls) {
                    (Protocol::Smtp, true) => "smtps",
                    (Protocol::Smtp, false) => "smtp",
                    (_, true) => "imaps",
                    (_, false) => "imap",
                };

                format!("{scheme}://{host}:{port}")
            }
            Endpoint::Http(url) => url.clone(),
        }
    }

    /// Whether the route takes a login, beside its secret.
    pub fn needs_login(&self) -> bool {
        match self.protocol {
            Protocol::Imap | Protocol::Smtp => true,
            Protocol::Jmap => self.sign_in == SignIn::Password,
            Protocol::Gmail | Protocol::Msgraph => false,
        }
    }

    /// Whether two routes sign in with one secret: one authorization
    /// server for OAuth, one group of servers of one name otherwise.
    pub fn shares_sign_in(&self, other: &Self) -> bool {
        if self.sign_in != other.sign_in {
            return false;
        }

        match (self.sign_in, &self.authority, &other.authority) {
            (SignIn::Oauth, Some(a), Some(b)) => a == b,
            _ => same_host(&self.endpoint.host(), &other.endpoint.host()),
        }
    }

    /// The routes a discovered config offers, one per sign-in method, none
    /// for an unencrypted server or a service Himalaya does not read.
    fn from_config(config: &DiscoveryServiceConfig) -> Vec<Self> {
        let protocol = match config.service {
            DiscoveryService::Imap => Protocol::Imap,
            DiscoveryService::Smtp => Protocol::Smtp,
            DiscoveryService::Jmap => Protocol::Jmap,
            DiscoveryService::Gmail => Protocol::Gmail,
            DiscoveryService::Msgraph => Protocol::Msgraph,
            _ => return Vec::new(),
        };

        let endpoint = match (&config.endpoint, protocol) {
            (DiscoveryEndpoint::Tcp { security, .. }, _)
                if *security == DiscoverySecurity::Plain =>
            {
                return Vec::new();
            }
            (
                DiscoveryEndpoint::Tcp {
                    host,
                    port,
                    security,
                },
                Protocol::Imap | Protocol::Smtp,
            ) => Endpoint::Tcp {
                host: host.trim_end_matches('.').to_lowercase(),
                port: *port,
                tls: *security == DiscoverySecurity::Tls,
            },
            (
                DiscoveryEndpoint::Http(url),
                Protocol::Jmap | Protocol::Gmail | Protocol::Msgraph,
            ) => match Url::parse(url) {
                Ok(url) if url.scheme() == "https" => Endpoint::Http(url.to_string()),
                _ => return Vec::new(),
            },
            _ => return Vec::new(),
        };

        let login = config
            .username
            .clone()
            .filter(|username| is_address(username));

        let mut sign_ins: Vec<(SignIn, Option<String>)> = Vec::new();

        for method in &config.auth {
            let (sign_in, authority) = match method {
                DiscoveryAuthMethod::Password => (SignIn::Password, None),
                DiscoveryAuthMethod::Bearer => (SignIn::Token, None),
                DiscoveryAuthMethod::OauthAuthorizationCodeGrant { token_endpoint, .. }
                | DiscoveryAuthMethod::OauthDeviceAuthorizationGrant { token_endpoint, .. } => {
                    (SignIn::Oauth, Some(token_endpoint.clone()))
                }
                DiscoveryAuthMethod::OauthIssuer(issuer) => (SignIn::Oauth, Some(issuer.clone())),
            };

            if !sign_ins.iter().any(|(known, _)| *known == sign_in) {
                sign_ins.push((sign_in, authority));
            }
        }

        // A server advertising nothing still takes a secret of some kind.
        if sign_ins.is_empty() {
            sign_ins = vec![(SignIn::Password, None), (SignIn::Token, None)];
        }

        // The provider APIs take OAuth alone.
        if matches!(protocol, Protocol::Gmail | Protocol::Msgraph) {
            sign_ins.retain(|(sign_in, _)| *sign_in == SignIn::Oauth);
        }

        sign_ins
            .into_iter()
            .map(|(sign_in, authority)| Self {
                protocol,
                endpoint: endpoint.clone(),
                sign_in,
                login: login.clone(),
                authority,
            })
            .collect()
    }

    /// Whether two routes read the same: protocol, server and sign-in.
    fn reads_same(&self, other: &Self) -> bool {
        self.protocol == other.protocol
            && self.endpoint == other.endpoint
            && self.sign_in == other.sign_in
    }

    /// The route's rank among the routes of its domain.
    fn rank(&self, ortie: bool) -> (u8, u8, u8) {
        (
            self.protocol.rank(),
            self.endpoint.rank(),
            self.sign_in.rank(ortie),
        )
    }

    /// The change-list entry, before any server is named.
    fn label(&self) -> String {
        format!("{} · {}", self.protocol.name(), self.sign_in)
    }

    /// What the review says of the route, `shared` when it signs in with
    /// mail's secret.
    fn line(&self, ortie: bool, shared: bool) -> String {
        let how = match (self.sign_in, shared, ortie) {
            (SignIn::Password, false, _) => "with your password",
            (SignIn::Password, true, _) => "with mail's password",
            (SignIn::Token, false, _) => "with an API token",
            (SignIn::Token, true, _) => "with mail's API token",
            (SignIn::Oauth, _, true) => "through your browser",
            (SignIn::Oauth, _, false) => "with a token from a broker",
        };

        match self.protocol {
            Protocol::Gmail | Protocol::Msgraph => format!("{}, {how}", self.protocol.name()),
            _ => format!(
                "{} at {}, {how}",
                self.protocol.name(),
                self.endpoint.host()
            ),
        }
    }
}

impl Plan {
    /// Builds the routes of `configs`, deduplicated and ranked.
    ///
    /// Two routes reading the same are one, the first keeping its place
    /// and taking the other's login when it had none. The sort is stable,
    /// so routes of one rank keep discovery's priority order.
    pub fn new(configs: &[DiscoveryServiceConfig], ortie: bool) -> Self {
        let mut mail: Vec<Route> = Vec::new();
        let mut sending: Vec<Route> = Vec::new();

        for route in configs.iter().flat_map(Route::from_config) {
            let routes = match route.protocol {
                Protocol::Smtp => &mut sending,
                _ => &mut mail,
            };

            match routes.iter_mut().find(|known| known.reads_same(&route)) {
                Some(known) => {
                    if known.login.is_none() {
                        known.login = route.login;
                    }
                }
                None => routes.push(route),
            }
        }

        mail.sort_by_key(|route| route.rank(ortie));
        sending.sort_by_key(|route| route.rank(ortie));

        Self {
            mail,
            sending,
            ortie,
        }
    }

    /// Resolves the mail pick.
    pub fn mail(&self, pick: &Pick) -> Setup {
        let route = match pick {
            Pick::Auto => self.mail.first(),
            Pick::Found(index) => self.mail.get(*index),
            Pick::Custom(route) => Some(route),
            Pick::ThroughMail | Pick::Off => None,
        };

        route.cloned().map(Setup::Route).unwrap_or(Setup::Off)
    }

    /// Resolves the sending pick against what mail resolved to.
    ///
    /// Auto, and a pick of sending through mail when mail no longer can,
    /// go through mail when it sends over its own session, else over the
    /// best SMTP server, signing in as mail does when that server offers
    /// it.
    pub fn sending(&self, pick: &Pick, mail: &Setup) -> Setup {
        let mail = mail.route();
        let through = mail.is_some_and(|route| route.protocol.sends());

        let route = match pick {
            Pick::Found(index) => self.sending.get(*index),
            Pick::Custom(route) => Some(route),
            Pick::Off => None,
            Pick::Auto | Pick::ThroughMail if through => return Setup::ThroughMail,
            Pick::Auto | Pick::ThroughMail => self.sending.first().map(|best| {
                mail.and_then(|mail| {
                    self.sending.iter().find(|route| {
                        route.endpoint == best.endpoint && route.sign_in == mail.sign_in
                    })
                })
                .unwrap_or(best)
            }),
        };

        route.cloned().map(Setup::Route).unwrap_or(Setup::Off)
    }

    /// What the review says of a domain set up as `setup`, `mail` being
    /// mail's route when the domain is sending.
    pub fn line(&self, setup: &Setup, mail: Option<&Route>) -> String {
        match setup {
            Setup::Route(route) => {
                let shared = mail.is_some_and(|mail| {
                    route.sign_in != SignIn::Oauth && mail.shares_sign_in(route)
                });

                route.line(self.ortie, shared)
            }
            Setup::ThroughMail => String::from("Through mail"),
            Setup::Off => String::from("Not set up"),
        }
    }

    /// The entries a change of `domain` offers: Auto naming what it is,
    /// every route found, the server named only where two entries would
    /// read the same, a custom server, and for sending the session of mail
    /// and no sending.
    pub fn changes(&self, domain: Domain, picks: &Picks) -> Vec<ChangeItem> {
        let mail = self.mail(&picks.mail);

        let (auto, routes) = match domain {
            Domain::Mail => (self.line(&self.mail(&Pick::Auto), None), &self.mail),
            Domain::Sending => {
                let auto = self.sending(&Pick::Auto, &mail);
                (self.line(&auto, mail.route()), &self.sending)
            }
        };

        let mut items = vec![ChangeItem {
            label: format!("Auto ({auto})"),
            change: Change::Pick(Pick::Auto),
        }];

        let labels: Vec<String> = routes.iter().map(Route::label).collect();

        for (index, route) in routes.iter().enumerate() {
            let label = &labels[index];
            let twins = labels.iter().filter(|other| *other == label).count();

            let label = match twins {
                1 => label.clone(),
                _ => format!("{label} · {}", route.url()),
            };

            items.push(ChangeItem {
                label,
                change: Change::Pick(Pick::Found(index)),
            });
        }

        let sends = mail.route().is_some_and(|route| route.protocol.sends());

        if domain == Domain::Sending && sends {
            items.push(ChangeItem {
                label: String::from("Through mail"),
                change: Change::Pick(Pick::ThroughMail),
            });
        }

        items.push(ChangeItem {
            label: String::from("Custom server URL"),
            change: Change::Custom,
        });

        if domain == Domain::Sending {
            items.push(ChangeItem {
                label: String::from("No sending"),
                change: Change::Pick(Pick::Off),
            });
        }

        items
    }
}

impl Picks {
    /// The pick of `domain`.
    pub fn get(&self, domain: Domain) -> &Pick {
        match domain {
            Domain::Mail => &self.mail,
            Domain::Sending => &self.sending,
        }
    }

    /// The pick of `domain`, to change it.
    pub fn get_mut(&mut self, domain: Domain) -> &mut Pick {
        match domain {
            Domain::Mail => &mut self.mail,
            Domain::Sending => &mut self.sending,
        }
    }
}

impl Setup {
    /// The route to sign in, if any.
    pub fn route(&self) -> Option<&Route> {
        match self {
            Self::Route(route) => Some(route),
            Self::ThroughMail | Self::Off => None,
        }
    }
}

impl fmt::Display for ChangeItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

/// Splits `routes` into sign-in groups, each signing in with one secret:
/// the indices of each group, in the order of their first route.
pub fn groups(routes: &[&Route]) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = Vec::new();

    for (index, route) in routes.iter().enumerate() {
        match groups
            .iter_mut()
            .find(|group| routes[group[0]].shares_sign_in(route))
        {
            Some(group) => group.push(index),
            None => groups.push(vec![index]),
        }
    }

    groups
}

/// Whether two hosts are servers of one name: equal, or equal once their
/// first label is dropped, an IP address or a two-label host never losing
/// one.
pub fn same_host(a: &str, b: &str) -> bool {
    let a = a.trim_end_matches('.').to_lowercase();
    let b = b.trim_end_matches('.').to_lowercase();

    if a == b {
        return true;
    }

    matches!((parent(&a), parent(&b)), (Some(a), Some(b)) if a == b)
}

/// The host without its first label, when that leaves a domain of two
/// labels or more and the host is no IP address.
fn parent(host: &str) -> Option<&str> {
    let bare = host.trim_start_matches('[').trim_end_matches(']');

    if bare.parse::<IpAddr>().is_ok() {
        return None;
    }

    let (_, rest) = host.split_once('.')?;

    rest.contains('.').then_some(rest)
}

/// Whether a string is a full `local@domain` address.
fn is_address(value: &str) -> bool {
    value
        .split_once('@')
        .is_some_and(|(local, domain)| !local.is_empty() && !domain.is_empty())
}

#[cfg(test)]
mod tests {
    use io_pim_discovery::compose::{
        config::{
            DiscoveryAuthMethod, DiscoveryConfigSource, DiscoveryEndpoint, DiscoverySecurity,
            DiscoveryService, DiscoveryServiceConfig,
        },
        providers::DiscoveryKnownProvider,
    };

    use crate::wizard::plan::*;

    const TOKEN_ENDPOINT: &str = "https://auth.example.org/token";

    fn grant() -> DiscoveryAuthMethod {
        DiscoveryAuthMethod::OauthAuthorizationCodeGrant {
            authorization_endpoint: String::from("https://auth.example.org/authorize"),
            token_endpoint: String::from(TOKEN_ENDPOINT),
            scope: None,
            resource: None,
            scopes_supported: Vec::new(),
        }
    }

    fn device_grant() -> DiscoveryAuthMethod {
        DiscoveryAuthMethod::OauthDeviceAuthorizationGrant {
            device_authorization_endpoint: String::from("https://auth.example.org/device"),
            token_endpoint: String::from(TOKEN_ENDPOINT),
            scope: None,
            resource: None,
            scopes_supported: Vec::new(),
        }
    }

    fn tcp(
        service: DiscoveryService,
        host: &str,
        port: u16,
        security: DiscoverySecurity,
        auth: Vec<DiscoveryAuthMethod>,
    ) -> DiscoveryServiceConfig {
        DiscoveryServiceConfig {
            service,
            endpoint: DiscoveryEndpoint::Tcp {
                host: host.to_string(),
                port,
                security,
            },
            username: Some(String::from("jane@example.org")),
            auth,
            source: DiscoveryConfigSource::Srv,
            resolved: false,
        }
    }

    fn http(
        service: DiscoveryService,
        url: &str,
        auth: Vec<DiscoveryAuthMethod>,
    ) -> DiscoveryServiceConfig {
        DiscoveryServiceConfig {
            service,
            endpoint: DiscoveryEndpoint::Http(url.to_string()),
            username: Some(String::from("jane@example.org")),
            auth,
            source: DiscoveryConfigSource::Jmap,
            resolved: false,
        }
    }

    fn imaps(host: &str, auth: Vec<DiscoveryAuthMethod>) -> DiscoveryServiceConfig {
        tcp(
            DiscoveryService::Imap,
            host,
            993,
            DiscoverySecurity::Tls,
            auth,
        )
    }

    fn smtps(host: &str, auth: Vec<DiscoveryAuthMethod>) -> DiscoveryServiceConfig {
        tcp(
            DiscoveryService::Smtp,
            host,
            465,
            DiscoverySecurity::Tls,
            auth,
        )
    }

    fn summary(routes: &[Route]) -> Vec<(Protocol, String, SignIn)> {
        routes
            .iter()
            .map(|route| (route.protocol, route.url(), route.sign_in))
            .collect()
    }

    #[test]
    fn oauth_through_several_grants_is_one_option() {
        let plan = Plan::new(
            &[imaps(
                "imap.example.org",
                vec![
                    grant(),
                    DiscoveryAuthMethod::Password,
                    device_grant(),
                    DiscoveryAuthMethod::Bearer,
                ],
            )],
            true,
        );

        assert_eq!(
            summary(&plan.mail),
            [
                (
                    Protocol::Imap,
                    String::from("imaps://imap.example.org:993"),
                    SignIn::Oauth
                ),
                (
                    Protocol::Imap,
                    String::from("imaps://imap.example.org:993"),
                    SignIn::Token
                ),
                (
                    Protocol::Imap,
                    String::from("imaps://imap.example.org:993"),
                    SignIn::Password
                ),
            ]
        );
        assert_eq!(plan.mail[0].authority.as_deref(), Some(TOKEN_ENDPOINT));
    }

    #[test]
    fn two_mechanisms_finding_one_server_give_one_option() {
        let mut other = imaps("IMAP.example.org.", vec![DiscoveryAuthMethod::Password]);
        other.username = None;
        let mut first = imaps("imap.example.org", vec![DiscoveryAuthMethod::Password]);
        first.username = Some(String::from("example"));

        let plan = Plan::new(&[first, other], false);

        assert_eq!(plan.mail.len(), 1);
        assert_eq!(plan.mail[0].login, None);
    }

    #[test]
    fn a_login_comes_from_a_twin_when_the_first_had_none() {
        let mut first = imaps("imap.example.org", vec![DiscoveryAuthMethod::Password]);
        first.username = Some(String::from("example"));
        let second = imaps("imap.example.org", vec![DiscoveryAuthMethod::Password]);

        let plan = Plan::new(&[first, second], false);

        assert_eq!(plan.mail.len(), 1);
        assert_eq!(plan.mail[0].login.as_deref(), Some("jane@example.org"));
    }

    #[test]
    fn unencrypted_servers_are_not_offered() {
        let plan = Plan::new(
            &[
                tcp(
                    DiscoveryService::Imap,
                    "imap.example.org",
                    143,
                    DiscoverySecurity::Plain,
                    vec![DiscoveryAuthMethod::Password],
                ),
                http(
                    DiscoveryService::Jmap,
                    "http://jmap.example.org/session",
                    vec![DiscoveryAuthMethod::Password],
                ),
                imaps("imap.example.org", vec![DiscoveryAuthMethod::Password]),
            ],
            false,
        );

        assert_eq!(
            summary(&plan.mail),
            [(
                Protocol::Imap,
                String::from("imaps://imap.example.org:993"),
                SignIn::Password
            )]
        );
    }

    #[test]
    fn a_server_advertising_nothing_takes_a_password_or_a_token() {
        let plan = Plan::new(&[imaps("imap.example.org", Vec::new())], true);

        let sign_ins: Vec<SignIn> = plan.mail.iter().map(|route| route.sign_in).collect();
        assert_eq!(sign_ins, [SignIn::Token, SignIn::Password]);
    }

    #[test]
    fn jmap_ranks_first_and_oauth_leads_only_with_ortie() {
        let configs = [
            imaps("imap.example.org", vec![DiscoveryAuthMethod::Password]),
            http(
                DiscoveryService::Jmap,
                "https://api.example.org/jmap/session",
                vec![DiscoveryAuthMethod::Password, grant()],
            ),
        ];

        let with = Plan::new(&configs, true);
        let without = Plan::new(&configs, false);

        assert_eq!(
            summary(&with.mail),
            [
                (
                    Protocol::Jmap,
                    String::from("https://api.example.org/jmap/session"),
                    SignIn::Oauth
                ),
                (
                    Protocol::Jmap,
                    String::from("https://api.example.org/jmap/session"),
                    SignIn::Password
                ),
                (
                    Protocol::Imap,
                    String::from("imaps://imap.example.org:993"),
                    SignIn::Password
                ),
            ]
        );
        assert_eq!(without.mail[0].sign_in, SignIn::Password);
        assert_eq!(without.mail[0].protocol, Protocol::Jmap);

        let mail = with.mail(&Pick::Auto);
        assert_eq!(with.sending(&Pick::Auto, &mail), Setup::ThroughMail);
    }

    #[test]
    fn a_provider_api_ranks_first_with_or_without_ortie() {
        let mut gmail = http(
            DiscoveryService::Gmail,
            "https://gmail.googleapis.com/",
            vec![grant()],
        );
        gmail.source = DiscoveryConfigSource::Provider(DiscoveryKnownProvider::Google);
        let configs = [
            imaps(
                "imap.example.org",
                vec![grant(), DiscoveryAuthMethod::Password],
            ),
            smtps(
                "smtp.example.org",
                vec![grant(), DiscoveryAuthMethod::Password],
            ),
            gmail,
        ];

        for ortie in [true, false] {
            let plan = Plan::new(&configs, ortie);
            let mail = plan.mail(&Pick::Auto);

            assert_eq!(
                mail.route().map(|route| route.protocol),
                Some(Protocol::Gmail)
            );
            assert_eq!(plan.sending(&Pick::Auto, &mail), Setup::ThroughMail);
        }
    }

    #[test]
    fn implicit_tls_ranks_before_starttls() {
        let plan = Plan::new(
            &[
                tcp(
                    DiscoveryService::Imap,
                    "imap.example.org",
                    143,
                    DiscoverySecurity::Starttls,
                    vec![DiscoveryAuthMethod::Password],
                ),
                imaps("imap.example.org", vec![DiscoveryAuthMethod::Password]),
                tcp(
                    DiscoveryService::Smtp,
                    "smtp.example.org",
                    587,
                    DiscoverySecurity::Starttls,
                    vec![DiscoveryAuthMethod::Password],
                ),
                smtps("smtp.example.org", vec![DiscoveryAuthMethod::Password]),
            ],
            false,
        );

        assert_eq!(plan.mail[0].url(), "imaps://imap.example.org:993");
        assert_eq!(plan.sending[0].url(), "smtps://smtp.example.org:465");

        let mail = plan.mail(&Pick::Auto);
        let sending = plan.sending(&Pick::Auto, &mail);
        assert_eq!(
            sending.route().map(Route::url).as_deref(),
            Some("smtps://smtp.example.org:465")
        );
    }

    #[test]
    fn sending_signs_in_as_mail_when_its_best_server_can() {
        let plan = Plan::new(
            &[
                imaps(
                    "imap.example.org",
                    vec![grant(), DiscoveryAuthMethod::Password],
                ),
                smtps(
                    "smtp.example.org",
                    vec![grant(), DiscoveryAuthMethod::Password],
                ),
            ],
            true,
        );

        let auto = plan.mail(&Pick::Auto);
        let sending = plan.sending(&Pick::Auto, &auto);
        assert_eq!(
            sending.route().map(|route| route.sign_in),
            Some(SignIn::Oauth)
        );

        let password = plan.mail(&Pick::Found(1));
        assert_eq!(
            password.route().map(|route| route.sign_in),
            Some(SignIn::Password)
        );
        let sending = plan.sending(&Pick::Auto, &password);
        assert_eq!(
            sending.route().map(|route| route.sign_in),
            Some(SignIn::Password)
        );
    }

    #[test]
    fn sending_is_never_guessed() {
        let plan = Plan::new(
            &[imaps(
                "imap.example.org",
                vec![DiscoveryAuthMethod::Password],
            )],
            false,
        );

        let mail = plan.mail(&Pick::Auto);
        assert_eq!(plan.sending(&Pick::Auto, &mail), Setup::Off);
    }

    #[test]
    fn sending_through_mail_falls_back_when_mail_cannot_send() {
        let plan = Plan::new(
            &[
                http(
                    DiscoveryService::Jmap,
                    "https://api.example.org/jmap/session",
                    vec![DiscoveryAuthMethod::Password],
                ),
                imaps("imap.example.org", vec![DiscoveryAuthMethod::Password]),
                smtps("smtp.example.org", vec![DiscoveryAuthMethod::Password]),
            ],
            false,
        );

        let jmap = plan.mail(&Pick::Auto);
        assert_eq!(plan.sending(&Pick::ThroughMail, &jmap), Setup::ThroughMail);

        let imap = plan.mail(&Pick::Found(1));
        let sending = plan.sending(&Pick::ThroughMail, &imap);
        assert_eq!(
            sending.route().map(|route| route.protocol),
            Some(Protocol::Smtp)
        );
    }

    #[test]
    fn the_review_reads_in_plain_words() {
        let configs = [
            imaps("imap.example.org", vec![DiscoveryAuthMethod::Password]),
            smtps("smtp.example.org", vec![DiscoveryAuthMethod::Password]),
        ];
        let plan = Plan::new(&configs, false);
        let mail = plan.mail(&Pick::Auto);
        let sending = plan.sending(&Pick::Auto, &mail);

        assert_eq!(
            plan.line(&mail, None),
            "IMAP at imap.example.org, with your password"
        );
        assert_eq!(
            plan.line(&sending, mail.route()),
            "SMTP at smtp.example.org, with mail's password"
        );
        assert_eq!(plan.line(&Setup::ThroughMail, None), "Through mail");
        assert_eq!(plan.line(&Setup::Off, None), "Not set up");

        let elsewhere = Plan::new(
            &[
                imaps("imap.example.org", vec![DiscoveryAuthMethod::Password]),
                smtps("smtp.example.net", vec![DiscoveryAuthMethod::Password]),
            ],
            false,
        );
        let mail = elsewhere.mail(&Pick::Auto);
        let sending = elsewhere.sending(&Pick::Auto, &mail);
        assert_eq!(
            elsewhere.line(&sending, mail.route()),
            "SMTP at smtp.example.net, with your password"
        );

        let token = Plan::new(
            &[http(
                DiscoveryService::Jmap,
                "https://api.example.org/jmap/session",
                vec![DiscoveryAuthMethod::Bearer],
            )],
            false,
        );
        assert_eq!(
            token.line(&token.mail(&Pick::Auto), None),
            "JMAP at api.example.org, with an API token"
        );

        let gmail = [http(
            DiscoveryService::Gmail,
            "https://gmail.googleapis.com/",
            vec![grant()],
        )];
        let with = Plan::new(&gmail, true);
        let without = Plan::new(&gmail, false);
        assert_eq!(
            with.line(&with.mail(&Pick::Auto), None),
            "Gmail API, through your browser"
        );
        assert_eq!(
            without.line(&without.mail(&Pick::Auto), None),
            "Gmail API, with a token from a broker"
        );
    }

    #[test]
    fn a_change_names_the_server_only_between_twins() {
        let plan = Plan::new(
            &[
                imaps("imap.example.org", vec![DiscoveryAuthMethod::Password]),
                tcp(
                    DiscoveryService::Imap,
                    "imap.example.org",
                    143,
                    DiscoverySecurity::Starttls,
                    vec![DiscoveryAuthMethod::Password, DiscoveryAuthMethod::Bearer],
                ),
                smtps("smtp.example.org", vec![DiscoveryAuthMethod::Password]),
            ],
            false,
        );

        let labels: Vec<String> = plan
            .changes(Domain::Mail, &Picks::default())
            .into_iter()
            .map(|item| item.label)
            .collect();
        assert_eq!(
            labels,
            [
                "Auto (IMAP at imap.example.org, with your password)",
                "IMAP · Password · imaps://imap.example.org:993",
                "IMAP · Password · imap://imap.example.org:143",
                "IMAP · API token",
                "Custom server URL",
            ]
        );

        let sending = plan.changes(Domain::Sending, &Picks::default());
        let labels: Vec<&str> = sending.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Auto (SMTP at smtp.example.org, with mail's password)",
                "SMTP · Password",
                "Custom server URL",
                "No sending",
            ]
        );
        assert_eq!(sending[3].change, Change::Pick(Pick::Off));
    }

    #[test]
    fn sending_offers_mail_only_when_mail_sends() {
        let plan = Plan::new(
            &[
                http(
                    DiscoveryService::Jmap,
                    "https://api.example.org/jmap/session",
                    vec![DiscoveryAuthMethod::Password],
                ),
                imaps("imap.example.org", vec![DiscoveryAuthMethod::Password]),
            ],
            false,
        );

        let through = |picks: &Picks| {
            plan.changes(Domain::Sending, picks)
                .iter()
                .any(|item| item.change == Change::Pick(Pick::ThroughMail))
        };

        assert!(through(&Picks::default()));
        assert!(!through(&Picks {
            mail: Pick::Found(1),
            sending: Pick::Auto,
        }));
    }

    #[test]
    fn hosts_of_one_name_share_a_password() {
        assert!(same_host("imap.example.org", "smtp.example.org"));
        assert!(same_host("Mail.Example.org.", "mail.example.org"));
        assert!(same_host("example.org", "example.org"));

        assert!(!same_host("imap.example.org", "smtp.example.net"));
        assert!(!same_host("imap.example.org", "example.org"));
        assert!(!same_host("a.org", "b.org"));
        assert!(!same_host("192.0.2.1", "192.0.2.2"));
        assert!(!same_host("[2001:db8::1]", "[2001:db8::2]"));
    }

    #[test]
    fn sign_ins_group_by_identity() {
        let route = |protocol, url: &str, sign_in, authority: Option<&str>| Route {
            protocol,
            endpoint: match protocol {
                Protocol::Imap | Protocol::Smtp => Endpoint::Tcp {
                    host: url.to_string(),
                    port: 993,
                    tls: true,
                },
                _ => Endpoint::Http(url.to_string()),
            },
            sign_in,
            login: None,
            authority: authority.map(ToString::to_string),
        };

        let imap = route(Protocol::Imap, "imap.example.org", SignIn::Password, None);
        let smtp = route(Protocol::Smtp, "smtp.example.org", SignIn::Password, None);
        let elsewhere = route(Protocol::Smtp, "smtp.example.net", SignIn::Password, None);
        let token = route(Protocol::Smtp, "smtp.example.org", SignIn::Token, None);
        assert_eq!(groups(&[&imap, &smtp]), [vec![0, 1]]);
        assert_eq!(groups(&[&imap, &elsewhere]), [vec![0], vec![1]]);
        assert_eq!(groups(&[&imap, &token]), [vec![0], vec![1]]);

        let gmail = route(
            Protocol::Gmail,
            "https://gmail.googleapis.com/",
            SignIn::Oauth,
            Some(TOKEN_ENDPOINT),
        );
        let smtp = route(
            Protocol::Smtp,
            "smtp.example.org",
            SignIn::Oauth,
            Some(TOKEN_ENDPOINT),
        );
        let other = route(
            Protocol::Smtp,
            "smtp.example.org",
            SignIn::Oauth,
            Some("https://auth.example.net/token"),
        );
        assert_eq!(groups(&[&gmail, &smtp]), [vec![0, 1]]);
        assert_eq!(groups(&[&gmail, &other]), [vec![0], vec![1]]);
    }
}
