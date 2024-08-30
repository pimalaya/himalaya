use std::{fs, path::PathBuf};

use color_eyre::Result;
use pimalaya_tui::{print, prompt};
use toml_edit::{DocumentMut, Table};

use crate::account;

use super::TomlConfig;

pub async fn configure(path: &PathBuf) -> Result<TomlConfig> {
    print::section("Configuring your default account");

    let mut config = TomlConfig::default();

    let (account_name, account_config) = account::wizard::configure().await?;
    config.accounts.insert(account_name, account_config);

    let path = prompt::path("Where to save the configuration?", Some(path))?;
    println!("Writing the configuration to {}…", path.display());

    let toml = pretty_serialize(&config)?;
    fs::create_dir_all(path.parent().unwrap_or(&path))?;
    fs::write(path, toml)?;

    println!("Done! Exiting the wizard…");
    Ok(config)
}

fn pretty_serialize(config: &TomlConfig) -> Result<String> {
    let mut doc: DocumentMut = toml::to_string(&config)?.parse()?;

    doc.iter_mut().for_each(|(_, item)| {
        if let Some(table) = item.as_table_mut() {
            table.iter_mut().for_each(|(_, item)| {
                if let Some(table) = item.as_table_mut() {
                    set_table_dotted(table);
                }
            })
        }
    });

    Ok(doc.to_string())
}

fn set_table_dotted(table: &mut Table) {
    let keys: Vec<String> = table.iter().map(|(key, _)| key.to_string()).collect();
    for ref key in keys {
        if let Some(table) = table.get_mut(key).unwrap().as_table_mut() {
            table.set_dotted(true);
            set_table_dotted(table)
        }
    }
}

// #[cfg(test)]
// mod test {
//     use std::collections::HashMap;

//     use crate::{account::config::TomlAccountConfig, config::TomlConfig};

//     use super::pretty_serialize;

//     fn assert_eq(config: TomlAccountConfig, expected_toml: &str) {
//         let config = TomlConfig {
//             accounts: HashMap::from_iter([("test".into(), config)]),
//             ..Default::default()
//         };

//         let toml = pretty_serialize(&config).expect("serialize error");
//         assert_eq!(toml, expected_toml);

//         let expected_config = toml::from_str(&toml).expect("deserialize error");
//         assert_eq!(config, expected_config);
//     }

//     #[test]
//     fn pretty_serialize_default() {
//         assert_eq(
//             TomlAccountConfig {
//                 email: "test@localhost".into(),
//                 ..Default::default()
//             },
//             r#"[accounts.test]
// email = "test@localhost"
// "#,
//         )
//     }

//     #[cfg(feature = "imap")]
//     #[test]
//     fn pretty_serialize_imap_passwd_cmd() {
//         use email::{
//             account::config::passwd::PasswdConfig,
//             imap::config::{ImapAuthConfig, ImapConfig},
//         };
//         use secret::Secret;

//         assert_eq(
//             TomlAccountConfig {
//                 email: "test@localhost".into(),
//                 imap: Some(ImapConfig {
//                     host: "localhost".into(),
//                     port: 143,
//                     login: "test@localhost".into(),
//                     auth: ImapAuthConfig::Passwd(PasswdConfig(Secret::new_command(
//                         "pass show test",
//                     ))),
//                     ..Default::default()
//                 }),
//                 ..Default::default()
//             },
//             r#"[accounts.test]
// email = "test@localhost"
// imap.host = "localhost"
// imap.port = 143
// imap.login = "test@localhost"
// imap.passwd.command = "pass show test"
// "#,
//         );
//     }

//     #[cfg(feature = "imap")]
//     #[test]
//     fn pretty_serialize_imap_passwd_cmds() {
//         use email::{
//             account::config::passwd::PasswdConfig,
//             imap::config::{ImapAuthConfig, ImapConfig},
//         };
//         use secret::Secret;

//         assert_eq(
//             TomlAccountConfig {
//                 email: "test@localhost".into(),
//                 imap: Some(ImapConfig {
//                     host: "localhost".into(),
//                     port: 143,
//                     login: "test@localhost".into(),
//                     auth: ImapAuthConfig::Passwd(PasswdConfig(Secret::new_command(vec![
//                         "pass show test",
//                         "tr -d '[:blank:]'",
//                     ]))),
//                     ..Default::default()
//                 }),
//                 ..Default::default()
//             },
//             r#"[accounts.test]
// email = "test@localhost"
// imap.host = "localhost"
// imap.port = 143
// imap.login = "test@localhost"
// imap.passwd.command = ["pass show test", "tr -d '[:blank:]'"]
// "#,
//         );
//     }

//     #[cfg(feature = "imap")]
//     #[test]
//     fn pretty_serialize_imap_oauth2() {
//         use email::{
//             account::config::oauth2::OAuth2Config,
//             imap::config::{ImapAuthConfig, ImapConfig},
//         };

