use clap::Parser;

const INBOX: &str = "INBOX";

/// The optional maildir name argument parser.
#[derive(Debug, Parser)]
pub struct MaildirNameOptionalArg {
    /// The name of the maildir.
    #[arg(name = "maildir_name", value_name = "MAILDIR", default_value = INBOX)]
    pub inner: String,
}

impl Default for MaildirNameOptionalArg {
    fn default() -> Self {
        Self {
            inner: INBOX.into(),
        }
    }
}

/// The optional maildir name flag parser.
#[derive(Debug, Parser)]
pub struct MaildirPathOptionalFlag {
    /// The name of the maildir.
    #[arg(long = "maildir", short = 'm')]
    #[arg(name = "maildir_name", value_name = "NAME", default_value = INBOX)]
    pub inner: String,
}

impl Default for MaildirPathOptionalFlag {
    fn default() -> Self {
        Self {
            inner: INBOX.into(),
        }
    }
}

#[derive(Debug, Parser)]
pub struct MaildirNoSelectFlag {
    /// Do not select the given maildir before performing the current
    /// action.
    ///
    /// This argument is useful when stateful IMAP sessions are used,
    /// for example with Sirup CLI:
    ///
    /// https://github.com/pimalaya/sirup
    #[arg(long = "no-select", default_value_t)]
    #[arg(name = "maildir_no_select")]
    pub inner: bool,
}

/// The required maildir name argument parser.
#[derive(Debug, Parser)]
pub struct MaildirNameArg {
    /// The name of the maildir.
    #[arg(name = "maildir_name", value_name = "MAILDIR")]
    pub inner: String,
}

/// The target maildir name argument parser.
#[derive(Debug, Clone, Parser)]
pub struct TargetMaildirNameArg {
    /// The name of the target maildir.
    #[arg(name = "target_maildir_name", value_name = "TARGET")]
    pub inner: String,
}
