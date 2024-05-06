use color_eyre::Result;
#[cfg(feature = "account-discovery")]
use email::account::discover::config::{AuthenticationType, AutoConfig, SecurityType, ServerType};
use email::{
    account::config::{
        oauth2::{OAuth2Config, OAuth2Method, OAuth2Scopes},
        passwd::PasswdConfig,
    },
    smtp::config::{SmtpAuthConfig, SmtpConfig, SmtpEncryptionKind},
};
use inquire::validator::{ErrorMessage, StringValidator, Validation};
use oauth::v2_0::{AuthorizationCodeGrant, Client};
use secret::Secret;

use crate::{backend::config::BackendConfig, ui::prompt, wizard_log};

const ENCRYPTIONS: &[SmtpEncryptionKind] = &[
    SmtpEncryptionKind::Tls,
    SmtpEncryptionKind::StartTls,
    SmtpEncryptionKind::None,
];

const XOAUTH2: &str = "XOAUTH2";
const OAUTHBEARER: &str = "OAUTHBEARER";
const OAUTH2_MECHANISMS: &[&str] = &[XOAUTH2, OAUTHBEARER];

const SECRETS: &[&str] = &[KEYRING, RAW, CMD];
const KEYRING: &str = "Ask my password, then save it in my system's global keyring";
const RAW: &str = "Ask my password, then save it in the configuration file (not safe)";
const CMD: &str = "Ask me a shell command that exposes my password";

#[derive(Clone, Copy)]
struct U16Validator;

impl StringValidator for U16Validator {
    fn validate(
        &self,
        input: &str,
    ) -> std::prelude::v1::Result<Validation, inquire::CustomUserError> {
        if input.parse::<u16>().is_ok() {
            Ok(Validation::Valid)
        } else {
            Ok(Validation::Invalid(ErrorMessage::Custom(format!(
                "you should enter a number between {} and {}",
                u16::MIN,
                u16::MAX
            ))))
        }
    }
}

