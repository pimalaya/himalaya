use color_eyre::Result;
use dialoguer::{Confirm, Input, Password, Select};
#[cfg(feature = "account-discovery")]
use email::account::discover::config::{AuthenticationType, AutoConfig, SecurityType, ServerType};
use email::{
    account::config::{
        oauth2::{OAuth2Config, OAuth2Method, OAuth2Scopes},
        passwd::PasswdConfig,
    },
    imap::config::{ImapAuthConfig, ImapConfig, ImapEncryptionKind},
};
use oauth::v2_0::{AuthorizationCodeGrant, Client};
use secret::Secret;

use crate::{
    backend::config::BackendConfig,
    ui::{prompt, THEME},
    wizard_log, wizard_prompt,
};

const ENCRYPTIONS: &[ImapEncryptionKind] = &[
    ImapEncryptionKind::Tls,
    ImapEncryptionKind::StartTls,
    ImapEncryptionKind::None,
];

const XOAUTH2: &str = "XOAUTH2";
const OAUTHBEARER: &str = "OAUTHBEARER";
const OAUTH2_MECHANISMS: &[&str] = &[XOAUTH2, OAUTHBEARER];

const SECRETS: &[&str] = &[KEYRING, RAW, CMD];
const KEYRING: &str = "Ask my password, then save it in my system's global keyring";
const RAW: &str = "Ask my password, then save it in the configuration file (not safe)";
const CMD: &str = "Ask me a shell command that exposes my password";

