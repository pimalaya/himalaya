//! # Configuration
//!
//! The TOML schema: a global block plus named account blocks, each
//! carrying the optional per-backend sub-blocks its protocols need.
//!
//! Backend defaults are duplicated here rather than read from the io-*
//! crates, so the schema compiles under any feature subset, none included.
//!
//! The configuration is a plain model every build parses whole, the
//! blocks of the backends it leaves out included, so every build accepts
//! and refuses the same file. The features turn it into runtime values
//! (TLS, proxy, SASL, server URL), so a build missing a backend leaves
//! part of the model unread.

#![cfg_attr(
    not(all(
        feature = "imap",
        feature = "smtp",
        feature = "jmap",
        feature = "gmail",
        feature = "msgraph",
        feature = "maildir",
        feature = "mbox",
        feature = "sieve",
        feature = "pimdir"
    )),
    allow(dead_code)
)]

pub mod envelope;
pub mod gmail;
pub mod imap;
pub mod jmap;
pub mod mailbox;
pub mod maildir;
pub mod mbox;
pub mod message;
pub mod msgraph;
pub mod pimdir;
pub mod proxy;
pub mod sasl;
pub mod sieve;
pub mod smtp;
pub mod tls;

use std::{collections::HashMap, path::PathBuf};

use pimalaya_config::toml::{TomlConfig, opt_shell_expanded_path};
use serde::{Deserialize, de::IgnoredAny};

use crate::config::{
    envelope::EnvelopeConfig, gmail::GmailConfig, imap::ImapConfig, jmap::JmapConfig,
    mailbox::MailboxConfig, maildir::MaildirConfig, mbox::MboxConfig, message::MessageConfig,
    msgraph::MsgraphConfig, pimdir::PimdirConfig, proxy::ProxyConfig, sieve::SieveConfig,
    smtp::SmtpConfig,
};

/// The whole TOML configuration file.
///
/// `deny_unknown_fields` is omitted so one file can be shared with
/// himalaya-tui, whose own top-level fields are ignored here.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Fallback for [`AccountConfig::display_name`].
    #[serde(alias = "from-name")]
    pub display_name: Option<String>,
    /// Fallback for [`AccountConfig::signature`].
    pub signature: Option<String>,
    /// Fallback for [`AccountConfig::signature_delim`].
    pub signature_delim: Option<String>,
    /// Directory attachments are downloaded to.
    #[serde(default, deserialize_with = "opt_shell_expanded_path")]
    pub downloads_dir: Option<PathBuf>,
    /// `envelope list` rendering options.
    #[serde(default)]
    pub envelope: EnvelopeConfig,
    /// Mailbox aliases.
    #[serde(default)]
    pub mailbox: MailboxConfig,
    /// Fallback for [`AccountConfig::message`].
    #[serde(default)]
    pub message: MessageConfig,
    /// The named `[accounts.<name>]` blocks.
    pub accounts: HashMap<String, AccountConfig>,
}

impl TomlConfig for Config {
    type Account = AccountConfig;

    fn project_name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn sample_url() -> &'static str {
        "https://github.com/pimalaya/himalaya/blob/master/config.sample.toml"
    }

    fn accounts(&self) -> impl Iterator<Item = (&str, bool)> {
        let accounts = self.accounts.iter();
        accounts.map(|(name, account)| (name.as_str(), account.default))
    }

    fn take_named_account(&mut self, name: &str) -> Option<(String, Self::Account)> {
        let (name, mut account) = self.accounts.remove_entry(name)?;
        account.inherit_proxy();
        Some((name, account))
    }
}

impl AccountConfig {
    /// Hands the account proxy to every network backend naming none of
    /// its own.
    fn inherit_proxy(&mut self) {
        let Some(proxy) = &self.proxy else {
            return;
        };

        let slots = [
            self.imap.as_mut().map(|c| &mut c.proxy),
            self.jmap.as_mut().map(|c| &mut c.proxy),
            self.gmail.as_mut().map(|c| &mut c.proxy),
            self.msgraph.as_mut().map(|c| &mut c.proxy),
            self.smtp.as_mut().map(|c| &mut c.proxy),
            self.sieve.as_mut().map(|c| &mut c.proxy),
        ];

        for slot in slots.into_iter().flatten() {
            slot.get_or_insert_with(|| proxy.clone());
        }
    }
}

