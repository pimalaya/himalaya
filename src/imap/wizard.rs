use anyhow::Result;
use dialoguer::{Confirm, Input, Password, Select};
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
    config::wizard::{prompt_passwd, THEME},
    wizard_log, wizard_prompt,
};

const PROTOCOLS: &[ImapEncryptionKind] = &[
    ImapEncryptionKind::Tls,
    ImapEncryptionKind::StartTls,
    ImapEncryptionKind::None,
];

const PASSWD: &str = "Password";
const OAUTH2: &str = "OAuth 2.0";
const AUTH_MECHANISMS: &[&str] = &[PASSWD, OAUTH2];

const XOAUTH2: &str = "XOAUTH2";
const OAUTHBEARER: &str = "OAUTHBEARER";
const OAUTH2_MECHANISMS: &[&str] = &[XOAUTH2, OAUTHBEARER];

const SECRETS: &[&str] = &[KEYRING, RAW, CMD];
const KEYRING: &str = "Ask my password, then save it in my system's global keyring";
const RAW: &str = "Ask my password, then save it in the configuration file (not safe)";
const CMD: &str = "Ask me a shell command that exposes my password";

pub(crate) async fn configure(account_name: &str, email: &str) -> Result<BackendConfig> {
    let mut config = ImapConfig::default();

    config.host = Input::with_theme(&*THEME)
        .with_prompt("IMAP host")
        .default(format!("imap.{}", email.rsplit_once('@').unwrap().1))
        .interact()?;

    let protocol = Select::with_theme(&*THEME)
        .with_prompt("IMAP security protocol")
        .items(PROTOCOLS)
        .default(0)
        .interact_opt()?;

    let default_port = match protocol {
        Some(idx) if PROTOCOLS[idx] == ImapEncryptionKind::Tls => {
            config.encryption = Some(ImapEncryptionKind::Tls);
            993
        }
        Some(idx) if PROTOCOLS[idx] == ImapEncryptionKind::StartTls => {
            config.encryption = Some(ImapEncryptionKind::StartTls);
            143
        }
        _ => {
            config.encryption = Some(ImapEncryptionKind::None);
            143
        }
    };

    config.port = Input::with_theme(&*THEME)
        .with_prompt("IMAP port")
        .validate_with(|input: &String| input.parse::<u16>().map(|_| ()))
        .default(default_port.to_string())
        .interact()
        .map(|input| input.parse::<u16>().unwrap())?;

    config.login = Input::with_theme(&*THEME)
        .with_prompt("IMAP login")
        .default(email.to_owned())
        .interact()?;

    let auth = Select::with_theme(&*THEME)
        .with_prompt("IMAP authentication mechanism")
        .items(AUTH_MECHANISMS)
        .default(0)
        .interact_opt()?;

    config.auth = match auth {
        Some(idx) if AUTH_MECHANISMS[idx] == PASSWD => {
            let secret = Select::with_theme(&*THEME)
                .with_prompt("IMAP authentication strategy")
                .items(SECRETS)
                .default(0)
                .interact_opt()?;

            let config = match secret {
                Some(idx) if SECRETS[idx] == KEYRING => {
                    Secret::new_keyring_entry(format!("{account_name}-imap-passwd"))
                        .set_keyring_entry_secret(prompt_passwd("IMAP password")?)
                        .await?;
                    PasswdConfig::default()
                }
                Some(idx) if SECRETS[idx] == RAW => PasswdConfig {
                    passwd: Secret::Raw(prompt_passwd("IMAP password")?),
                },
                Some(idx) if SECRETS[idx] == CMD => PasswdConfig {
                    passwd: Secret::new_cmd(
                        Input::with_theme(&*THEME)
                            .with_prompt("Shell command")
                            .default(format!("pass show {account_name}-imap-passwd"))
                            .interact()?,
                    ),
                },
                _ => PasswdConfig::default(),
            };
            ImapAuthConfig::Passwd(config)
        }
        Some(idx) if AUTH_MECHANISMS[idx] == OAUTH2 => {
            let mut config = OAuth2Config::new()?;

            let method = Select::with_theme(&*THEME)
                .with_prompt("IMAP OAuth 2.0 mechanism")
                .items(OAUTH2_MECHANISMS)
                .default(0)
                .interact_opt()?;

            config.method = match method {
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
            Secret::new_keyring_entry(format!("{account_name}-imap-oauth2-client-secret"))
                .set_keyring_entry_secret(&client_secret)
                .await?;

            config.auth_url = Input::with_theme(&*THEME)
                .with_prompt("IMAP OAuth 2.0 authorization URL")
                .interact()?;

            config.token_url = Input::with_theme(&*THEME)
                .with_prompt("IMAP OAuth 2.0 token URL")
                .interact()?;

            config.scopes = OAuth2Scopes::Scope(
                Input::with_theme(&*THEME)
                    .with_prompt("IMAP OAuth 2.0 main scope")
                    .interact()?,
            );

            while Confirm::new()
                .with_prompt(wizard_prompt!(
                    "Would you like to add more IMAP OAuth 2.0 scopes?"
                ))
                .default(false)
                .interact_opt()?
                .unwrap_or_default()
            {
                let mut scopes = match config.scopes {
                    OAuth2Scopes::Scope(scope) => vec![scope],
                    OAuth2Scopes::Scopes(scopes) => scopes,
                };

                scopes.push(
                    Input::with_theme(&*THEME)
                        .with_prompt("Additional IMAP OAuth 2.0 scope")
                        .interact()?,
                );

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
            .with_redirect_host(config.redirect_host.clone())
            .with_redirect_port(config.redirect_port)
            .build()?;

            let mut auth_code_grant = AuthorizationCodeGrant::new()
                .with_redirect_host(config.redirect_host.clone())
                .with_redirect_port(config.redirect_port);

            if config.pkce {
                auth_code_grant = auth_code_grant.with_pkce();
            }

            for scope in config.scopes.clone() {
                auth_code_grant = auth_code_grant.with_scope(scope);
            }

            let (redirect_url, csrf_token) = auth_code_grant.get_redirect_url(&client);

            println!("{}", redirect_url.to_string());
            println!("");

            let (access_token, refresh_token) = auth_code_grant
                .wait_for_redirection(&client, csrf_token)
                .await?;

            Secret::new_keyring_entry(format!("{account_name}-imap-oauth2-access-token"))
                .set_keyring_entry_secret(access_token)
                .await?;

            if let Some(refresh_token) = &refresh_token {
                Secret::new_keyring_entry(format!("{account_name}-imap-oauth2-refresh-token"))
                    .set_keyring_entry_secret(refresh_token)
                    .await?;
            }

            ImapAuthConfig::OAuth2(config)
        }
        _ => ImapAuthConfig::default(),
    };

    Ok(BackendConfig::Imap(config))
}
