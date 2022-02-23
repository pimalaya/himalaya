use anyhow::{anyhow, Context, Result};
use lettre::transport::smtp::authentication::Credentials as SmtpCredentials;
use log::{debug, info, trace};
use mailparse::MailAddr;
use std::{env, ffi::OsStr, fs, path::{PathBuf, Path}};

use crate::{config::*, output::run_cmd};

/// Represents the user account.
#[derive(Debug, Default, Clone)]
pub struct AccountConfig {
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
    /// Represents the inbox folder name for this account.
    pub inbox_folder: String,
    /// Represents the sent folder name for this account.
    pub sent_folder: String,
    /// Represents the draft folder name for this account.
    pub draft_folder: String,
    /// Represents the notify command.
    pub notify_cmd: Option<String>,
    /// Overrides the default IMAP query "NEW" used to fetch new messages
    pub notify_query: String,
    /// Represents the watch commands.
    pub watch_cmds: Vec<String>,

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

impl<'a> AccountConfig {
    /// tries to create an account from a config and an optional account name.
    pub fn from_config_and_opt_account_name(
        config: &'a DeserializedConfig,
        account_name: Option<&str>,
    ) -> Result<(AccountConfig, BackendConfig)> {
        info!("begin: parsing account and backend configs from config and account name");

        debug!("account name: {:?}", account_name.unwrap_or("default"));
        let (name, account) = match account_name.map(|name| name.trim()) {
            Some("default") | Some("") | None => config
                .accounts
                .iter()
                .find(|(_, account)| match account {
                    DeserializedAccountConfig::Imap(account) => account.default.unwrap_or_default(),
                    DeserializedAccountConfig::Maildir(account) => {
                        account.default.unwrap_or_default()
                    }
                })
                .map(|(name, account)| (name.to_owned(), account))
                .ok_or_else(|| anyhow!("cannot find default account")),
            Some(name) => config
                .accounts
                .get(name)
                .map(|account| (name.to_owned(), account))
                .ok_or_else(|| anyhow!(r#"cannot find account "{}""#, name)),
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

        let account_config = AccountConfig {
            name,
            display_name: base_account
                .name
                .as_ref()
                .unwrap_or(&config.name)
                .to_owned(),
            downloads_dir,
            sig,
            default_page_size,
            inbox_folder: base_account
                .inbox_folder
                .as_deref()
                .or_else(|| config.inbox_folder.as_deref())
                .unwrap_or(DEFAULT_INBOX_FOLDER)
                .to_string(),
            sent_folder: base_account
                .sent_folder
                .as_deref()
                .or_else(|| config.sent_folder.as_deref())
                .unwrap_or(DEFAULT_SENT_FOLDER)
                .to_string(),
            draft_folder: base_account
                .draft_folder
                .as_deref()
                .or_else(|| config.draft_folder.as_deref())
                .unwrap_or(DEFAULT_DRAFT_FOLDER)
                .to_string(),
            notify_cmd: base_account.notify_cmd.clone(),
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
            DeserializedAccountConfig::Imap(config) => BackendConfig::Imap(ImapBackendConfig {
                imap_host: config.imap_host.clone(),
                imap_port: config.imap_port.clone(),
                imap_starttls: config.imap_starttls.unwrap_or_default(),
                imap_insecure: config.imap_insecure.unwrap_or_default(),
                imap_login: config.imap_login.clone(),
                imap_passwd_cmd: config.imap_passwd_cmd.clone(),
            }),
            DeserializedAccountConfig::Maildir(config) => {
                let maildir_path_str = config
                    .maildir_dir
                    .to_string_lossy();
                let expanded_path = shellexpand::full(&maildir_path_str)?
                    .into_owned();
                let expanded_path = Path::new(&expanded_path);
                BackendConfig::Maildir(MaildirBackendConfig {
                    maildir_dir: expanded_path.to_path_buf(),
                })
            }
        };
        trace!("backend config: {:?}", backend_config);

        info!("end: parsing account and backend configs from config and account name");
        Ok((account_config, backend_config))
    }

    /// Builds the full RFC822 compliant address of the user account.
    pub fn address(&self) -> Result<MailAddr> {
        let has_special_chars =
            "()<>[]:;@.,".contains(|special_char| self.display_name.contains(special_char));
        let addr = if self.display_name.is_empty() {
            self.email.clone()
        } else if has_special_chars {
            // Wraps the name with double quotes if it contains any special character.
            format!("\"{}\" <{}>", self.display_name, self.email)
        } else {
            format!("{} <{}>", self.display_name, self.email)
        };

        Ok(mailparse::addrparse(&addr)
            .context(format!(
                "cannot parse account address {:?}",
                self.display_name
            ))?
            .first()
            .ok_or_else(|| anyhow!("cannot parse account address {:?}", self.display_name))?
            .clone())
    }

    /// Builds the user account SMTP credentials.
    pub fn smtp_creds(&self) -> Result<SmtpCredentials> {
        let passwd = run_cmd(&self.smtp_passwd_cmd).context("cannot run SMTP passwd cmd")?;
        let passwd = passwd
            .trim_end_matches(|c| c == '\r' || c == '\n')
            .to_owned();

        Ok(SmtpCredentials::new(self.smtp_login.to_owned(), passwd))
    }

    /// Encrypts a file.
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

    /// Decrypts a file.
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

    /// Gets the download path from a file name.
    pub fn get_download_file_path<S: AsRef<str>>(&self, file_name: S) -> Result<PathBuf> {
        let file_path = self.downloads_dir.join(file_name.as_ref());
        self.get_unique_download_file_path(&file_path, |path, _count| path.is_file())
            .context(format!(
                "cannot get download file path of {:?}",
                file_name.as_ref()
            ))
    }

    /// Gets the unique download path from a file name by adding suffixes in case of name conflicts.
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

    /// Runs the notify command.
    pub fn run_notify_cmd<S: AsRef<str>>(&self, subject: S, sender: S) -> Result<()> {
        let subject = subject.as_ref();
        let sender = sender.as_ref();

        let default_cmd = format!(r#"notify-send "New message from {}" "{}""#, sender, subject);
        let cmd = self
            .notify_cmd
            .as_ref()
            .map(|cmd| format!(r#"{} {:?} {:?}"#, cmd, subject, sender))
            .unwrap_or(default_cmd);

        debug!("run command: {}", cmd);
        run_cmd(&cmd).context("cannot run notify cmd")?;
        Ok(())
    }
}

/// Represents all existing kind of account (backend).
#[derive(Debug, Clone)]
pub enum BackendConfig {
    Imap(ImapBackendConfig),
    Maildir(MaildirBackendConfig),
}

/// Represents the IMAP backend.
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

impl ImapBackendConfig {
    /// Gets the IMAP password of the user account.
    pub fn imap_passwd(&self) -> Result<String> {
        let passwd = run_cmd(&self.imap_passwd_cmd).context("cannot run IMAP passwd cmd")?;
        let passwd = passwd
            .trim_end_matches(|c| c == '\r' || c == '\n')
            .to_owned();
        Ok(passwd)
    }
}

/// Represents the Maildir backend.
#[derive(Debug, Default, Clone)]
pub struct MaildirBackendConfig {
    /// Represents the Maildir directory path.
    pub maildir_dir: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_get_unique_download_file_path() {
        let account = AccountConfig::default();
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
