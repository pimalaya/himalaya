use anyhow::{anyhow, Context, Error, Result};
use lettre::transport::smtp::authentication::Credentials as SmtpCredentials;
use log::{debug, trace};
use std::{convert::TryFrom, env, fs, path::PathBuf};

use crate::{
    config::{Config, DEFAULT_PAGE_SIZE, DEFAULT_SIG_DELIM},
    output::run_cmd,
};

/// Represent a user account.
#[derive(Debug, Default)]
pub struct Account {
    pub name: String,
    pub from: String,
    pub downloads_dir: PathBuf,
    pub sig: Option<String>,
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
                .ok_or_else(|| anyhow!("cannot find account `{}`", name)),
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
        let sig_delim = account
            .signature_delimiter
            .as_ref()
            .or_else(|| config.signature_delimiter.as_ref())
            .unwrap_or(&default_sig_delim);
        let sig = account
            .signature
            .as_ref()
            .or_else(|| config.signature.as_ref());
        let sig = sig
            .and_then(|sig| shellexpand::full(sig).ok())
            .map(String::from)
            .and_then(|sig| fs::read_to_string(sig).ok())
            .or_else(|| sig.map(|sig| sig.to_owned()))
            .map(|sig| format!("{}{}", sig_delim, sig.trim_end()));

        let account = Account {
            name,
            from: account.name.as_ref().unwrap_or(&config.name).to_owned(),
            downloads_dir,
            sig,
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
