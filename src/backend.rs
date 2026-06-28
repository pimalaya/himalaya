use std::fmt;

use clap::ValueEnum;

/// Selects which backend a cross-protocol command should target.
///
/// `Auto` lets the command pick the first configured-and-supported
/// backend in its own priority order. The named variants pin the
/// command to that backend; the command bails if it cannot be served
/// (config missing, or the operation has no arm for that backend).
///
/// The protocol-specific subcommands (`imap`, `jmap`, `maildir`,
/// `m2dir`, `smtp`) ignore this arg entirely.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum Backend {
    #[default]
    Auto,
    Imap,
    Jmap,
    Gmail,
    Msgraph,
    Maildir,
    M2dir,
    Smtp,
}

#[allow(unused)]
impl Backend {
    /// Whether the IMAP arm of a shared command is allowed to run.
    pub fn allows_imap(self) -> bool {
        matches!(self, Self::Auto | Self::Imap)
    }

    /// Whether the JMAP arm of a shared command is allowed to run.
    pub fn allows_jmap(self) -> bool {
        matches!(self, Self::Auto | Self::Jmap)
    }

    /// Whether the Gmail arm of a shared command is allowed to run.
    pub fn allows_gmail(self) -> bool {
        matches!(self, Self::Auto | Self::Gmail)
    }

    /// Whether the Microsoft Graph arm of a shared command is allowed to
    /// run.
    pub fn allows_msgraph(self) -> bool {
        matches!(self, Self::Auto | Self::Msgraph)
    }

    /// Whether the Maildir arm of a shared command is allowed to run.
    pub fn allows_maildir(self) -> bool {
        matches!(self, Self::Auto | Self::Maildir)
    }

    /// Whether the m2dir arm of a shared command is allowed to run.
    pub fn allows_m2dir(self) -> bool {
        matches!(self, Self::Auto | Self::M2dir)
    }

    /// Whether the SMTP arm of a shared command is allowed to run.
    pub fn allows_smtp(self) -> bool {
        matches!(self, Self::Auto | Self::Smtp)
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Imap => write!(f, "imap"),
            Self::Jmap => write!(f, "jmap"),
            Self::Gmail => write!(f, "gmail"),
            Self::Msgraph => write!(f, "msgraph"),
            Self::Maildir => write!(f, "maildir"),
            Self::M2dir => write!(f, "m2dir"),
            Self::Smtp => write!(f, "smtp"),
        }
    }
}
