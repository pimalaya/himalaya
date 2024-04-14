use color_eyre::Result;
use dialoguer::{Confirm, Input, Select};
use shellexpand_utils::expand;
use std::{fs, path::PathBuf, process};
use toml_edit::{DocumentMut, Item};

use crate::{account, ui::THEME};

use super::TomlConfig;

#[macro_export]
macro_rules! wizard_warn {
    ($($arg:tt)*) => {
	println!("{}", console::style(format!($($arg)*)).yellow().bold());
    };
}

#[macro_export]
macro_rules! wizard_prompt {
    ($($arg:tt)*) => {
	format!("{}", console::style(format!($($arg)*)).italic())
    };
}

#[macro_export]
macro_rules! wizard_log {
    ($($arg:tt)*) => {
	println!();
	println!("{}", console::style(format!($($arg)*)).underlined());
	println!();
    };
}

pub(crate) async fn configure(path: &PathBuf) -> Result<TomlConfig> {
    wizard_log!("Configuring your first account:");

    let mut config = TomlConfig::default();

    while let Some((name, account_config)) = account::wizard::configure().await? {
        config.accounts.insert(name, account_config);

        if !Confirm::new()
            .with_prompt(wizard_prompt!(
                "Would you like to configure another account?"
            ))
            .default(false)
            .interact_opt()?
            .unwrap_or_default()
        {
            break;
        }

        wizard_log!("Configuring another account:");
    }

    // If one account is setup, make it the default. If multiple
    // accounts are setup, decide which will be the default. If no
    // accounts are setup, exit the process.
    let default_account = match config.accounts.len() {
        0 => {
            wizard_warn!("No account configured, exiting.");
            process::exit(0);
        }
        1 => Some(config.accounts.values_mut().next().unwrap()),
        _ => {
            let accounts = config.accounts.clone();
            let accounts: Vec<&String> = accounts.keys().collect();

            println!("{} accounts have been configured.", accounts.len());

            Select::with_theme(&*THEME)
                .with_prompt(wizard_prompt!(
                    "Which account would you like to set as your default?"
                ))
                .items(&accounts)
                .default(0)
                .interact_opt()?
                .and_then(|idx| config.accounts.get_mut(accounts[idx]))
        }
    };

    if let Some(account) = default_account {
        account.default = Some(true);
    } else {
        process::exit(0)
    }

    let path = Input::with_theme(&*THEME)
        .with_prompt(wizard_prompt!(
            "Where would you like to save your configuration?"
        ))
        .default(path.to_string_lossy().to_string())
        .interact()?;
    let path = expand::path(path);

    println!("Writing the configuration to {path:?}…");
    let toml = pretty_serialize(&config)?;
    fs::create_dir_all(path.parent().unwrap_or(&path))?;
    fs::write(path, toml)?;

    println!("Exiting the wizard…");
    Ok(config)
}

fn pretty_serialize(config: &TomlConfig) -> Result<String> {
    let mut doc: DocumentMut = toml::to_string(&config)?.parse()?;

    doc.iter_mut().for_each(|(_, item)| {
        if let Some(item) = item.as_table_mut() {
            item.iter_mut().for_each(|(_, item)| {
                set_table_dotted(item, "folder");
                if let Some(item) = get_table_mut(item, "folder") {
                    let keys = ["alias", "add", "list", "expunge", "purge", "delete", "sync"];
                    set_tables_dotted(item, keys);

                    if let Some(item) = get_table_mut(item, "sync") {
                        set_tables_dotted(item, ["filter", "permissions"]);
                    }
                }

                set_table_dotted(item, "envelope");
                if let Some(item) = get_table_mut(item, "envelope") {
                    set_tables_dotted(item, ["list", "get"]);
                }

                set_table_dotted(item, "flag");
                if let Some(item) = get_table_mut(item, "flag") {
                    set_tables_dotted(item, ["add", "set", "remove"]);
                }

                set_table_dotted(item, "message");
                if let Some(item) = get_table_mut(item, "message") {
                    let keys = ["add", "send", "peek", "get", "copy", "move", "delete"];
                    set_tables_dotted(item, keys);
                }

                #[cfg(feature = "maildir")]
                set_table_dotted(item, "maildir");

                #[cfg(feature = "imap")]
                {
                    set_table_dotted(item, "imap");
                    if let Some(item) = get_table_mut(item, "imap") {
                        set_tables_dotted(item, ["passwd", "oauth2"]);
                    }
                }

                #[cfg(feature = "notmuch")]
                set_table_dotted(item, "notmuch");

                #[cfg(feature = "smtp")]
                {
                    set_table_dotted(item, "smtp");
                    if let Some(item) = get_table_mut(item, "smtp") {
                        set_tables_dotted(item, ["passwd", "oauth2"]);
                    }
                }

                #[cfg(feature = "sendmail")]
                set_table_dotted(item, "sendmail");

                #[cfg(feature = "account-sync")]
                set_table_dotted(item, "sync");

                #[cfg(feature = "pgp")]
                set_table_dotted(item, "pgp");
            })
        }
    });

    Ok(doc.to_string())
}

fn get_table_mut<'a>(item: &'a mut Item, key: &'a str) -> Option<&'a mut Item> {
    item.get_mut(key).filter(|item| item.is_table())
}

fn set_table_dotted(item: &mut Item, key: &str) {
    if let Some(table) = get_table_mut(item, key).and_then(|item| item.as_table_mut()) {
        table.set_dotted(true)
    }
}

fn set_tables_dotted<'a>(item: &'a mut Item, keys: impl IntoIterator<Item = &'a str>) {
    for key in keys {
        set_table_dotted(item, key)
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

//     #[cfg(feature = "account-sync")]
//     #[test]
//     fn pretty_serialize_sync_all() {
//         use email::account::sync::config::SyncConfig;

//         assert_eq(
//             TomlAccountConfig {
//                 email: "test@localhost".into(),
//                 sync: Some(SyncConfig {
//                     enable: Some(false),
//                     dir: Some("/tmp/test".into()),
//                     ..Default::default()
//                 }),
//                 ..Default::default()
//             },
//             r#"[accounts.test]
// email = "test@localhost"
// sync.enable = false
// sync.dir = "/tmp/test"
// "#,
//         );
//     }

//     #[cfg(feature = "account-sync")]
//     #[test]
//     fn pretty_serialize_sync_include() {
//         use email::{
//             account::sync::config::SyncConfig,
//             folder::sync::config::{FolderSyncConfig, FolderSyncStrategy},
//         };
//         use std::collections::BTreeSet;

//         use crate::folder::config::FolderConfig;

//         assert_eq(
//             TomlAccountConfig {
//                 email: "test@localhost".into(),
//                 sync: Some(SyncConfig {
//                     enable: Some(true),
//                     dir: Some("/tmp/test".into()),
//                     ..Default::default()
//                 }),
//                 folder: Some(FolderConfig {
//                     sync: Some(FolderSyncConfig {
//                         filter: FolderSyncStrategy::Include(BTreeSet::from_iter(["test".into()])),
//                         ..Default::default()
//                     }),
//                     ..Default::default()
//                 }),
//                 ..Default::default()
//             },
//             r#"[accounts.test]
// email = "test@localhost"
// sync.enable = true
// sync.dir = "/tmp/test"
// folder.sync.filter.include = ["test"]
// folder.sync.permissions.create = true
// folder.sync.permissions.delete = true
// "#,
//         );
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
