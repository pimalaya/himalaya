#[cfg(feature = "imap-backend")]
use email::imap::ImapConfig;
#[cfg(feature = "notmuch-backend")]
use email::notmuch::NotmuchConfig;
#[cfg(feature = "smtp-sender")]
use email::smtp::SmtpConfig;
use email::{maildir::MaildirConfig, sendmail::SendmailConfig};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendConfig {
    Maildir(MaildirConfig),
    #[cfg(feature = "imap-backend")]
    Imap(ImapConfig),
    #[cfg(feature = "notmuch-backend")]
    Notmuch(NotmuchConfig),
    #[cfg(feature = "smtp-sender")]
    Smtp(SmtpConfig),
    Sendmail(SendmailConfig),
}