//         assert_eq(
//             TomlAccountConfig {
//                 email: "test@localhost".into(),
//                 imap: Some(ImapConfig {
//                     host: "localhost".into(),
//                     port: 143,
//                     login: "test@localhost".into(),
//                     auth: ImapAuthConfig::OAuth2(OAuth2Config {
//                         client_id: "client-id".into(),
//                         auth_url: "auth-url".into(),
//                         token_url: "token-url".into(),
//                         ..Default::default()
//                     }),
//                     ..Default::default()
//                 }),
//                 ..Default::default()
//             },
//             r#"[accounts.test]
// email = "test@localhost"
// imap.host = "localhost"
// imap.port = 143
// imap.login = "test@localhost"
// imap.oauth2.method = "xoauth2"
// imap.oauth2.client-id = "client-id"
// imap.oauth2.auth-url = "auth-url"
// imap.oauth2.token-url = "token-url"
// imap.oauth2.pkce = false
// imap.oauth2.scopes = []
// "#,
//         );
//     }

//     #[cfg(feature = "maildir")]
//     #[test]
//     fn pretty_serialize_maildir() {
//         use email::maildir::config::MaildirConfig;

//         assert_eq(
//             TomlAccountConfig {
//                 email: "test@localhost".into(),
//                 maildir: Some(MaildirConfig {
//                     root_dir: "/tmp/test".into(),
//                 }),
//                 ..Default::default()
//             },
//             r#"[accounts.test]
// email = "test@localhost"
// maildir.root-dir = "/tmp/test"
// "#,
//         );
//     }

//     #[cfg(feature = "smtp")]
//     #[test]
//     fn pretty_serialize_smtp_passwd_cmd() {
//         use email::{
//             account::config::passwd::PasswdConfig,
//             smtp::config::{SmtpAuthConfig, SmtpConfig},
//         };
//         use secret::Secret;

//         assert_eq(
//             TomlAccountConfig {
//                 email: "test@localhost".into(),
//                 smtp: Some(SmtpConfig {
//                     host: "localhost".into(),
//                     port: 143,
//                     login: "test@localhost".into(),
//                     auth: SmtpAuthConfig::Passwd(PasswdConfig(Secret::new_command(
//                         "pass show test",
//                     ))),
//                     ..Default::default()
//                 }),
//                 ..Default::default()
//             },
//             r#"[accounts.test]
// email = "test@localhost"
// smtp.host = "localhost"
// smtp.port = 143
// smtp.login = "test@localhost"
// smtp.passwd.command = "pass show test"
// "#,
//         );
//     }

//     #[cfg(feature = "smtp")]
//     #[test]
//     fn pretty_serialize_smtp_passwd_cmds() {
//         use email::{
//             account::config::passwd::PasswdConfig,
//             smtp::config::{SmtpAuthConfig, SmtpConfig},
//         };
//         use secret::Secret;

//         assert_eq(
//             TomlAccountConfig {
//                 email: "test@localhost".into(),
//                 smtp: Some(SmtpConfig {
//                     host: "localhost".into(),
//                     port: 143,
//                     login: "test@localhost".into(),
//                     auth: SmtpAuthConfig::Passwd(PasswdConfig(Secret::new_command(vec![
//                         "pass show test",
//                         "tr -d '[:blank:]'",
//                     ]))),
//                     ..Default::default()
//                 }),
//                 ..Default::default()
//             },
//             r#"[accounts.test]
// email = "test@localhost"
// smtp.host = "localhost"
// smtp.port = 143
// smtp.login = "test@localhost"
// smtp.passwd.command = ["pass show test", "tr -d '[:blank:]'"]
// "#,
//         );
//     }

//     #[cfg(feature = "smtp")]
//     #[test]
//     fn pretty_serialize_smtp_oauth2() {
//         use email::{
//             account::config::oauth2::OAuth2Config,
//             smtp::config::{SmtpAuthConfig, SmtpConfig},
//         };

//         assert_eq(
//             TomlAccountConfig {
//                 email: "test@localhost".into(),
//                 smtp: Some(SmtpConfig {
//                     host: "localhost".into(),
//                     port: 143,
//                     login: "test@localhost".into(),
//                     auth: SmtpAuthConfig::OAuth2(OAuth2Config {
//                         client_id: "client-id".into(),
//                         auth_url: "auth-url".into(),
//                         token_url: "token-url".into(),
//                         ..Default::default()
//                     }),
//                     ..Default::default()
//                 }),
//                 ..Default::default()
//             },
//             r#"[accounts.test]
// email = "test@localhost"
// smtp.host = "localhost"
// smtp.port = 143
// smtp.login = "test@localhost"
// smtp.oauth2.method = "xoauth2"
// smtp.oauth2.client-id = "client-id"
// smtp.oauth2.auth-url = "auth-url"
// smtp.oauth2.token-url = "token-url"
// smtp.oauth2.pkce = false
// smtp.oauth2.scopes = []
// "#,
//         );
//     }

//     #[cfg(feature = "pgp-cmds")]
//     #[test]
//     fn pretty_serialize_pgp_cmds() {
//         use email::account::config::pgp::PgpConfig;

//         assert_eq(
//             TomlAccountConfig {
//                 email: "test@localhost".into(),
//                 pgp: Some(PgpConfig::Cmds(Default::default())),
//                 ..Default::default()
//             },
//             r#"[accounts.test]
// email = "test@localhost"
// pgp.backend = "cmds"
// "#,
//         );
//     }
// }
