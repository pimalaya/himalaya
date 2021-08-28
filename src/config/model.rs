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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
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
    pub fn new<S: ToString>(name: Option<S>, email_addr: S) -> Self {
        Self {
            name: name.and_then(|name| Some(name.to_string())),
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
    pub fn new_with_signature<S: AsRef<str> + ToString>(
        name: Option<S>,
        email_addr: S,
        signature: Option<S>,
    ) -> Self {

        let mut account = Account::new(name, email_addr);
        account.signature = signature.and_then(|signature| Some(signature.to_string()));
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

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub name: String,
    pub downloads_dir: Option<PathBuf>,
    pub notify_cmd: Option<String>,
    /// Option to override the default signature delimiter `--\n `.
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

    /// Parses the config file by the given path and stores the values into the struct.
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

    /// Returns the account by the given name.
    /// If `name` is `None`, then the default account is returned.
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

    /// Returns the path to the given filename in the download directory.
    /// You can imagine this as:
    /// ```skip
    /// Account-specifique-download-dir-path + Attachment-Filename
    /// ```
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

    /// This is a little helper-function like which uses the the name and email
    /// of the account to create a valid address for the header of the envelope
    /// of an msg.
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
    pub fn address(&self, account: &Account) -> String {
        let name = account.name.as_ref().unwrap_or(&self.name);

        let has_special_chars: bool = "()<>[]:;@.,".contains(|character| name.contains(character));

        if name.is_empty() {
            format!("{}", account.email)
        } else if has_special_chars {
            // so the name has special characters => Wrap it with '"'
            format!("\"{}\" <{}>", name, account.email)
        } else {
            format!("{} <{}>", name, account.email)
        }
    }

    pub fn run_notify_cmd<S: AsRef<str>>(&self, subject: S, sender: S) -> Result<()> {
        let subject = subject.as_ref();
        let sender = sender.as_ref();

        let default_cmd = format!(r#"notify-send "📫 {}" "{}""#, sender, subject);
        let cmd = self
            .notify_cmd
            .as_ref()
            .map(|cmd| format!(r#"{} {:?} {:?}"#, cmd, subject, sender))
            .unwrap_or(default_cmd);

        run_cmd(&cmd).chain_err(|| "Cannot run notify cmd")?;

        Ok(())
    }

    /// Returns the signature of the given acccount in combination witht the sigantion delimiter.
    /// If the account doesn't have a signature, then the global signature is used.
    ///
    /// # Example
    /// ```
    /// use himalaya::config::model::{Config, Account};
    ///
    /// fn main() {
    ///     let config = Config {
    ///         signature: Some("Global signature".to_string()),
    ///         .. Config::default()
    ///     };
    ///
    ///     // a config without a global signature
    ///     let config_no_global = Config::default();
    ///
    ///     let account1 = Account::new_with_signature(Some("Account Name"), "mail@address.com", Some("Cya"));
    ///     let account2 = Account::new(Some("Bruh"), "mail@address.com");
    ///
    ///     // Hint: Don't forget the default signature delimiter: '\n-- \n'
    ///     assert_eq!(config.signature(&account1), Some("\n-- \nCya".to_string()));
    ///     assert_eq!(config.signature(&account2), Some("\n-- \nGlobal signature".to_string()));
    ///     
    ///     assert_eq!(config_no_global.signature(&account2), None);
    /// }
    /// ```
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
            .map(|sig| format!("\n{}{}", sig_delim, sig))
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

#[cfg(test)]
mod tests {

    #[cfg(test)]
    mod config_test {

        use crate::config::model::{Config, Account};

        // a quick way to get a config instance for testing
        fn get_config() -> Config {
            Config {
                name: String::from("Config Name"),
                .. Config::default()
            }
        }

        #[test]
        fn test_find_account_by_name() {
            let mut config = get_config();

            let account1 = Account::new(None, "one@mail.com");
            let account2 = Account::new(Some("Two"), "two@mail.com");

            // add some accounts
            config.accounts.insert(
                "One".to_string(), account1.clone());
            config.accounts.insert(
                "Two".to_string(), account2.clone());

            let ret1 = config.find_account_by_name(Some("One")).unwrap();
            let ret2 = config.find_account_by_name(Some("Two")).unwrap();
            
            assert_eq!(*ret1, account1);
            assert_eq!(*ret2, account2);
        }

        #[test]
        fn test_address() {
            let config = get_config();

            let account1 = Account::new(None, "one@mail.com");
            let account2 = Account::new(Some("Two"), "two@mail.com");
            let account3 = Account::new(Some("TL;DR"), "three@mail.com");
            let account4 = Account::new(Some("TL,DR"), "lol@mail.com");
            let account5 = Account::new(Some("TL:DR"), "rofl@mail.com");
            let account6 = Account::new(Some("TL.DR"), "rust@mail.com");

            assert_eq!(&config.address(&account1), "Config Name <one@mail.com>");
            assert_eq!(&config.address(&account2), "Two <two@mail.com>");
            assert_eq!(&config.address(&account3), "\"TL;DR\" <three@mail.com>");
            assert_eq!(&config.address(&account4), "\"TL,DR\" <lol@mail.com>");
            assert_eq!(&config.address(&account5), "\"TL:DR\" <rofl@mail.com>");
            assert_eq!(&config.address(&account6), "\"TL.DR\" <rust@mail.com>");
        }
    }
}