#[cfg(feature = "account-discovery")]
pub(crate) async fn configure(
    account_name: &str,
    email: &str,
    autoconfig: Option<&AutoConfig>,
) -> Result<BackendConfig> {
    let autoconfig_oauth2 = autoconfig.and_then(|c| c.oauth2());
    let autoconfig_server = autoconfig.and_then(|c| {
        c.email_provider()
            .incoming_servers()
            .into_iter()
            .find(|server| matches!(server.server_type(), ServerType::Imap))
    });

    let autoconfig_host = autoconfig_server
        .and_then(|s| s.hostname())
        .map(ToOwned::to_owned);

    let default_host =
        autoconfig_host.unwrap_or_else(|| format!("imap.{}", email.rsplit_once('@').unwrap().1));

    let host = Input::with_theme(&*THEME)
        .with_prompt("IMAP hostname")
        .default(default_host)
        .interact()?;

    let autoconfig_encryption = autoconfig_server
        .and_then(|imap| {
            imap.security_type().map(|encryption| match encryption {
                SecurityType::Plain => ImapEncryptionKind::None,
                SecurityType::Starttls => ImapEncryptionKind::StartTls,
                SecurityType::Tls => ImapEncryptionKind::Tls,
            })
        })
        .unwrap_or_default();

    let default_encryption_idx = match &autoconfig_encryption {
        ImapEncryptionKind::Tls => 0,
        ImapEncryptionKind::StartTls => 1,
        ImapEncryptionKind::None => 2,
    };

    let encryption_idx = Select::with_theme(&*THEME)
        .with_prompt("IMAP encryption")
        .items(ENCRYPTIONS)
        .default(default_encryption_idx)
        .interact_opt()?;

    let autoconfig_port = autoconfig_server
        .and_then(|s| s.port())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| match &autoconfig_encryption {
            ImapEncryptionKind::Tls => 993,
            ImapEncryptionKind::StartTls => 143,
            ImapEncryptionKind::None => 143,
        });

    let (encryption, default_port) = match encryption_idx {
        Some(idx) if idx == default_encryption_idx => {
            (Some(autoconfig_encryption), autoconfig_port)
        }
        Some(idx) if ENCRYPTIONS[idx] == ImapEncryptionKind::Tls => {
            (Some(ImapEncryptionKind::Tls), 993)
        }
        Some(idx) if ENCRYPTIONS[idx] == ImapEncryptionKind::StartTls => {
            (Some(ImapEncryptionKind::StartTls), 143)
        }
        _ => (Some(ImapEncryptionKind::None), 143),
    };

    let port = Input::with_theme(&*THEME)
        .with_prompt("IMAP port")
        .validate_with(|input: &String| input.parse::<u16>().map(|_| ()))
        .default(default_port.to_string())
        .interact()
        .map(|input| input.parse::<u16>().unwrap())?;

    let autoconfig_login = autoconfig_server.map(|imap| match imap.username() {
        Some("%EMAILLOCALPART%") => email.rsplit_once('@').unwrap().0.to_owned(),
        Some("%EMAILADDRESS%") => email.to_owned(),
        _ => email.to_owned(),
    });

    let default_login = autoconfig_login.unwrap_or_else(|| email.to_owned());

    let login = Input::with_theme(&*THEME)
        .with_prompt("IMAP login")
        .default(default_login)
        .interact()?;

    let default_oauth2_enabled = autoconfig_server
        .and_then(|imap| {
            imap.authentication_type()
                .into_iter()
                .find_map(|t| Option::from(matches!(t, AuthenticationType::OAuth2)))
        })
        .filter(|_| autoconfig_oauth2.is_some())
        .unwrap_or_default();

    let oauth2_enabled = Confirm::new()
        .with_prompt(wizard_prompt!("Would you like to enable OAuth 2.0?"))
        .default(default_oauth2_enabled)
        .interact_opt()?
        .unwrap_or_default();

    let auth = if oauth2_enabled {
        let mut config = OAuth2Config::default();
        let redirect_host = OAuth2Config::LOCALHOST.to_owned();
        let redirect_port = OAuth2Config::get_first_available_port()?;

        let method_idx = Select::with_theme(&*THEME)
            .with_prompt("IMAP OAuth 2.0 mechanism")
            .items(OAUTH2_MECHANISMS)
            .default(0)
            .interact_opt()?;

        config.method = match method_idx {
            Some(idx) if OAUTH2_MECHANISMS[idx] == XOAUTH2 => OAuth2Method::XOAuth2,
            Some(idx) if OAUTH2_MECHANISMS[idx] == OAUTHBEARER => OAuth2Method::OAuthBearer,
            _ => OAuth2Method::XOAuth2,
        };

        config.client_id = Input::with_theme(&*THEME)
            .with_prompt("IMAP OAuth 2.0 client id")
            .interact()?;

        let client_secret: String = Password::with_theme(&*THEME)
            .with_prompt("IMAP OAuth 2.0 client secret")
            .interact()?;
        config.client_secret =
            Secret::try_new_keyring_entry(format!("{account_name}-imap-oauth2-client-secret"))?;
        config
            .client_secret
            .set_only_keyring(&client_secret)
            .await?;

        let default_auth_url = autoconfig_oauth2
            .map(|o| o.auth_url().to_owned())
            .unwrap_or_default();
        config.auth_url = Input::with_theme(&*THEME)
            .with_prompt("IMAP OAuth 2.0 authorization URL")
            .default(default_auth_url)
            .interact()?;

        let default_token_url = autoconfig_oauth2
            .map(|o| o.token_url().to_owned())
            .unwrap_or_default();
        config.token_url = Input::with_theme(&*THEME)
            .with_prompt("IMAP OAuth 2.0 token URL")
            .default(default_token_url)
            .interact()?;

        let autoconfig_scopes = autoconfig_oauth2.map(|o| o.scope());

        let prompt_scope = |prompt: &str| -> Result<Option<String>> {
            Ok(match &autoconfig_scopes {
                Some(scopes) => Select::with_theme(&*THEME)
                    .with_prompt(prompt)
                    .items(scopes)
                    .default(0)
                    .interact_opt()?
                    .and_then(|idx| scopes.get(idx))
                    .map(|scope| scope.to_string()),
                None => Some(
                    Input::with_theme(&*THEME)
                        .with_prompt(prompt)
                        .default(String::default())
                        .interact()?
                        .to_owned(),
                )
                .filter(|scope| !scope.is_empty()),
            })
        };

        if let Some(scope) = prompt_scope("IMAP OAuth 2.0 main scope")? {
            config.scopes = OAuth2Scopes::Scope(scope);
        }

        let confirm_additional_scope = || -> Result<bool> {
            let confirm = Confirm::new()
                .with_prompt(wizard_prompt!(
                    "Would you like to add more IMAP OAuth 2.0 scopes?"
                ))
                .default(false)
                .interact_opt()?
                .unwrap_or_default();

            Ok(confirm)
        };

        while confirm_additional_scope()? {
            let mut scopes = match config.scopes {
                OAuth2Scopes::Scope(scope) => vec![scope],
                OAuth2Scopes::Scopes(scopes) => scopes,
            };

            if let Some(scope) = prompt_scope("Additional IMAP OAuth 2.0 scope")? {
                scopes.push(scope)
            }

            config.scopes = OAuth2Scopes::Scopes(scopes);
        }

        config.pkce = Confirm::new()
            .with_prompt(wizard_prompt!(
                "Would you like to enable PKCE verification?"
            ))
            .default(true)
            .interact_opt()?
            .unwrap_or(true);

        wizard_log!("To complete your OAuth 2.0 setup, click on the following link:");

        let client = Client::new(
            config.client_id.clone(),
            client_secret,
            config.auth_url.clone(),
            config.token_url.clone(),
        )?
        .with_redirect_host(redirect_host.to_owned())
        .with_redirect_port(redirect_port)
        .build()?;

        let mut auth_code_grant = AuthorizationCodeGrant::new()
            .with_redirect_host(redirect_host.to_owned())
            .with_redirect_port(redirect_port);

        if config.pkce {
            auth_code_grant = auth_code_grant.with_pkce();
        }

        for scope in config.scopes.clone() {
            auth_code_grant = auth_code_grant.with_scope(scope);
        }

        let (redirect_url, csrf_token) = auth_code_grant.get_redirect_url(&client);

        println!("{redirect_url}");
        println!();

        let (access_token, refresh_token) = auth_code_grant
            .wait_for_redirection(&client, csrf_token)
            .await?;

        config.access_token =
            Secret::try_new_keyring_entry(format!("{account_name}-imap-oauth2-access-token"))?;
        config.access_token.set_only_keyring(access_token).await?;

        if let Some(refresh_token) = &refresh_token {
            config.refresh_token =
                Secret::try_new_keyring_entry(format!("{account_name}-imap-oauth2-refresh-token"))?;
            config.refresh_token.set_only_keyring(refresh_token).await?;
        }

        ImapAuthConfig::OAuth2(config)
    } else {
        let secret_idx = Select::with_theme(&*THEME)
            .with_prompt("IMAP authentication strategy")
            .items(SECRETS)
            .default(0)
            .interact_opt()?;

        let secret = match secret_idx {
            Some(idx) if SECRETS[idx] == KEYRING => {
                let secret = Secret::try_new_keyring_entry(format!("{account_name}-imap-passwd"))?;
                secret
                    .set_only_keyring(prompt::passwd("IMAP password")?)
                    .await?;
                secret
            }
            Some(idx) if SECRETS[idx] == RAW => Secret::new_raw(prompt::passwd("IMAP password")?),
            Some(idx) if SECRETS[idx] == CMD => Secret::new_command(
                Input::with_theme(&*THEME)
                    .with_prompt("Shell command")
                    .default(format!("pass show {account_name}-imap-passwd"))
                    .interact()?,
            ),
            _ => Default::default(),
        };

        ImapAuthConfig::Passwd(PasswdConfig(secret))
    };

    let config = ImapConfig {
        host,
        port,
        encryption,
        login,
        auth,
        watch: None,
    };

    Ok(BackendConfig::Imap(config))
}

