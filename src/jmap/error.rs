use io_jmap::rfc8621::{
    email::{JmapEmailCopyItemError, JmapEmailImportItemError, JmapEmailSetItemError},
    email_submission::JmapEmailSubmissionSetItemError,
    identity::JmapIdentitySetItemError,
    mailbox::JmapMailboxSetItemError,
};

/// Returns the optional human-readable description carried by a JMAP set error.
pub trait JmapSetError {
    fn description(&self) -> Option<&str>;
    fn properties(&self) -> &[String];
    fn type_name(&self) -> &'static str;
}

/// Renders a JMAP `*Set`-style error suffix like
/// `: invalidProperties (\`name\`) — too long`.
pub fn format_set_error<E: JmapSetError>(err: &E) -> String {
    let mut msg = format!(": {}", err.type_name());

    if !err.properties().is_empty() {
        msg.push_str(" (`");
        msg.push_str(&err.properties().join("`, `"));
        msg.push(')');
    }

    if let Some(desc) = err.description() {
        msg.push_str(" — ");
        msg.push_str(desc.trim_end_matches(['.', '\n']));
    }

    msg
}

impl JmapSetError for JmapMailboxSetItemError {
    fn description(&self) -> Option<&str> {
        match self {
            Self::MailboxHasChild { description }
            | Self::MailboxHasEmail { description }
            | Self::NotFound { description }
            | Self::InvalidPatch { description }
            | Self::WillDestroy { description }
            | Self::InvalidProperties { description, .. }
            | Self::Singleton { description } => description.as_deref(),
            Self::Unknown => None,
        }
    }

    fn properties(&self) -> &[String] {
        match self {
            Self::InvalidProperties { properties, .. } => properties,
            _ => &[],
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Self::MailboxHasChild { .. } => "mailboxHasChild",
            Self::MailboxHasEmail { .. } => "mailboxHasEmail",
            Self::NotFound { .. } => "notFound",
            Self::InvalidPatch { .. } => "invalidPatch",
            Self::WillDestroy { .. } => "willDestroy",
            Self::InvalidProperties { .. } => "invalidProperties",
            Self::Singleton { .. } => "singleton",
            Self::Unknown => "unknown",
        }
    }
}

impl JmapSetError for JmapEmailSetItemError {
    fn description(&self) -> Option<&str> {
        match self {
            Self::TooManyKeywords { description }
            | Self::TooManyMailboxes { description }
            | Self::BlobNotFound { description }
            | Self::NotFound { description }
            | Self::InvalidPatch { description }
            | Self::WillDestroy { description }
            | Self::InvalidProperties { description, .. }
            | Self::Singleton { description } => description.as_deref(),
            Self::Unknown => None,
        }
    }

    fn properties(&self) -> &[String] {
        match self {
            Self::InvalidProperties { properties, .. } => properties,
            _ => &[],
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Self::TooManyKeywords { .. } => "tooManyKeywords",
            Self::TooManyMailboxes { .. } => "tooManyMailboxes",
            Self::BlobNotFound { .. } => "blobNotFound",
            Self::NotFound { .. } => "notFound",
            Self::InvalidPatch { .. } => "invalidPatch",
            Self::WillDestroy { .. } => "willDestroy",
            Self::InvalidProperties { .. } => "invalidProperties",
            Self::Singleton { .. } => "singleton",
            Self::Unknown => "unknown",
        }
    }
}

impl JmapSetError for JmapEmailImportItemError {
    fn description(&self) -> Option<&str> {
        match self {
            Self::InvalidEmail { description }
            | Self::NotFound { description }
            | Self::InvalidProperties { description, .. } => description.as_deref(),
            Self::Unknown => None,
        }
    }

    fn properties(&self) -> &[String] {
        match self {
            Self::InvalidProperties { properties, .. } => properties,
            _ => &[],
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Self::InvalidEmail { .. } => "invalidEmail",
            Self::NotFound { .. } => "notFound",
            Self::InvalidProperties { .. } => "invalidProperties",
            Self::Unknown => "unknown",
        }
    }
}

impl JmapSetError for JmapEmailCopyItemError {
    fn description(&self) -> Option<&str> {
        match self {
            Self::AlreadyExists { description }
            | Self::NotFound { description }
            | Self::InvalidProperties { description, .. } => description.as_deref(),
            Self::Unknown => None,
        }
    }

    fn properties(&self) -> &[String] {
        match self {
            Self::InvalidProperties { properties, .. } => properties,
            _ => &[],
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Self::AlreadyExists { .. } => "alreadyExists",
            Self::NotFound { .. } => "notFound",
            Self::InvalidProperties { .. } => "invalidProperties",
            Self::Unknown => "unknown",
        }
    }
}

impl JmapSetError for JmapIdentitySetItemError {
    fn description(&self) -> Option<&str> {
        match self {
            Self::NotFound { description }
            | Self::InvalidPatch { description }
            | Self::WillDestroy { description }
            | Self::InvalidProperties { description, .. }
            | Self::Singleton { description } => description.as_deref(),
            Self::Unknown => None,
        }
    }

    fn properties(&self) -> &[String] {
        match self {
            Self::InvalidProperties { properties, .. } => properties,
            _ => &[],
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "notFound",
            Self::InvalidPatch { .. } => "invalidPatch",
            Self::WillDestroy { .. } => "willDestroy",
            Self::InvalidProperties { .. } => "invalidProperties",
            Self::Singleton { .. } => "singleton",
            Self::Unknown => "unknown",
        }
    }
}

impl JmapSetError for JmapEmailSubmissionSetItemError {
    fn description(&self) -> Option<&str> {
        match self {
            Self::TooManyRecipients { description }
            | Self::NoRecipients { description }
            | Self::InvalidRecipients { description }
            | Self::ForbiddenMailFrom { description }
            | Self::ForbiddenFrom { description }
            | Self::ForbiddenToSend { description }
            | Self::CannotUnsendMessage { description }
            | Self::InvalidEmail { description }
            | Self::NotFound { description }
            | Self::InvalidProperties { description, .. } => description.as_deref(),
            Self::Unknown => None,
        }
    }

    fn properties(&self) -> &[String] {
        match self {
            Self::InvalidProperties { properties, .. } => properties,
            _ => &[],
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Self::TooManyRecipients { .. } => "tooManyRecipients",
            Self::NoRecipients { .. } => "noRecipients",
            Self::InvalidRecipients { .. } => "invalidRecipients",
            Self::ForbiddenMailFrom { .. } => "forbiddenMailFrom",
            Self::ForbiddenFrom { .. } => "forbiddenFrom",
            Self::ForbiddenToSend { .. } => "forbiddenToSend",
            Self::CannotUnsendMessage { .. } => "cannotUnsendMessage",
            Self::InvalidEmail { .. } => "invalidEmail",
            Self::NotFound { .. } => "notFound",
            Self::InvalidProperties { .. } => "invalidProperties",
            Self::Unknown => "unknown",
        }
    }
}
