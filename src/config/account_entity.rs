use anyhow::{anyhow, Context, Error, Result};
use lettre::transport::smtp::authentication::Credentials as SmtpCredentials;
use log::{debug, trace};
use mailparse::MailAddr;
use std::{convert::TryFrom, env, ffi::OsStr, fs, path::PathBuf};

use crate::{
    config::{Config, ConfigAccount, DEFAULT_PAGE_SIZE, DEFAULT_SIG_DELIM},
    output::run_cmd,
};

pub const DEFAULT_INBOX_FOLDER: &str = "INBOX";
pub const DEFAULT_SENT_FOLDER: &str = "Sent";
pub const DEFAULT_DRAFT_FOLDER: &str = "Drafts";

#[derive(Debug, Default, Clone)]
pub struct BasicAccount {
    pub name: String,
    pub from: String,
    pub downloads_dir: PathBuf,
    pub sig: Option<String>,
    pub default_page_size: usize,
    /// Defines the inbox folder name for this account
    pub inbox_folder: String,
    /// Defines the sent folder name for this account
    pub sent_folder: String,
    /// Defines the draft folder name for this account
    pub draft_folder: String,
    /// Defines the IMAP query used to fetch new messages.
    pub notify_query: String,
    pub watch_cmds: Vec<String>,
    pub default: bool,
    pub email: String,

    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_starttls: bool,
    pub smtp_insecure: bool,
    pub smtp_login: String,
    pub smtp_passwd_cmd: String,

    pub pgp_encrypt_cmd: Option<String>,
    pub pgp_decrypt_cmd: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Account {
    pub name: String,
    pub from: String,
    pub downloads_dir: PathBuf,
    pub sig: Option<String>,
    pub default_page_size: usize,
    /// Defines the inbox folder name for this account
    pub inbox_folder: String,
    /// Defines the sent folder name for this account
    pub sent_folder: String,
    /// Defines the draft folder name for this account
    pub draft_folder: String,
    /// Defines the IMAP query used to fetch new messages.
    pub notify_query: String,
    pub watch_cmds: Vec<String>,
    pub default: bool,
    pub email: String,

    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_starttls: bool,
    pub smtp_insecure: bool,
    pub smtp_login: String,
    pub smtp_passwd_cmd: String,

    pub pgp_encrypt_cmd: Option<String>,
    pub pgp_decrypt_cmd: Option<String>,

    pub with_backend_config: AccountKind,
}

impl Account {
    pub fn address(&self) -> Result<MailAddr> {
        let has_special_chars =
            "()<>[]:;@.,".contains(|special_char| self.from.contains(special_char));
        let addr = if self.from.is_empty() {
            self.email.clone()
        } else if has_special_chars {
            // so the name has special characters => Wrap it with '"'
            format!("\"{}\" <{}>", self.from, self.email)
        } else {
            format!("{} <{}>", self.from, self.email)
        };

        Ok(mailparse::addrparse(&addr)
            .context(format!("cannot parse account address {:?}", self.from))?
            .first()
            .ok_or_else(|| anyhow!("cannot parse account address {:?}", self.from))?
            .clone())
    }

    pub fn smtp_creds(&self) -> Result<SmtpCredentials> {
        let passwd = run_cmd(&self.smtp_passwd_cmd).context("cannot run SMTP passwd cmd")?;
        let passwd = passwd
            .trim_end_matches(|c| c == '\r' || c == '\n')
            .to_owned();

        Ok(SmtpCredentials::new(self.smtp_login.to_owned(), passwd))
    }

    pub fn pgp_encrypt_file(&self, addr: &str, path: PathBuf) -> Result<Option<String>> {
        if let Some(cmd) = self.pgp_encrypt_cmd.as_ref() {
            let encrypt_file_cmd = format!("{} {} {:?}", cmd, addr, path);
            run_cmd(&encrypt_file_cmd).map(Some).context(format!(
                "cannot run pgp encrypt command {:?}",
                encrypt_file_cmd
            ))
        } else {
            Ok(None)
        }
    }

