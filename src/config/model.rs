use error_chain::error_chain;
use lettre::transport::smtp::authentication::Credentials as SmtpCredentials;
use log::debug;
use serde::Deserialize;
use shellexpand;
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::Read,
    path::PathBuf,
    thread,
};
use toml;

use crate::output::utils::run_cmd;

error_chain! {}

const DEFAULT_PAGE_SIZE: usize = 10;

// Account

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Account {
    // Override
    pub name: Option<String>,
    pub downloads_dir: Option<PathBuf>,
    pub signature_delimiter: Option<String>,
    pub signature: Option<String>,
    pub default_page_size: Option<usize>,
    pub watch_cmds: Option<Vec<String>>,

    // Specific
    pub default: Option<bool>,
    pub email: String,

    pub imap_host: String,
    pub imap_port: u16,
    pub imap_starttls: Option<bool>,
    pub imap_insecure: Option<bool>,
    pub imap_login: String,
    pub imap_passwd_cmd: String,

    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_starttls: Option<bool>,
    pub smtp_insecure: Option<bool>,
    pub smtp_login: String,
    pub smtp_passwd_cmd: String,
}

impl Account {
    pub fn imap_addr(&self) -> (&str, u16) {
        debug!("host: {}", self.imap_host);
        debug!("port: {}", self.imap_port);
        (&self.imap_host, self.imap_port)
    }

    pub fn imap_passwd(&self) -> Result<String> {
        let passwd = run_cmd(&self.imap_passwd_cmd).chain_err(|| "Cannot run IMAP passwd cmd")?;
        let passwd = passwd
            .trim_end_matches(|c| c == '\r' || c == '\n')
            .to_owned();

        Ok(passwd)
    }

    pub fn imap_starttls(&self) -> bool {
        let starttls = match self.imap_starttls {
            Some(true) => true,
            _ => false,
        };

        debug!("STARTTLS: {}", starttls);
        starttls
    }

    pub fn imap_insecure(&self) -> bool {
        let insecure = match self.imap_insecure {
            Some(true) => true,
            _ => false,
        };

        debug!("insecure: {}", insecure);
        insecure
    }

    pub fn smtp_creds(&self) -> Result<SmtpCredentials> {
        let passwd = run_cmd(&self.smtp_passwd_cmd).chain_err(|| "Cannot run SMTP passwd cmd")?;
        let passwd = passwd
            .trim_end_matches(|c| c == '\r' || c == '\n')
            .to_owned();

        Ok(SmtpCredentials::new(self.smtp_login.to_owned(), passwd))
    }

    pub fn smtp_starttls(&self) -> bool {
        match self.smtp_starttls {
            Some(true) => true,
            _ => false,
        }
    }

    pub fn smtp_insecure(&self) -> bool {
        match self.smtp_insecure {
            Some(true) => true,
            _ => false,
        }
    }

    /// This is a little helper-function like which uses the the name and email
    /// of the account to create a valid address for the header of the envelope
    /// of an msg.
    ///
    /// # Example 1: With name
    /// Suppose the name field in the account struct *has* a value:
    ///
    /// ```rust
    /// use himalaya::config::model::Account;
    ///
    /// fn main() {
    ///     let account = Account {
    ///         // we just need those two values
    ///         name: Some(String::from("Name")),
    ///         email: String::from("BestEmail@Ever.lol"),
    ///         ..Account::default()
    ///     };
    ///
    ///     // get the address of the account
    ///     let address = account.get_full_address();
    ///
    ///     assert_eq!("Name <BestEmail@Ever.lol>".to_string(), address);
    /// }
    /// ```
    ///
    /// # Example 2: Without name
    /// Suppose the name field in the account-struct *hasn't* a value:
    ///
    /// ```rust
    /// use himalaya::config::model::Account;
    ///
    /// fn main() {
    ///     let account = Account {
    ///         // we just need those two values
    ///         name: None,
    ///         email: String::from("BestEmail@Ever.lol"),
    ///         ..Account::default()
    ///     };
    ///
    ///     // get the address of the account
    ///     let address = account.get_full_address();
    ///
    ///     assert_eq!("BestEmail@Ever.lol".to_string(), address);
    /// }
    /// ```
    pub fn get_full_address(&self) -> String {
        if let Some(name) = &self.name {
            format!("{} <{}>", name, self.email)
        } else {
            format!("{}", self.email)
        }
    }

