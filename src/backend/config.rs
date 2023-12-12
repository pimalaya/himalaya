#[cfg(feature = "imap")]
use email::imap::config::ImapConfig;
#[cfg(feature = "notmuch")]
use email::notmuch::config::NotmuchConfig;
#[cfg(feature = "smtp")]
use email::smtp::config::SmtpConfig;
use email::{maildir::config::MaildirConfig, sendmail::config::SendmailConfig};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendConfig {
    Maildir(MaildirConfig),
    #[cfg(feature = "imap")]
    Imap(ImapConfig),
    #[cfg(feature = "notmuch")]
    Notmuch(NotmuchConfig),
    #[cfg(feature = "smtp")]
    Smtp(SmtpConfig),
    Sendmail(SendmailConfig),
}