    pub fn pgp_decrypt_file(&self, path: PathBuf) -> Result<Option<String>> {
        if let Some(cmd) = self.pgp_decrypt_cmd.as_ref() {
            let decrypt_file_cmd = format!("{} {:?}", cmd, path);
            run_cmd(&decrypt_file_cmd).map(Some).context(format!(
                "cannot run pgp decrypt command {:?}",
                decrypt_file_cmd
            ))
        } else {
            Ok(None)
        }
    }

    pub fn get_download_file_path<S: AsRef<str>>(&self, file_name: S) -> Result<PathBuf> {
        let file_path = self.downloads_dir.join(file_name.as_ref());
        self.get_unique_download_file_path(&file_path, |path, _count| path.is_file())
            .context(format!(
                "cannot get download file path of {:?}",
                file_name.as_ref()
            ))
    }

    pub fn get_unique_download_file_path(
        &self,
        original_file_path: &PathBuf,
        is_file: impl Fn(&PathBuf, u8) -> bool,
    ) -> Result<PathBuf> {
        let mut count = 0;
        let file_ext = original_file_path
            .extension()
            .and_then(OsStr::to_str)
            .map(|fext| String::from(".") + fext)
            .unwrap_or_default();
        let mut file_path = original_file_path.clone();

        while is_file(&file_path, count) {
            count += 1;
            file_path.set_file_name(OsStr::new(
                &original_file_path
                    .file_stem()
                    .and_then(OsStr::to_str)
                    .map(|fstem| format!("{}_{}{}", fstem, count, file_ext))
                    .ok_or_else(|| anyhow!("cannot get stem from file {:?}", original_file_path))?,
            ));
        }

        Ok(file_path)
    }
}

#[derive(Debug, Clone)]
pub enum AccountKind {
    Imap(ImapAccount),
    None,
}

impl Default for AccountKind {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Default, Clone)]
pub struct ImapAccount {
    pub name: String,
    pub from: String,
    pub downloads_dir: PathBuf,
    pub sig: Option<String>,
    pub default_page_size: usize,
    /// Defines the inbox folder name for this account
    pub inbox_folder: String,
    /// Defines the sent folder name for this account
    pub sent_folder: String,
    /// Defines the draft folder name for this account
    pub draft_folder: String,
    /// Defines the IMAP query used to fetch new messages.
    pub notify_query: String,
    pub watch_cmds: Vec<String>,
    pub default: bool,
    pub email: String,

    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_starttls: bool,
    pub smtp_insecure: bool,
    pub smtp_login: String,
    pub smtp_passwd_cmd: String,

    pub pgp_encrypt_cmd: Option<String>,
    pub pgp_decrypt_cmd: Option<String>,

    pub imap_host: String,
    pub imap_port: u16,
    pub imap_starttls: bool,
    pub imap_insecure: bool,
    pub imap_login: String,
    pub imap_passwd_cmd: String,
}

impl ImapAccount {
    pub fn imap_passwd(&self) -> Result<String> {
        let passwd = run_cmd(&self.imap_passwd_cmd).context("cannot run IMAP passwd cmd")?;
        let passwd = passwd
            .trim_end_matches(|c| c == '\r' || c == '\n')
            .to_owned();

        Ok(passwd)
    }
}

impl<'a> TryFrom<(&'a Config, Option<&str>)> for Account {
    type Error = Error;