    pub fn new(name: Option<&str>, email_addr: &str) -> Self {
        Self {
            name: name.and_then(|name| Some(name.to_string())),
            downloads_dir: Some(PathBuf::from(r"/tmp")),
            signature: None,
            signature_delimiter: None,
            default_page_size: Some(42),
            default: Some(true),
            email: email_addr.into(),
            watch_cmds: Some(vec!["mbsync".to_string(), "-a".to_string()]),
            imap_host: String::from("localhost"),
            imap_port: 3993,
            imap_starttls: Some(false),
            imap_insecure: Some(true),
            imap_login: email_addr.into(),
            imap_passwd_cmd: String::from("echo 'password'"),
            smtp_host: String::from("localhost"),
            smtp_port: 3465,
            smtp_starttls: Some(false),
            smtp_insecure: Some(true),
            smtp_login: email_addr.into(),
            smtp_passwd_cmd: String::from("echo 'password'"),
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
    ///         Some("Email name"),
    ///         "some@mail.com",
    ///         Some("Custom signature! :)")
    ///     );
    ///
    ///     let account_with_default_signature = Account::new_with_signature(
    ///         Some("Email name"),
    ///         "some@mail.com",
    ///         None
    ///     );
    ///
    ///     // How they should look like
    ///     let account_comp1 = Account {
    ///         name: Some("Email name".to_string()),
    ///         email: "some@mail.com".to_string(),
    ///         signature: Some("Custom signature! :)".to_string()),
    ///         .. Account::default()
    ///     };
    ///
    ///     let account_cmp2 = Account {
    ///         name: Some("Email name".to_string()),
    ///         email: "some@mail.com".to_string(),
    ///         signature: Some("Account Signature"),
    ///         .. Account::default()
    ///     };
    ///
    ///     assert_eq!(account_with_custom_signature, account_cmp1);
    ///     assert_eq!(account_with_default_signature, account_cmp2);
    /// }
    /// ```
    pub fn new_with_signature(
        name: Option<&str>,
        email_addr: &str,
        signature: Option<&str>,
    ) -> Self {
        let mut account = Account::new(name, email_addr);

        // Use the default signature "Account Signature", if the programmer didn't provide a custom
        // one.
        if let Some(signature) = signature {
            account.signature = Some(signature.to_string());
        } else {
            account.signature = Some(String::from("Account Signature"));
        }
        account
    }
}

impl Default for Account {
    fn default() -> Self {
        Self {
            name: None,
            downloads_dir: None,
            signature_delimiter: None,
            signature: None,
            default_page_size: None,
            default: None,
            email: String::new(),
            watch_cmds: None,
            imap_host: String::new(),
            imap_port: 0,
            imap_starttls: None,
            imap_insecure: None,
            imap_login: String::new(),
            imap_passwd_cmd: String::new(),
            smtp_host: String::new(),
            smtp_port: 0,
            smtp_starttls: None,
            smtp_insecure: None,
            smtp_login: String::new(),
            smtp_passwd_cmd: String::new(),
        }
    }
}

// Config

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub name: String,
    pub downloads_dir: Option<PathBuf>,
    pub notify_cmd: Option<String>,
    pub signature_delimiter: Option<String>,
    pub signature: Option<String>,
    pub default_page_size: Option<usize>,
    pub watch_cmds: Option<Vec<String>>,
    #[serde(flatten)]
    pub accounts: HashMap<String, Account>,
}

impl Config {
    fn path_from_xdg() -> Result<PathBuf> {
        let path =
            env::var("XDG_CONFIG_HOME").chain_err(|| "Cannot find `XDG_CONFIG_HOME` env var")?;
        let mut path = PathBuf::from(path);
        path.push("himalaya");
        path.push("config.toml");

        Ok(path)
    }

    fn path_from_xdg_alt() -> Result<PathBuf> {
        let home_var = if cfg!(target_family = "windows") {
            "USERPROFILE"
        } else {
            "HOME"
        };
        let mut path: PathBuf = env::var(home_var)
            .chain_err(|| format!("Cannot find `{}` env var", home_var))?
            .into();
        path.push(".config");
        path.push("himalaya");
        path.push("config.toml");

        Ok(path)
    }