#[cfg(feature = "account-discovery")]
pub(crate) async fn configure(
    account_name: &str,
    email: &str,
    autoconfig: Option<&AutoConfig>,
) -> Result<BackendConfig> {
    use color_eyre::eyre::OptionExt as _;
    use inquire::{validator, Confirm, Password, Select, Text};

    let autoconfig_oauth2 = autoconfig.and_then(|c| c.oauth2());
    let autoconfig_server = autoconfig.and_then(|c| {
        c.email_provider()
            .outgoing_servers()
            .into_iter()
            .find(|server| matches!(server.server_type(), ServerType::Smtp))
    });

    let autoconfig_host = autoconfig_server
        .and_then(|s| s.hostname())
        .map(ToOwned::to_owned);

    let default_host =
        autoconfig_host.unwrap_or_else(|| format!("smtp.{}", email.rsplit_once('@').unwrap().1));

    let host = Text::new("SMTP hostname")
        .with_default(&default_host)
        .prompt()?;

    let autoconfig_encryption = autoconfig_server
        .and_then(|smtp| {
            smtp.security_type().map(|encryption| match encryption {
                SecurityType::Plain => SmtpEncryptionKind::None,
                SecurityType::Starttls => SmtpEncryptionKind::StartTls,
                SecurityType::Tls => SmtpEncryptionKind::Tls,
            })
        })
        .unwrap_or_default();

    let default_encryption_idx = match &autoconfig_encryption {
        SmtpEncryptionKind::Tls => 0,
        SmtpEncryptionKind::StartTls => 1,
        SmtpEncryptionKind::None => 2,
    };

    let encryption_kind = Select::new("SMTP encryption", ENCRYPTIONS.to_vec())
        .with_starting_cursor(default_encryption_idx)
        .prompt_skippable()?;

    let autoconfig_port = autoconfig_server
        .and_then(|s| s.port())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| match &autoconfig_encryption {
            SmtpEncryptionKind::Tls => 465,
            SmtpEncryptionKind::StartTls => 587,
            SmtpEncryptionKind::None => 25,
        });

    let (encryption, default_port) = match encryption_kind {
        Some(idx)
            if &idx
                == ENCRYPTIONS.get(default_encryption_idx).ok_or_eyre(
                    "something impossible happened during finding default match for encryption.",
                )? =>
        {
            (Some(autoconfig_encryption), autoconfig_port)
        }
        Some(SmtpEncryptionKind::Tls) => (Some(SmtpEncryptionKind::Tls), 465),
        Some(SmtpEncryptionKind::StartTls) => (Some(SmtpEncryptionKind::StartTls), 587),
        _ => (Some(SmtpEncryptionKind::None), 25),
    };

    let port = Text::new("SMTP port")
        .with_validators(&[
            Box::new(validator::MinLengthValidator::new(1)),
            Box::new(U16Validator {}),
        ])
        .with_default(&default_port.to_string())
        .prompt()
        .map(|input| input.parse::<u16>().unwrap())?;

    let autoconfig_login = autoconfig_server.map(|smtp| match smtp.username() {
        Some("%EMAILLOCALPART%") => email.rsplit_once('@').unwrap().0.to_owned(),
        Some("%EMAILADDRESS%") => email.to_owned(),
        _ => email.to_owned(),
    });

    let default_login = autoconfig_login.unwrap_or_else(|| email.to_owned());

    let login = Text::new("SMTP login")
        .with_default(&default_login)
        .prompt()?;

    let default_oauth2_enabled = autoconfig_server
        .and_then(|smtp| {
            smtp.authentication_type()
                .into_iter()
                .find_map(|t| Option::from(matches!(t, AuthenticationType::OAuth2)))
        })
        .filter(|_| autoconfig_oauth2.is_some())
        .unwrap_or_default();

    let oauth2_enabled = Confirm::new("Would you like to enable OAuth 2.0?")
        .with_default(default_oauth2_enabled)
        .prompt_skippable()?
        .unwrap_or_default();

    let auth = if oauth2_enabled {
        let mut config = OAuth2Config::default();
        let redirect_host = OAuth2Config::LOCALHOST;
        let redirect_port = OAuth2Config::get_first_available_port()?;

        let method_idx = Select::new("SMTP OAuth 2.0 mechanism", OAUTH2_MECHANISMS.to_vec())
            .with_starting_cursor(0)
            .prompt_skippable()?;

        config.method = match method_idx {
            Some(choice) if choice == XOAUTH2 => OAuth2Method::XOAuth2,
            Some(choice) if choice == OAUTHBEARER => OAuth2Method::OAuthBearer,
            _ => OAuth2Method::XOAuth2,
        };

        config.client_id = Text::new("SMTP OAuth 2.0 client id").prompt()?;

        let client_secret: String = Password::new("SMTP OAuth 2.0 client secret")
            .with_display_mode(inquire::PasswordDisplayMode::Masked)
            .prompt()?;
        config.client_secret =
            Secret::try_new_keyring_entry(format!("{account_name}-smtp-oauth2-client-secret"))?;
        config
            .client_secret
            .set_only_keyring(&client_secret)
            .await?;

        let default_auth_url = autoconfig_oauth2
            .map(|o| o.auth_url().to_owned())
            .unwrap_or_default();
        config.auth_url = Text::new("SMTP OAuth 2.0 authorization URL")
            .with_default(&default_auth_url)
            .prompt()?;

        let default_token_url = autoconfig_oauth2
            .map(|o| o.token_url().to_owned())
            .unwrap_or_default();
        config.token_url = Text::new("SMTP OAuth 2.0 token URL")
            .with_default(&default_token_url)
            .prompt()?;

        let autoconfig_scopes = autoconfig_oauth2.map(|o| o.scope());

        let prompt_scope = |prompt: &str| -> Result<Option<String>> {
            Ok(match &autoconfig_scopes {
                Some(scopes) => Select::new(prompt, scopes.to_vec())
                    .with_starting_cursor(0)
                    .prompt_skippable()?
                    .map(ToOwned::to_owned),
                None => Some(Text::new(prompt).prompt()?).filter(|scope| !scope.is_empty()),
            })
        };

        if let Some(scope) = prompt_scope("SMTP OAuth 2.0 main scope")? {
            config.scopes = OAuth2Scopes::Scope(scope);
        }

        let confirm_additional_scope = || -> Result<bool> {
            let confirm = Confirm::new("Would you like to add more SMTP OAuth 2.0 scopes?")
                .with_default(false)
                .prompt_skippable()?
                .unwrap_or_default();

            Ok(confirm)
        };

        while confirm_additional_scope()? {
            let mut scopes = match config.scopes {
                OAuth2Scopes::Scope(scope) => vec![scope],
                OAuth2Scopes::Scopes(scopes) => scopes,
            };

            if let Some(scope) = prompt_scope("Additional SMTP OAuth 2.0 scope")? {
                scopes.push(scope)
            }

            config.scopes = OAuth2Scopes::Scopes(scopes);
        }

        config.pkce = Confirm::new("Would you like to enable PKCE verification?")
            .with_default(true)
            .prompt_skippable()?
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
            Secret::try_new_keyring_entry(format!("{account_name}-smtp-oauth2-access-token"))?;
        config.access_token.set_only_keyring(access_token).await?;

        if let Some(refresh_token) = &refresh_token {
            config.refresh_token =
                Secret::try_new_keyring_entry(format!("{account_name}-smtp-oauth2-refresh-token"))?;
            config.refresh_token.set_only_keyring(refresh_token).await?;
        }

        SmtpAuthConfig::OAuth2(config)
    } else {
        let secret_idx = Select::new("SMTP authentication strategy", SECRETS.to_vec())
            .with_starting_cursor(0)
            .prompt_skippable()?;

        let secret = match secret_idx {
            Some(sec) if sec == KEYRING => {
                let secret = Secret::try_new_keyring_entry(format!("{account_name}-smtp-passwd"))?;
                secret
                    .set_only_keyring(prompt::passwd("SMTP password")?)
                    .await?;
                secret
            }
            Some(sec) if sec == RAW => Secret::new_raw(prompt::passwd("SMTP password")?),
            Some(sec) if sec == CMD => Secret::new_command(
                Text::new("Shell command")
                    .with_default(&format!("pass show {account_name}-smtp-passwd"))
                    .prompt()?,
            ),
            _ => Default::default(),
        };

        SmtpAuthConfig::Passwd(PasswdConfig(secret))
    };

    let config = SmtpConfig {
        host,
        port,
        encryption,
        login,
        auth,
    };

    Ok(BackendConfig::Smtp(config))
}