    fn try_from((config, account_name): (&'a Config, Option<&str>)) -> Result<Self, Self::Error> {
        debug!("init account `{}`", account_name.unwrap_or("default"));
        let (name, config_account) = match account_name.map(|name| name.trim()) {
            Some("default") | Some("") | None => config
                .accounts
                .iter()
                .find_map(|(name, config_account)| match config_account {
                    ConfigAccount::Imap(account) => {
                        if account.default.unwrap_or(false) {
                            Some((name.to_owned(), config_account))
                        } else {
                            None
                        }
                    }
                })
                .ok_or_else(|| anyhow!("cannot find default account")),
            Some(name) => config
                .accounts
                .get(name)
                .map(|account| (name.to_owned(), account))
                .ok_or_else(|| anyhow!(r#"cannot find account "{}""#, name)),
        }?;

        let downloads_dir = match config_account {
            ConfigAccount::Imap(account) => account
                .downloads_dir
                .as_ref()
                .and_then(|dir| dir.to_str())
                .and_then(|dir| shellexpand::full(dir).ok())
                .map(|dir| PathBuf::from(dir.to_string()))
                .or_else(|| {
                    config
                        .downloads_dir
                        .as_ref()
                        .and_then(|dir| dir.to_str())
                        .and_then(|dir| shellexpand::full(dir).ok())
                        .map(|dir| PathBuf::from(dir.to_string()))
                })
                .unwrap_or_else(env::temp_dir),
        };

        let default_page_size = match config_account {
            ConfigAccount::Imap(account) => account
                .default_page_size
                .as_ref()
                .or_else(|| config.default_page_size.as_ref())
                .unwrap_or(&DEFAULT_PAGE_SIZE)
                .to_owned(),
        };

        let default_sig_delim = DEFAULT_SIG_DELIM.to_string();
        let sig_delim = match config_account {
            ConfigAccount::Imap(account) => account
                .signature_delimiter
                .as_ref()
                .or_else(|| config.signature_delimiter.as_ref())
                .unwrap_or(&default_sig_delim),
        };
        let sig = match config_account {
            ConfigAccount::Imap(account) => account
                .signature
                .as_ref()
                .or_else(|| config.signature.as_ref()),
        };
        let sig = sig
            .and_then(|sig| shellexpand::full(sig).ok())
            .map(String::from)
            .and_then(|sig| fs::read_to_string(sig).ok())
            .or_else(|| sig.map(|sig| sig.to_owned()))
            .map(|sig| format!("{}{}", sig_delim, sig.trim_end()));

        let from = match config_account {
            ConfigAccount::Imap(account) => {
                account.name.as_ref().unwrap_or(&config.name).to_owned()
            }
        };

        let inbox_folder = match config_account {
            ConfigAccount::Imap(account) => account
                .inbox_folder
                .as_deref()
                .or_else(|| config.inbox_folder.as_deref())
                .unwrap_or(DEFAULT_INBOX_FOLDER)
                .to_string(),
        };

        let sent_folder = match config_account {
            ConfigAccount::Imap(account) => account
                .sent_folder
                .as_deref()
                .or_else(|| config.sent_folder.as_deref())
                .unwrap_or(DEFAULT_SENT_FOLDER)
                .to_string(),
        };

        let draft_folder = match config_account {
            ConfigAccount::Imap(account) => account
                .draft_folder
                .as_deref()
                .or_else(|| config.draft_folder.as_deref())
                .unwrap_or(DEFAULT_DRAFT_FOLDER)
                .to_string(),
        };

        let notify_query = match config_account {
            ConfigAccount::Imap(account) => account
                .notify_query
                .as_ref()
                .or_else(|| config.notify_query.as_ref())
                .unwrap_or(&String::from("NEW"))
                .to_owned(),
        };

        let watch_cmds = match config_account {
            ConfigAccount::Imap(account) => account
                .watch_cmds
                .as_ref()
                .or_else(|| config.watch_cmds.as_ref())
                .unwrap_or(&vec![])
                .to_owned(),
        };

        let default = match config_account {
            ConfigAccount::Imap(account) => account.default.unwrap_or_default(),
        };

        let email = match config_account {
            ConfigAccount::Imap(account) => account.email.to_owned(),
        };

        let smtp_host = match config_account {
            ConfigAccount::Imap(account) => account.smtp_host.clone(),
        };

        let smtp_port = match config_account {
            ConfigAccount::Imap(account) => account.smtp_port,
        };

        let smtp_starttls = match config_account {
            ConfigAccount::Imap(account) => account.smtp_starttls.unwrap_or_default(),
        };

        let smtp_insecure = match config_account {
            ConfigAccount::Imap(account) => account.smtp_insecure.unwrap_or_default(),
        };

        let smtp_login = match config_account {
            ConfigAccount::Imap(account) => account.smtp_login.clone(),
        };

        let smtp_passwd_cmd = match config_account {
            ConfigAccount::Imap(account) => account.smtp_passwd_cmd.clone(),
        };

        let pgp_encrypt_cmd = match config_account {
            ConfigAccount::Imap(account) => account.pgp_encrypt_cmd.clone(),
        };

        let pgp_decrypt_cmd = match config_account {
            ConfigAccount::Imap(account) => account.pgp_decrypt_cmd.clone(),
        };

        let account = Account {
            name: name.clone(),
            from: from.clone(),
            downloads_dir: downloads_dir.clone(),
            sig: sig.clone(),
            default_page_size,
            inbox_folder: inbox_folder.clone(),
            sent_folder: sent_folder.clone(),
            draft_folder: draft_folder.clone(),
            notify_query: notify_query.clone(),
            watch_cmds: watch_cmds.clone(),
            default,
            email: email.clone(),
            smtp_host: smtp_host.clone(),
            smtp_port,
            smtp_starttls,
            smtp_insecure,
            smtp_login: smtp_login.clone(),
            smtp_passwd_cmd: smtp_passwd_cmd.clone(),
            pgp_encrypt_cmd: pgp_encrypt_cmd.clone(),
            pgp_decrypt_cmd: pgp_decrypt_cmd.clone(),
            with_backend_config: match &config_account {
                ConfigAccount::Imap(backend_config) => AccountKind::Imap(ImapAccount {
                    name,
                    from,
                    downloads_dir,
                    sig,
                    default_page_size,
                    inbox_folder,
                    sent_folder,
                    draft_folder,
                    notify_query,
                    watch_cmds,
                    default,
                    email,
                    smtp_host,
                    smtp_port,
                    smtp_starttls,
                    smtp_insecure,
                    smtp_login,
                    smtp_passwd_cmd,
                    pgp_encrypt_cmd,
                    pgp_decrypt_cmd,
                    imap_host: backend_config.imap_host.to_owned(),
                    imap_port: backend_config.imap_port,
                    imap_starttls: backend_config.imap_starttls.unwrap_or_default(),
                    imap_insecure: backend_config.imap_insecure.unwrap_or_default(),
                    imap_login: backend_config.imap_login.to_owned(),
                    imap_passwd_cmd: backend_config.imap_passwd_cmd.to_owned(),
                }),
            },
        };

        trace!("account: {:?}", account);
        Ok(account)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_unique_download_file_path() {
        let account = Account::default();
        let path = PathBuf::from("downloads/file.ext");

        // When file path is unique
        assert!(matches!(
            account.get_unique_download_file_path(&path, |_, _| false),
            Ok(path) if path == PathBuf::from("downloads/file.ext")
        ));

        // When 1 file path already exist
        assert!(matches!(
            account.get_unique_download_file_path(&path, |_, count| count <  1),
            Ok(path) if path == PathBuf::from("downloads/file_1.ext")
        ));

        // When 5 file paths already exist
        assert!(matches!(
            account.get_unique_download_file_path(&path, |_, count| count < 5),
            Ok(path) if path == PathBuf::from("downloads/file_5.ext")
        ));

        // When file path has no extension
        let path = PathBuf::from("downloads/file");
        assert!(matches!(
            account.get_unique_download_file_path(&path, |_, count| count < 5),
            Ok(path) if path == PathBuf::from("downloads/file_5")
        ));

        // When file path has 2 extensions
        let path = PathBuf::from("downloads/file.ext.ext2");
        assert!(matches!(
            account.get_unique_download_file_path(&path, |_, count| count < 5),
            Ok(path) if path == PathBuf::from("downloads/file.ext_5.ext2")
        ));
    }
}