    fn path_from_home() -> Result<PathBuf> {
        let home_var = if cfg!(target_family = "windows") {
            "USERPROFILE"
        } else {
            "HOME"
        };
        let mut path: PathBuf = env::var(home_var)
            .chain_err(|| format!("Cannot find `{}` env var", home_var))?
            .into();
        path.push(".himalayarc");

        Ok(path)
    }

    pub fn new(path: Option<PathBuf>) -> Result<Self> {
        let path = match path {
            Some(path) => path,
            None => Self::path_from_xdg()
                .or_else(|_| Self::path_from_xdg_alt())
                .or_else(|_| Self::path_from_home())
                .chain_err(|| "Cannot find config path")?,
        };

        let mut file = File::open(path).chain_err(|| "Cannot open config file")?;
        let mut content = vec![];
        file.read_to_end(&mut content)
            .chain_err(|| "Cannot read config file")?;

        Ok(toml::from_slice(&content).chain_err(|| "Cannot parse config file")?)
    }

    pub fn find_account_by_name(&self, name: Option<&str>) -> Result<&Account> {
        match name {
            Some("") | None => self
                .accounts
                .iter()
                .find(|(_, account)| account.default.unwrap_or(false))
                .map(|(_, account)| account)
                .ok_or_else(|| "Cannot find default account".into()),
            Some(name) => self
                .accounts
                .get(name)
                .ok_or_else(|| format!("Cannot find account `{}`", name).into()),
        }
    }

    pub fn downloads_filepath(&self, account: &Account, filename: &str) -> PathBuf {
        account
            .downloads_dir
            .as_ref()
            .and_then(|dir| dir.to_str())
            .and_then(|dir| shellexpand::full(dir).ok())
            .map(|dir| PathBuf::from(dir.to_string()))
            .unwrap_or(
                self.downloads_dir
                    .as_ref()
                    .and_then(|dir| dir.to_str())
                    .and_then(|dir| shellexpand::full(dir).ok())
                    .map(|dir| PathBuf::from(dir.to_string()))
                    .unwrap_or(env::temp_dir()),
            )
            .join(filename)
    }

    pub fn address(&self, account: &Account) -> String {
        let name = account.name.as_ref().unwrap_or(&self.name);
        format!("{} <{}>", name, account.email)
    }

    pub fn run_notify_cmd(&self, subject: &str, sender: &str) -> Result<()> {
        let default_cmd = format!(r#"notify-send "📫 {}" "{}""#, sender, subject);
        let cmd = self
            .notify_cmd
            .as_ref()
            .map(|cmd| format!(r#"{} {:?} {:?}"#, cmd, subject, sender))
            .unwrap_or(default_cmd);

        run_cmd(&cmd).chain_err(|| "Cannot run notify cmd")?;

        Ok(())
    }

    pub fn signature(&self, account: &Account) -> Option<String> {
        let default_sig_delim = String::from("-- \n");
        let sig_delim = account
            .signature_delimiter
            .as_ref()
            .or_else(|| self.signature_delimiter.as_ref())
            .unwrap_or(&default_sig_delim);
        let sig = account
            .signature
            .as_ref()
            .or_else(|| self.signature.as_ref());
        sig.and_then(|sig| shellexpand::full(sig).ok())
            .map(|sig| sig.to_string())
            .and_then(|sig| fs::read_to_string(sig).ok())
            .or_else(|| sig.map(|sig| sig.to_owned()))
            .map(|sig| sig_delim.to_owned() + sig.as_ref())
    }

    pub fn default_page_size(&self, account: &Account) -> usize {
        account
            .default_page_size
            .as_ref()
            .or_else(|| self.default_page_size.as_ref())
            .or(Some(&DEFAULT_PAGE_SIZE))
            .unwrap()
            .to_owned()
    }

    pub fn exec_watch_cmds(&self, account: &Account) -> Result<()> {
        let cmds = account
            .watch_cmds
            .as_ref()
            .or_else(|| self.watch_cmds.as_ref())
            .map(|cmds| cmds.to_owned())
            .unwrap_or_default();

        thread::spawn(move || {
            debug!("batch execution of {} cmd(s)", cmds.len());
            cmds.iter().for_each(|cmd| {
                debug!("running command {:?}…", cmd);
                let res = run_cmd(cmd);
                debug!("{:?}", res);
            })
        });

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: String::new(),
            downloads_dir: None,
            notify_cmd: None,
            signature_delimiter: None,
            signature: None,
            default_page_size: None,
            watch_cmds: None,
            accounts: HashMap::new(),
        }
    }
}