#[cfg(not(feature = "account-discovery"))]
pub(crate) async fn configure(account_name: &str, email: &str) -> Result<BackendConfig> {
    let default_host = format!("imap.{}", email.rsplit_once('@').unwrap().1);

    let host = Input::with_theme(&*THEME)
        .with_prompt("IMAP hostname")
        .default(default_host)
        .interact()?;

    let encryption_idx = Select::with_theme(&*THEME)
        .with_prompt("IMAP encryption")
        .items(ENCRYPTIONS)
        .default(0)
        .interact_opt()?;

    let (encryption, default_port) = match encryption_idx {
        Some(idx) if ENCRYPTIONS[idx] == ImapEncryptionKind::Tls => {
            (Some(ImapEncryptionKind::Tls), 993)
        }
        Some(idx) if ENCRYPTIONS[idx] == ImapEncryptionKind::StartTls => {
            (Some(ImapEncryptionKind::StartTls), 143)
        }
        _ => (Some(ImapEncryptionKind::None), 143),
    };

    let port = Input::with_theme(&*THEME)
        .with_prompt("IMAP port")
        .validate_with(|input: &String| input.parse::<u16>().map(|_| ()))
        .default(default_port.to_string())
        .interact()
        .map(|input| input.parse::<u16>().unwrap())?;

    let default_login = email.to_owned();

    let login = Input::with_theme(&*THEME)
        .with_prompt("IMAP login")
        .default(default_login)
        .interact()?;

    let oauth2_enabled = Confirm::new()
        .with_prompt(wizard_prompt!("Would you like to enable OAuth 2.0?"))
        .default(false)
        .interact_opt()?
        .unwrap_or_default();

    let auth = if oauth2_enabled {
        let mut config = OAuth2Config::default();
        let redirect_host = OAuth2Config::LOCALHOST.to_owned();
        let redirect_port = OAuth2Config::get_first_available_port()?;

        let method_idx = Select::with_theme(&*THEME)
            .with_prompt("IMAP OAuth 2.0 mechanism")
            .items(OAUTH2_MECHANISMS)
            .default(0)
            .interact_opt()?;

        config.method = match method_idx {
            Some(idx) if OAUTH2_MECHANISMS[idx] == XOAUTH2 => OAuth2Method::XOAuth2,
            Some(idx) if OAUTH2_MECHANISMS[idx] == OAUTHBEARER => OAuth2Method::OAuthBearer,
            _ => OAuth2Method::XOAuth2,
        };

        config.client_id = Input::with_theme(&*THEME)
            .with_prompt("IMAP OAuth 2.0 client id")
            .interact()?;

        let client_secret: String = Password::with_theme(&*THEME)
            .with_prompt("IMAP OAuth 2.0 client secret")
            .interact()?;
        config.client_secret =
            Secret::try_new_keyring_entry(format!("{account_name}-imap-oauth2-client-secret"))?;
        config
            .client_secret
            .set_only_keyring(&client_secret)
            .await?;

        config.auth_url = Input::with_theme(&*THEME)
            .with_prompt("IMAP OAuth 2.0 authorization URL")
            .interact()?;

        config.token_url = Input::with_theme(&*THEME)
            .with_prompt("IMAP OAuth 2.0 token URL")
            .interact()?;

        let prompt_scope = |prompt: &str| -> Result<Option<String>> {
            Ok(Some(
                Input::with_theme(&*THEME)
                    .with_prompt(prompt)
                    .default(String::default())
                    .interact()?
                    .to_owned(),
            )
            .filter(|scope| !scope.is_empty()))
        };

        if let Some(scope) = prompt_scope("IMAP OAuth 2.0 main scope")? {
            config.scopes = OAuth2Scopes::Scope(scope);
        }

        let confirm_additional_scope = || -> Result<bool> {
            let confirm = Confirm::new()
                .with_prompt(wizard_prompt!(
                    "Would you like to add more IMAP OAuth 2.0 scopes?"
                ))
                .default(false)
                .interact_opt()?
                .unwrap_or_default();

            Ok(confirm)
        };

        while confirm_additional_scope()? {
            let mut scopes = match config.scopes {
                OAuth2Scopes::Scope(scope) => vec![scope],
                OAuth2Scopes::Scopes(scopes) => scopes,
            };

            if let Some(scope) = prompt_scope("Additional IMAP OAuth 2.0 scope")? {
                scopes.push(scope)
            }

            config.scopes = OAuth2Scopes::Scopes(scopes);
        }

        config.pkce = Confirm::new()
            .with_prompt(wizard_prompt!(
                "Would you like to enable PKCE verification?"
            ))
            .default(true)
            .interact_opt()?
            .unwrap_or(true);

        wizard_log!("To complete your OAuth 2.0 setup, click on the following link:");

        let client = Client::new(
            config.client_id.clone(),
            client_secret,
            config.auth_url.clone(),
            config.token_url.clone(),
        )?
        .with_redirect_host(redirect_host.to_owned())
        .with_redirect_port(redirect_port)
        .build()?;

        let mut auth_code_grant = AuthorizationCodeGrant::new()
            .with_redirect_host(redirect_host.to_owned())
            .with_redirect_port(redirect_port);

        if config.pkce {
            auth_code_grant = auth_code_grant.with_pkce();
        }

        for scope in config.scopes.clone() {
            auth_code_grant = auth_code_grant.with_scope(scope);
        }

        let (redirect_url, csrf_token) = auth_code_grant.get_redirect_url(&client);

        println!("{redirect_url}");
        println!();

        let (access_token, refresh_token) = auth_code_grant
            .wait_for_redirection(&client, csrf_token)
            .await?;

        config.access_token =
            Secret::try_new_keyring_entry(format!("{account_name}-imap-oauth2-access-token"))?;
        config.access_token.set_only_keyring(access_token).await?;

        if let Some(refresh_token) = &refresh_token {
            config.refresh_token =
                Secret::try_new_keyring_entry(format!("{account_name}-imap-oauth2-refresh-token"))?;
            config.refresh_token.set_only_keyring(refresh_token).await?;
        }

        ImapAuthConfig::OAuth2(config)
    } else {
        let secret_idx = Select::with_theme(&*THEME)
            .with_prompt("IMAP authentication strategy")
            .items(SECRETS)
            .default(0)
            .interact_opt()?;

        let secret = match secret_idx {
            Some(idx) if SECRETS[idx] == KEYRING => {
                let secret = Secret::try_new_keyring_entry(format!("{account_name}-imap-passwd"))?;
                secret
                    .set_only_keyring(prompt::passwd("IMAP password")?)
                    .await?;
                secret
            }
            Some(idx) if SECRETS[idx] == RAW => Secret::new_raw(prompt::passwd("IMAP password")?),
            Some(idx) if SECRETS[idx] == CMD => Secret::new_command(
                Input::with_theme(&*THEME)
                    .with_prompt("Shell command")
                    .default(format!("pass show {account_name}-imap-passwd"))
                    .interact()?,
            ),
            _ => Default::default(),
        };

        ImapAuthConfig::Passwd(PasswdConfig(secret))
    };

    let config = ImapConfig {
        host,
        port,
        encryption,
        login,
        auth,
        watch: None,
    };

    Ok(BackendConfig::Imap(config))
}