/// One `[accounts.<name>]` block.
///
/// `deny_unknown_fields` is omitted so a block written for one binary
/// loads in the other: the CLI-only sub-blocks have to be tolerated by
/// himalaya-tui, which models the rest.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AccountConfig {
    /// Whether a command with no `-a/--account` runs against this one.
    #[serde(default)]
    pub default: bool,
    /// Address this account sends as, absent `--from`.
    ///
    /// Neither validated nor required, an account that never composes
    /// having no use for it. Aliased to `from`, the spelling himalaya-tui
    /// writes, the two binaries sharing one file.
    #[serde(alias = "from")]
    pub email: Option<String>,
    /// Name the `From` address carries, falling back to the global one.
    ///
    /// Quoting and encoding are the MIME builder's business, so write the
    /// name as it should read. Aliased to `from-name`, the spelling
    /// himalaya-tui writes.
    #[serde(alias = "from-name")]
    pub display_name: Option<String>,
    /// Signature appended to a composed message, falling back to the
    /// global one.
    ///
    /// The value is the signature alone, the separator before it being
    /// `signature-delim`'s business, so one written for either binary
    /// reads the same in the other.
    pub signature: Option<String>,
    /// Separator written before the signature, defaulting to the RFC 3676
    /// section 4.3 `"-- \n"`.
    ///
    /// Written verbatim, so a value meant to stand on its own line carries
    /// its own trailing newline.
    pub signature_delim: Option<String>,
    /// Directory attachments are downloaded to.
    #[serde(default, deserialize_with = "opt_shell_expanded_path")]
    pub downloads_dir: Option<PathBuf>,
    /// `envelope list` rendering options.
    #[serde(default)]
    pub envelope: EnvelopeConfig,
    /// Mailbox aliases.
    #[serde(default)]
    pub mailbox: MailboxConfig,
    /// Message sending options.
    #[serde(default)]
    pub message: MessageConfig,
    /// Proxy every network backend of this account goes through, unless
    /// its own block names one.
    #[serde(default)]
    pub proxy: Option<ProxyConfig>,
    /// The IMAP backend of this account.
    pub imap: Option<ImapConfig>,
    /// The JMAP backend of this account.
    pub jmap: Option<JmapConfig>,
    /// The Gmail backend of this account.
    pub gmail: Option<GmailConfig>,
    /// The Microsoft Graph backend of this account.
    pub msgraph: Option<MsgraphConfig>,
    /// The Maildir backend of this account.
    pub maildir: Option<MaildirConfig>,
    /// The mbox backend of this account.
    pub mbox: Option<MboxConfig>,
    /// The pimdir backend of this account.
    pub pimdir: Option<PimdirConfig>,
    /// The SMTP transport of this account.
    pub smtp: Option<SmtpConfig>,
    /// The ManageSieve endpoint of this account.
    pub sieve: Option<SieveConfig>,
    /// The v1 `backend` table, kept only to point at MIGRATION.md.
    #[serde(default)]
    pub backend: Option<IgnoredAny>,
    /// The `m2dir` block, whose backend was removed, kept only to point
    /// at MIGRATION.md.
    #[serde(default)]
    pub m2dir: Option<IgnoredAny>,
}

#[cfg(test)]
mod tests {
    use std::{env::var, fs, io::Write};

    use pimalaya_config::sample::{Variant, variants};
    use tempfile::NamedTempFile;

    use crate::{backend::Backend, config::*};

    /// Run after run the map order changes, so the error has to come
    /// whichever default the iteration would have met first.
    #[test]
    fn several_defaults_are_refused_unless_an_account_is_named() {
        let toml = "[accounts.work]\ndefault = true\n\
                    [accounts.perso]\ndefault = true\n\
                    [accounts.spare]\n";

        let mut config: Config = toml::from_str(toml).unwrap();
        let err = config.take_account(None).unwrap_err().to_string();
        assert!(err.contains("(perso, work)"), "{err}");

        let mut config: Config = toml::from_str(toml).unwrap();
        let err = config.take_account(Some("default")).unwrap_err();
        assert!(err.to_string().starts_with("Several accounts"), "{err}");

        let mut config: Config = toml::from_str(toml).unwrap();
        let (name, _) = config.take_account(Some("spare")).unwrap();
        assert_eq!(name, "spare");
    }

    /// An `m2dir` block loads as if absent, whatever it holds, and the
    /// error of an account left with no backend names its removal.
    #[test]
    fn m2dir_block_names_its_removal() {
        let toml =
            "[accounts.example]\ndefault = true\nm2dir.root = \"~/Mail\"\nm2dir.unknown = true\n";
        let mut config: Config = toml::from_str(toml).unwrap();
        let (name, account) = config.take_account(None).unwrap();

        let err = account.no_backend_error(&name, Backend::Auto, &[]);
        let err = err.to_string();
        assert!(err.contains("`m2dir` block is ignored"), "{err}");
        assert!(err.contains("MIGRATION.md"), "{err}");
    }