#[cfg(not(feature = "account-discovery"))]
pub(crate) async fn configure(account_name: &str, email: &str) -> Result<BackendConfig> {
    use inquire::{validator::MinLengthValidator, Confirm, Password, Select, Text};

    let default_host = format!("smtp.{}", email.rsplit_once('@').unwrap().1);

    let host = Text::new("SMTP hostname")
        .with_default(&default_host)
        .prompt()?;

    let encryption_idx = Select::new("SMTP encryption", ENCRYPTIONS.to_vec())
        .with_starting_cursor(0)
        .prompt_skippable()?;

    let (encryption, default_port) = match encryption_idx {
        Some(SmtpEncryptionKind::Tls) => (Some(SmtpEncryptionKind::Tls), 465),
        Some(SmtpEncryptionKind::StartTls) => (Some(SmtpEncryptionKind::StartTls), 587),
        _ => (Some(SmtpEncryptionKind::None), 25),
    };

    let port = Text::new("SMTP port")
        .with_validators(&[
            Box::new(MinLengthValidator::new(1)),
            Box::new(U16Validator {}),
        ])
        .with_default(&default_port.to_string())
        .prompt()
        .map(|input| input.parse::<u16>().unwrap())?;

    let default_login = email.to_owned();

    let login = Text::new("SMTP login")
        .with_default(&default_login)
        .prompt()?;

    let oauth2_enabled = Confirm::new("Would you like to enable OAuth 2.0?")
        .with_default(false)
        .prompt_skippable()?
        .unwrap_or_default();

    let auth = if oauth2_enabled {
        let mut config = OAuth2Config::default();
        let redirect_host = OAuth2Config::LOCALHOST.to_owned();
        let redirect_port = OAuth2Config::get_first_available_port()?;

        let method_idx = Select::new("SMTP OAuth 2.0 mechanism", OAUTH2_MECHANISMS.to_vec())
            .with_starting_cursor(0)
            .prompt_skippable()?;

        config.method = match method_idx {
            Some(XOAUTH2) => OAuth2Method::XOAuth2,
            Some(OAUTHBEARER) => OAuth2Method::OAuthBearer,
            _ => OAuth2Method::XOAuth2,
        };

        config.client_id = Text::new("SMTP OAuth 2.0 client id").prompt()?;

        let client_secret: String = Password::new("SMTP OAuth 2.0 client secret")
            .with_display_mode(inquire::PasswordDisplayMode::Masked)
            .prompt()?;
        config.client_secret =
            Secret::try_new_keyring_entry(format!("{account_name}-smtp-oauth2-client-secret"))?;
        config
            .client_secret
            .set_only_keyring(&client_secret)
            .await?;

        config.auth_url = Text::new("SMTP OAuth 2.0 authorization URL").prompt()?;

        config.token_url = Text::new("SMTP OAuth 2.0 token URL").prompt()?;

        let prompt_scope = |prompt: &str| -> Result<Option<String>> {
            Ok(Some(Text::new(prompt).prompt()?.to_owned()).filter(|scope| !scope.is_empty()))
        };

        if let Some(scope) = prompt_scope("SMTP OAuth 2.0 main scope")? {
            config.scopes = OAuth2Scopes::Scope(scope);
        }

        let confirm_additional_scope = || -> Result<bool> {
            let confirm = Confirm::new("Would you like to add more SMTP OAuth 2.0 scopes?")
                .with_default(false)
                .prompt_skippable()?
                .unwrap_or_default();

            Ok(confirm)
        };

        while confirm_additional_scope()? {
            let mut scopes = match config.scopes {
                OAuth2Scopes::Scope(scope) => vec![scope],
                OAuth2Scopes::Scopes(scopes) => scopes,
            };

            if let Some(scope) = prompt_scope("Additional SMTP OAuth 2.0 scope")? {
                scopes.push(scope)
            }

            config.scopes = OAuth2Scopes::Scopes(scopes);
        }

        config.pkce = Confirm::new("Would you like to enable PKCE verification?")
            .with_default(true)
            .prompt_skippable()?
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
            Secret::try_new_keyring_entry(format!("{account_name}-smtp-oauth2-access-token"))?;
        config.access_token.set_only_keyring(access_token).await?;

        if let Some(refresh_token) = &refresh_token {
            config.refresh_token =
                Secret::try_new_keyring_entry(format!("{account_name}-smtp-oauth2-refresh-token"))?;
            config.refresh_token.set_only_keyring(refresh_token).await?;
        }

        SmtpAuthConfig::OAuth2(config)
    } else {
        let secret_idx = Select::new("SMTP authentication strategy", SECRETS.to_vec())
            .with_starting_cursor(0)
            .prompt_skippable()?;

        let secret = match secret_idx {
            Some(KEYRING) => {
                let secret = Secret::try_new_keyring_entry(format!("{account_name}-smtp-passwd"))?;
                secret
                    .set_only_keyring(prompt::passwd("SMTP password")?)
                    .await?;
                secret
            }
            Some(RAW) => Secret::new_raw(prompt::passwd("SMTP password")?),
            Some(CMD) => Secret::new_command(
                Text::new("Shell command")
                    .with_default(&format!("pass show {account_name}-smtp-passwd"))
                    .prompt()?,
            ),
            _ => Default::default(),
        };

        SmtpAuthConfig::Passwd(PasswdConfig(secret))
    };

    let config = SmtpConfig {
        host,
        port,
        encryption,
        login,
        auth,
    };

    Ok(BackendConfig::Smtp(config))
}
