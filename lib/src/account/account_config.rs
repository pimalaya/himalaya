//! Account config module.
//!
//! This module contains the representation of the user account.

use lettre::transport::smtp::authentication::Credentials as SmtpCredentials;
use log::{debug, info, trace};
use mailparse::MailAddr;
use serde::Deserialize;
use shellexpand;
use std::{collections::HashMap, env, ffi::OsStr, fs, path::PathBuf};
use thiserror::Error;

use crate::process::{self, ProcessError};

use super::*;

pub const DEFAULT_PAGE_SIZE: usize = 10;
pub const DEFAULT_SIG_DELIM: &str = "-- \n";

pub const DEFAULT_INBOX_FOLDER: &str = "INBOX";
pub const DEFAULT_SENT_FOLDER: &str = "Sent";
pub const DEFAULT_DRAFT_FOLDER: &str = "Drafts";

#[derive(Debug, Error)]
pub enum AccountError {
    #[error("cannot encrypt file using pgp")]
    EncryptFileError(#[source] ProcessError),
    #[error("cannot find encrypt file command from config file")]
    EncryptFileMissingCmdError,

    #[error("cannot decrypt file using pgp")]
    DecryptFileError(#[source] ProcessError),
    #[error("cannot find decrypt file command from config file")]
    DecryptFileMissingCmdError,

    #[error("cannot get smtp password")]
    GetSmtpPasswdError(#[source] ProcessError),
    #[error("cannot get smtp password: password is empty")]
    GetSmtpPasswdEmptyError,

    #[cfg(feature = "imap-backend")]
    #[error("cannot get imap password")]
    GetImapPasswdError(#[source] ProcessError),
    #[cfg(feature = "imap-backend")]
    #[error("cannot get imap password: password is empty")]
    GetImapPasswdEmptyError,

    #[error("cannot find default account")]
    FindDefaultAccountError,
    #[error("cannot find account {0}")]
    FindAccountError(String),
    #[error("cannot parse account address {0}")]
    ParseAccountAddrError(#[source] mailparse::MailParseError, String),
    #[error("cannot find account address in {0}")]
    ParseAccountAddrNotFoundError(String),

    #[cfg(feature = "maildir-backend")]
    #[error("cannot expand maildir path")]
    ExpandMaildirPathError(#[source] shellexpand::LookupError<env::VarError>),
    #[cfg(feature = "notmuch-backend")]
    #[error("cannot expand notmuch path")]
    ExpandNotmuchDatabasePathError(#[source] shellexpand::LookupError<env::VarError>),
    #[error("cannot expand mailbox alias {1}")]
    ExpandMboxAliasError(#[source] shellexpand::LookupError<env::VarError>, String),

    #[error("cannot parse download file name from {0}")]
    ParseDownloadFileNameError(PathBuf),

    #[error("cannot start the notify mode")]
    StartNotifyModeError(#[source] ProcessError),
}

/// Represents the user account.
#[derive(Debug, Default, Clone)]
pub struct Account {
    /// Represents the name of the user account.
    pub name: String,
    /// Makes this account the default one.
    pub default: bool,
    /// Represents the display name of the user account.
    pub display_name: String,
    /// Represents the email address of the user account.
    pub email: String,
    /// Represents the downloads directory (mostly for attachments).
    pub downloads_dir: PathBuf,
    /// Represents the signature of the user.
    pub sig: Option<String>,
    /// Represents the default page size for listings.
    pub default_page_size: usize,
    /// Represents the notify command.
    pub notify_cmd: Option<String>,
    /// Overrides the default IMAP query "NEW" used to fetch new messages
    pub notify_query: String,
    /// Represents the watch commands.
    pub watch_cmds: Vec<String>,
    /// Represents the text/plain format as defined in the
    /// [RFC2646](https://www.ietf.org/rfc/rfc2646.txt)
    pub format: TextPlainFormat,
    /// Overrides the default headers displayed at the top of
    /// the read message.
    pub read_headers: Vec<String>,

    /// Represents mailbox aliases.
    pub mailboxes: HashMap<String, String>,

    /// Represents hooks.
    pub hooks: Hooks,

    /// Represents the SMTP host.
    pub smtp_host: String,
    /// Represents the SMTP port.
    pub smtp_port: u16,
    /// Enables StartTLS.
    pub smtp_starttls: bool,
    /// Trusts any certificate.
    pub smtp_insecure: bool,
    /// Represents the SMTP login.
    pub smtp_login: String,
    /// Represents the SMTP password command.
    pub smtp_passwd_cmd: String,

    /// Represents the command used to encrypt a message.
    pub pgp_encrypt_cmd: Option<String>,
    /// Represents the command used to decrypt a message.
    pub pgp_decrypt_cmd: Option<String>,
}

impl<'a> Account {
    /// Tries to create an account from a config and an optional
    /// account name.
    pub fn from_config_and_opt_account_name(
        config: &'a DeserializedConfig,
        account_name: Option<&str>,
    ) -> Result<(Account, BackendConfig), AccountError> {
        info!("begin: parsing account and backend configs from config and account name");

        debug!("account name: {:?}", account_name.unwrap_or("default"));
        let (name, account) = match account_name.map(|name| name.trim()) {
            Some("default") | Some("") | None => config
                .accounts
                .iter()
                .find(|(_, account)| match account {
                    #[cfg(feature = "imap-backend")]
                    DeserializedAccountConfig::Imap(account) => account.default.unwrap_or_default(),
                    #[cfg(feature = "maildir-backend")]
                    DeserializedAccountConfig::Maildir(account) => {
                        account.default.unwrap_or_default()
                    }
                    #[cfg(feature = "notmuch-backend")]
                    DeserializedAccountConfig::Notmuch(account) => {
                        account.default.unwrap_or_default()
                    }
                })
                .map(|(name, account)| (name.to_owned(), account))
                .ok_or_else(|| AccountError::FindDefaultAccountError),
            Some(name) => config
                .accounts
                .get(name)
                .map(|account| (name.to_owned(), account))
                .ok_or_else(|| AccountError::FindAccountError(name.to_owned())),
        }?;

        let base_account = account.to_base();
        let downloads_dir = base_account
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
            .unwrap_or_else(env::temp_dir);

        let default_page_size = base_account
            .default_page_size
            .as_ref()
            .or_else(|| config.default_page_size.as_ref())
            .unwrap_or(&DEFAULT_PAGE_SIZE)
            .to_owned();

        let default_sig_delim = DEFAULT_SIG_DELIM.to_string();
        let sig_delim = base_account
            .signature_delimiter
            .as_ref()
            .or_else(|| config.signature_delimiter.as_ref())
            .unwrap_or(&default_sig_delim);
        let sig = base_account
            .signature
            .as_ref()
            .or_else(|| config.signature.as_ref());
        let sig = sig
            .and_then(|sig| shellexpand::full(sig).ok())
            .map(String::from)
            .and_then(|sig| fs::read_to_string(sig).ok())
            .or_else(|| sig.map(|sig| sig.to_owned()))
            .map(|sig| format!("{}{}", sig_delim, sig.trim_end()));

        let account_config = Account {
            name,
            display_name: base_account
                .name
                .as_ref()
                .unwrap_or(&config.name)
                .to_owned(),
            downloads_dir,
            sig,
            default_page_size,
            notify_cmd: base_account
                .notify_cmd
                .as_ref()
                .or_else(|| config.notify_cmd.as_ref())
                .cloned(),
            notify_query: base_account
                .notify_query
                .as_ref()
                .or_else(|| config.notify_query.as_ref())
                .unwrap_or(&String::from("NEW"))
                .to_owned(),
            watch_cmds: base_account
                .watch_cmds
                .as_ref()
                .or_else(|| config.watch_cmds.as_ref())
                .unwrap_or(&vec![])
                .to_owned(),
            format: base_account.format.unwrap_or_default(),
            read_headers: base_account.read_headers,
            mailboxes: base_account.mailboxes.clone(),
            hooks: base_account.hooks.unwrap_or_default(),
            default: base_account.default.unwrap_or_default(),
            email: base_account.email.to_owned(),

            smtp_host: base_account.smtp_host.to_owned(),
            smtp_port: base_account.smtp_port,
            smtp_starttls: base_account.smtp_starttls.unwrap_or_default(),
            smtp_insecure: base_account.smtp_insecure.unwrap_or_default(),
            smtp_login: base_account.smtp_login.to_owned(),
            smtp_passwd_cmd: base_account.smtp_passwd_cmd.to_owned(),

            pgp_encrypt_cmd: base_account.pgp_encrypt_cmd.to_owned(),
            pgp_decrypt_cmd: base_account.pgp_decrypt_cmd.to_owned(),
        };
        trace!("account config: {:?}", account_config);

        let backend_config = match account {
            #[cfg(feature = "imap-backend")]
            DeserializedAccountConfig::Imap(config) => BackendConfig::Imap(ImapBackendConfig {
                imap_host: config.imap_host.clone(),
                imap_port: config.imap_port.clone(),
                imap_starttls: config.imap_starttls.unwrap_or_default(),
                imap_insecure: config.imap_insecure.unwrap_or_default(),
                imap_login: config.imap_login.clone(),
                imap_passwd_cmd: config.imap_passwd_cmd.clone(),
            }),
            #[cfg(feature = "maildir-backend")]
            DeserializedAccountConfig::Maildir(config) => {
                BackendConfig::Maildir(MaildirBackendConfig {
                    maildir_dir: shellexpand::full(&config.maildir_dir)
                        .map_err(AccountError::ExpandMaildirPathError)?
                        .to_string()
                        .into(),
                })
            }
            #[cfg(feature = "notmuch-backend")]
            DeserializedAccountConfig::Notmuch(config) => {
                BackendConfig::Notmuch(NotmuchBackendConfig {
                    notmuch_database_dir: shellexpand::full(&config.notmuch_database_dir)
                        .map_err(AccountError::ExpandNotmuchDatabasePathError)?
                        .to_string()
                        .into(),
                })
            }
        };
        trace!("backend config: {:?}", backend_config);

        info!("end: parsing account and backend configs from config and account name");
        Ok((account_config, backend_config))
    }

    /// Builds the full RFC822 compliant address of the user account.
    pub fn address(&self) -> Result<MailAddr, AccountError> {
        let has_special_chars = "()<>[]:;@.,".contains(|c| self.display_name.contains(c));
        let addr = if self.display_name.is_empty() {
            self.email.clone()
        } else if has_special_chars {
            // Wraps the name with double quotes if it contains any special character.
            format!("\"{}\" <{}>", self.display_name, self.email)
        } else {
            format!("{} <{}>", self.display_name, self.email)
        };

        Ok(mailparse::addrparse(&addr)
            .map_err(|err| AccountError::ParseAccountAddrError(err, addr.to_owned()))?
            .first()
            .ok_or_else(|| AccountError::ParseAccountAddrNotFoundError(addr.to_owned()))?
            .clone())
    }

    /// Builds the user account SMTP credentials.
    pub fn smtp_creds(&self) -> Result<SmtpCredentials, AccountError> {
        let passwd =
            process::run(&self.smtp_passwd_cmd).map_err(AccountError::GetSmtpPasswdError)?;
        let passwd = passwd
            .lines()
            .next()
            .ok_or_else(|| AccountError::GetSmtpPasswdEmptyError)?;

        Ok(SmtpCredentials::new(
            self.smtp_login.to_owned(),
            passwd.to_owned(),
        ))
    }

    /// Encrypts a file.
    pub fn pgp_encrypt_file(&self, addr: &str, path: PathBuf) -> Result<String, AccountError> {
        if let Some(cmd) = self.pgp_encrypt_cmd.as_ref() {
            let encrypt_file_cmd = format!("{} {} {:?}", cmd, addr, path);
            Ok(process::run(&encrypt_file_cmd).map_err(AccountError::EncryptFileError)?)
        } else {
            Err(AccountError::EncryptFileMissingCmdError)
        }
    }

    /// Decrypts a file.
    pub fn pgp_decrypt_file(&self, path: PathBuf) -> Result<String, AccountError> {
        if let Some(cmd) = self.pgp_decrypt_cmd.as_ref() {
            let decrypt_file_cmd = format!("{} {:?}", cmd, path);
            Ok(process::run(&decrypt_file_cmd).map_err(AccountError::DecryptFileError)?)
        } else {
            Err(AccountError::DecryptFileMissingCmdError)
        }
    }

    /// Gets the download path from a file name.
    pub fn get_download_file_path<S: AsRef<str>>(
        &self,
        file_name: S,
    ) -> Result<PathBuf, AccountError> {
        let file_path = self.downloads_dir.join(file_name.as_ref());
        self.get_unique_download_file_path(&file_path, |path, _count| path.is_file())
    }

    /// Gets the unique download path from a file name by adding
    /// suffixes in case of name conflicts.
    pub fn get_unique_download_file_path(
        &self,
        original_file_path: &PathBuf,
        is_file: impl Fn(&PathBuf, u8) -> bool,
    ) -> Result<PathBuf, AccountError> {
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
                    .ok_or_else(|| {
                        AccountError::ParseDownloadFileNameError(file_path.to_owned())
                    })?,
            ));
        }

        Ok(file_path)
    }

    /// Runs the notify command.
    pub fn run_notify_cmd<S: AsRef<str>>(&self, subject: S, sender: S) -> Result<(), AccountError> {
        let subject = subject.as_ref();
        let sender = sender.as_ref();

        let default_cmd = format!(r#"notify-send "New message from {}" "{}""#, sender, subject);
        let cmd = self
            .notify_cmd
            .as_ref()
            .map(|cmd| format!(r#"{} {:?} {:?}"#, cmd, subject, sender))
            .unwrap_or(default_cmd);

        process::run(&cmd).map_err(AccountError::StartNotifyModeError)?;
        Ok(())
    }

    /// Gets the mailbox alias if exists, otherwise returns the
    /// mailbox. Also tries to expand shell variables.
    pub fn get_mbox_alias(&self, mbox: &str) -> Result<String, AccountError> {
        let mbox = self
            .mailboxes
            .get(&mbox.trim().to_lowercase())
            .map(|s| s.as_str())
            .unwrap_or(mbox);
        let mbox = shellexpand::full(mbox)
            .map(String::from)
            .map_err(|err| AccountError::ExpandMboxAliasError(err, mbox.to_owned()))?;
        Ok(mbox)
    }
}

/// Represents all existing kind of account (backend).
#[derive(Debug, Clone)]
pub enum BackendConfig {
    #[cfg(feature = "imap-backend")]
    Imap(ImapBackendConfig),
    #[cfg(feature = "maildir-backend")]
    Maildir(MaildirBackendConfig),
    #[cfg(feature = "notmuch-backend")]
    Notmuch(NotmuchBackendConfig),
}

/// Represents the IMAP backend.
#[cfg(feature = "imap-backend")]
#[derive(Debug, Default, Clone)]
pub struct ImapBackendConfig {
    /// Represents the IMAP host.
    pub imap_host: String,
    /// Represents the IMAP port.
    pub imap_port: u16,
    /// Enables StartTLS.
    pub imap_starttls: bool,
    /// Trusts any certificate.
    pub imap_insecure: bool,
    /// Represents the IMAP login.
    pub imap_login: String,
    /// Represents the IMAP password command.
    pub imap_passwd_cmd: String,
}

#[cfg(feature = "imap-backend")]
impl ImapBackendConfig {
    /// Gets the IMAP password of the user account.
    pub fn imap_passwd(&self) -> Result<String, AccountError> {
        let passwd =
            process::run(&self.imap_passwd_cmd).map_err(AccountError::GetImapPasswdError)?;
        let passwd = passwd
            .lines()
            .next()
            .ok_or_else(|| AccountError::GetImapPasswdEmptyError)?;
        Ok(passwd.to_string())
    }
}

/// Represents the Maildir backend.
#[cfg(feature = "maildir-backend")]
#[derive(Debug, Default, Clone)]
pub struct MaildirBackendConfig {
    /// Represents the Maildir directory path.
    pub maildir_dir: PathBuf,
}

/// Represents the Notmuch backend.
#[cfg(feature = "notmuch-backend")]
#[derive(Debug, Default, Clone)]
pub struct NotmuchBackendConfig {
    /// Represents the Notmuch database path.
    pub notmuch_database_dir: PathBuf,
}

/// Represents the text/plain format as defined in the [RFC2646].
///
/// [RFC2646]: https://www.ietf.org/rfc/rfc2646.txt
#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(tag = "type", content = "width", rename_all = "lowercase")]
pub enum TextPlainFormat {
    // Forces the content width with a fixed amount of pixels.
    Fixed(usize),
    // Makes the content fit the terminal.
    Auto,
    // Does not restrict the content.
    Flowed,
}

impl Default for TextPlainFormat {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Hooks {
    pub pre_send: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_get_unique_download_file_path() {
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
