use anyhow::{anyhow, Context, Error, Result};
use lettre::transport::smtp::authentication::Credentials as SmtpCredentials;
use log::{debug, trace};
use std::{convert::TryFrom, env, fs, path::PathBuf};

use crate::{domain::config::entity::Config, output::utils::run_cmd};

const DEFAULT_PAGE_SIZE: usize = 10;
const DEFAULT_SIG_DELIM: &str = "-- \n";

/// Representation of a user account.
#[derive(Debug, Default)]
pub struct Account {
    pub name: String,
    pub from: String,
    pub downloads_dir: PathBuf,
    pub signature: String,
    pub default_page_size: usize,
    pub watch_cmds: Vec<String>,

    pub default: bool,
    pub email: String,

    pub imap_host: String,
    pub imap_port: u16,
    pub imap_starttls: bool,
    pub imap_insecure: bool,
    pub imap_login: String,
    pub imap_passwd_cmd: String,

    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_starttls: bool,
    pub smtp_insecure: bool,
    pub smtp_login: String,
    pub smtp_passwd_cmd: String,
}

impl Account {
    /// This is a little helper-function like which uses the the name and email
    /// of the account to create a valid address for the header of the headers
    /// of a msg.
    ///
    /// # Hint
    /// If the name includes some special characters like a whitespace, comma or semicolon, then
    /// the name will be automatically wrapped between two `"`.
    ///
    /// # Exapmle
    /// ```
    /// use himalaya::config::model::{Account, Config};
    ///
    /// fn main() {
    ///     let config = Config::default();
    ///
    ///     let normal_account = Account::new(Some("Acc1"), "acc1@mail.com");
    ///     // notice the semicolon in the name!
    ///     let special_account = Account::new(Some("TL;DR"), "acc2@mail.com");
    ///
    ///     // -- Expeced outputs --
    ///     let expected_normal = Account {
    ///         name: Some("Acc1".to_string()),
    ///         email: "acc1@mail.com".to_string(),
    ///         .. Account::default()
    ///     };
    ///
    ///     let expected_special = Account {
    ///         name: Some("\"TL;DR\"".to_string()),
    ///         email: "acc2@mail.com".to_string(),
    ///         .. Account::default()
    ///     };
    ///
    ///     assert_eq!(config.address(&normal_account), "Acc1 <acc1@mail.com>");
    ///     assert_eq!(config.address(&special_account), "\"TL;DR\" <acc2@mail.com>");
    /// }
    /// ```
    pub fn address(&self) -> String {
        let name = &self.from;
        let has_special_chars = "()<>[]:;@.,".contains(|special_char| name.contains(special_char));

        if name.is_empty() {
            format!("{}", self.email)
        } else if has_special_chars {
            // so the name has special characters => Wrap it with '"'
            format!("\"{}\" <{}>", name, self.email)
        } else {
            format!("{} <{}>", name, self.email)
        }
    }
    /// Returns the imap-host address + the port usage of the account
    ///
    /// # Example
    /// ```rust
    /// use himalaya::config::model::Account;
    /// fn main () {
    ///     let account = Account {
    ///         imap_host: String::from("hostExample"),
    ///         imap_port: 42,
    ///         .. Account::default()
    ///     };
    ///
    ///     let expected_output = ("hostExample", 42);
    ///
    ///     assert_eq!(account.imap_addr(), expected_output);
    /// }
    /// ```
    pub fn imap_addr(&self) -> (&str, u16) {
        debug!("host: {}", self.imap_host);
        debug!("port: {}", self.imap_port);
        (&self.imap_host, self.imap_port)
    }

    /// Runs the given command in your password string and returns it.
    pub fn imap_passwd(&self) -> Result<String> {
        let passwd = run_cmd(&self.imap_passwd_cmd).context("cannot run IMAP passwd cmd")?;
        let passwd = passwd
            .trim_end_matches(|c| c == '\r' || c == '\n')
            .to_owned();

        Ok(passwd)
    }

    pub fn smtp_creds(&self) -> Result<SmtpCredentials> {
        let passwd = run_cmd(&self.smtp_passwd_cmd).context("cannot run SMTP passwd cmd")?;
        let passwd = passwd
            .trim_end_matches(|c| c == '\r' || c == '\n')
            .to_owned();

        Ok(SmtpCredentials::new(self.smtp_login.to_owned(), passwd))
    }

    /// Creates a new account with the given values and returns it. All other attributes of the
    /// account are gonna be empty/None.
    ///
    /// # Example
    /// ```rust
    /// use himalaya::config::model::Account;
    ///
    /// fn main() {
    ///     let account1 = Account::new(Some("Name1"), "email@address.com");
    ///     let account2 = Account::new(None, "email@address.com");
    ///
    ///     let expected1 = Account {
    ///         name: Some("Name1".to_string()),
    ///         email: "email@address.com".to_string(),
    ///         .. Account::default()
    ///     };
    ///
    ///     let expected2 = Account {
    ///         email: "email@address.com".to_string(),
    ///         .. Account::default()
    ///     };
    ///
    ///     assert_eq!(account1, expected1);
    ///     assert_eq!(account2, expected2);
    /// }
    /// ```
    pub fn new<S: ToString + Default>(name: Option<S>, email_addr: S) -> Self {
        Self {
            name: name.unwrap_or_default().to_string(),
            email: email_addr.to_string(),
            ..Self::default()
        }
    }