    /// himalaya-tui spells the identity `from` and `from-name`, and one
    /// file backs both binaries, so a config it wrote must reach the same
    /// fields the composers read.
    #[test]
    fn the_composing_config_reads_under_the_tui_spelling() {
        let config: Config = toml::from_str(
            r#"
            from-name = "Alice"
            signature = "Alice"

            [accounts.example]
            from = "alice@example.org"
            from-name = "Alice at work"
            signature-delim = "~~~\n"
            "#,
        )
        .expect("the himalaya-tui spelling must deserialize");

        let account = config.accounts.get("example").expect("the example account");

        assert_eq!(config.display_name.as_deref(), Some("Alice"));
        assert_eq!(config.signature.as_deref(), Some("Alice"));
        assert_eq!(account.email.as_deref(), Some("alice@example.org"));
        assert_eq!(account.display_name.as_deref(), Some("Alice at work"));
        assert_eq!(account.signature_delim.as_deref(), Some("~~~\n"));
    }

    #[test]
    fn backends_inherit_the_account_proxy_unless_they_name_one() {
        let mut config: Config = toml::from_str(
            r#"
            [accounts.example]
            proxy.url = "socks5h://account:1080"
            imap.server = "imap.example.org"
            smtp.server = "smtp.example.org"
            smtp.proxy.url = "http://smtp:3128"
            "#,
        )
        .expect("the proxy blocks must deserialize");

        let (_, account) = config.take_named_account("example").unwrap();
        let proxy = |proxy: Option<ProxyConfig>| proxy.map(|proxy| proxy.url);

        assert_eq!(
            proxy(account.imap.unwrap().proxy).as_deref(),
            Some("socks5h://account:1080")
        );
        assert_eq!(
            proxy(account.smtp.unwrap().proxy).as_deref(),
            Some("http://smtp:3128")
        );
    }

    #[test]
    fn local_roots_expand_the_leading_tilde() {
        let home = PathBuf::from(var("HOME").expect("HOME must be set"));

        let maildir: MaildirConfig = toml::from_str(r#"root = "~/Mail""#).unwrap();
        let pimdir: PimdirConfig = toml::from_str(r#"root = "~/Mail""#).unwrap();
        let mbox: MboxConfig = toml::from_str(
            r#"root = "~/Mail"
inbox = "~/spool""#,
        )
        .unwrap();

        assert_eq!(maildir.root, home.join("Mail"));
        assert_eq!(pimdir.root, home.join("Mail"));
        assert_eq!(mbox.root, home.join("Mail"));
        assert_eq!(mbox.inbox, Some(home.join("spool")));
    }

    /// The sections of config.sample.toml, one of which a user
    /// uncomments.
    const SECTIONS: [&str; 10] = [
        "# IMAP and SMTP",
        "# ManageSieve",
        "# JMAP",
        "# Gmail",
        "# Microsoft Graph",
        "# Microsoft IMAP and SMTP",
        "# Fastmail",
        "# Maildir",
        "# mbox",
        "# pimdir",
    ];

    /// Checks config.sample.toml: one `[accounts.example]` table and no
    /// other, its sections each parsed through the loader `-c` uses with
    /// every variant on top, `example` resolving as the default account
    /// and declaring a backend.
    #[test]
    fn the_sample_configuration_parses_with_each_variant() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.sample.toml");
        let sample = fs::read_to_string(&path).unwrap();
        let headers: Vec<_> = sample.lines().filter(|l| l.starts_with('[')).collect();
        assert_eq!(headers, ["[accounts.example]"]);

        for Variant { name, text } in variants(&sample, &SECTIONS) {
            let context = format!("config.sample.toml with {name} uncommented");

            let mut file = NamedTempFile::new().unwrap();
            file.write_all(text.as_bytes()).unwrap();

            let mut config = Config::load(&[file.path().to_path_buf()])
                .unwrap_or_else(|err| panic!("{context}: {err:?}"));
            let (account, account_config) = config
                .take_account(None)
                .unwrap_or_else(|err| panic!("{context}: {err:?}"));

            let blocks = [
                account_config.imap.is_some(),
                account_config.jmap.is_some(),
                account_config.gmail.is_some(),
                account_config.msgraph.is_some(),
                account_config.maildir.is_some(),
                account_config.mbox.is_some(),
                account_config.pimdir.is_some(),
                account_config.sieve.is_some(),
            ];

            assert_eq!(account, "example", "{context}");
            assert!(blocks.contains(&true), "{context}: no backend declared");
        }
    }
}
