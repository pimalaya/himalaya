#[cfg(feature = "imap")]
use email::imap::config::ImapConfig;
#[cfg(feature = "maildir")]
use email::maildir::config::MaildirConfig;
#[cfg(feature = "notmuch")]
use email::notmuch::config::NotmuchConfig;
#[cfg(feature = "sendmail")]
use email::sendmail::config::SendmailConfig;
#[cfg(feature = "smtp")]
use email::smtp::config::SmtpConfig;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendConfig {
    #[cfg(feature = "imap")]
    Imap(ImapConfig),
    #[cfg(feature = "maildir")]
    Maildir(MaildirConfig),
    #[cfg(feature = "notmuch")]
    Notmuch(NotmuchConfig),
    #[cfg(feature = "smtp")]
    Smtp(SmtpConfig),
    #[cfg(feature = "sendmail")]
    Sendmail(SendmailConfig),
}