    /// Creates a new account with a custom signature. Passing `None` to `signature` sets the
    /// signature to `Account Signature`.
    ///
    /// # Examples
    /// ```rust
    /// use himalaya::config::model::Account;
    ///
    /// fn main() {
    ///
    ///     // the testing accounts
    ///     let account_with_custom_signature = Account::new_with_signature(
    ///         Some("Email name"), "some@mail.com", Some("Custom signature! :)"));
    ///     let account_with_default_signature = Account::new_with_signature(
    ///         Some("Email name"), "some@mail.com", None);
    ///
    ///     // How they should look like
    ///     let account_cmp1 = Account {
    ///         name: Some("Email name".to_string()),
    ///         email: "some@mail.com".to_string(),
    ///         signature: Some("Custom signature! :)".to_string()),
    ///         .. Account::default()
    ///     };
    ///
    ///     let account_cmp2 = Account {
    ///         name: Some("Email name".to_string()),
    ///         email: "some@mail.com".to_string(),
    ///         .. Account::default()
    ///     };
    ///
    ///     assert_eq!(account_with_custom_signature, account_cmp1);
    ///     assert_eq!(account_with_default_signature, account_cmp2);
    /// }
    /// ```
    pub fn new_with_signature<S: AsRef<str> + ToString + Default>(
        name: Option<S>,
        email_addr: S,
        signature: Option<S>,
    ) -> Self {
        let mut account = Account::new(name, email_addr);
        account.signature = signature.unwrap_or_default().to_string();
        account
    }
}

impl<'a> TryFrom<(&'a Config, Option<&str>)> for Account {
    type Error = Error;

    fn try_from((config, account_name): (&'a Config, Option<&str>)) -> Result<Self, Self::Error> {
        debug!("init account `{}`", account_name.unwrap_or("default"));
        let (name, account) = match account_name {
            Some("") | None => config
                .accounts
                .iter()
                .find(|(_, account)| account.default.unwrap_or(false))
                .map(|(name, account)| (name.to_owned(), account))
                .ok_or_else(|| anyhow!("cannot find default account")),
            Some(name) => config
                .accounts
                .get(name)
                .map(|account| (name.to_owned(), account))
                .ok_or_else(|| anyhow!(format!("cannot find account `{}`", name))),
        }?;

        let downloads_dir = account
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
            .unwrap_or_else(|| env::temp_dir());

        let default_page_size = account
            .default_page_size
            .as_ref()
            .or_else(|| config.default_page_size.as_ref())
            .unwrap_or(&DEFAULT_PAGE_SIZE)
            .to_owned();

        let default_sig_delim = DEFAULT_SIG_DELIM.to_string();
        let signature_delim = account
            .signature_delimiter
            .as_ref()
            .or_else(|| config.signature_delimiter.as_ref())
            .unwrap_or(&default_sig_delim);
        let signature = account
            .signature
            .as_ref()
            .or_else(|| config.signature.as_ref());
        let signature = signature
            .and_then(|sig| shellexpand::full(sig).ok())
            .map(|sig| sig.to_string())
            .and_then(|sig| fs::read_to_string(sig).ok())
            .or_else(|| signature.map(|sig| sig.to_owned()))
            .map(|sig| format!("\n{}{}", signature_delim, sig))
            .unwrap_or_default();

        let account = Account {
            name,
            from: account.name.as_ref().unwrap_or(&config.name).to_owned(),
            downloads_dir,
            signature,
            default_page_size,
            watch_cmds: account
                .watch_cmds
                .as_ref()
                .or_else(|| config.watch_cmds.as_ref())
                .unwrap_or(&vec![])
                .to_owned(),
            default: account.default.unwrap_or(false),
            email: account.email.to_owned(),
            imap_host: account.imap_host.to_owned(),
            imap_port: account.imap_port,
            imap_starttls: account.imap_starttls.unwrap_or_default(),
            imap_insecure: account.imap_insecure.unwrap_or_default(),
            imap_login: account.imap_login.to_owned(),
            imap_passwd_cmd: account.imap_passwd_cmd.to_owned(),
            smtp_host: account.smtp_host.to_owned(),
            smtp_port: account.smtp_port,
            smtp_starttls: account.smtp_starttls.unwrap_or_default(),
            smtp_insecure: account.smtp_insecure.unwrap_or_default(),
            smtp_login: account.smtp_login.to_owned(),
            smtp_passwd_cmd: account.smtp_passwd_cmd.to_owned(),
        };

        trace!("{:#?}", account);
        Ok(account)
    }
}
