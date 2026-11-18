//! # JMAP error
//!
//! Renders the `SetError` a JMAP method returns into the message a
//! command fails with, codes what the server refused, and reads a refusal
//! into the result a `jmap` command prints.

use std::fmt;

use anyhow::Error;
use io_jmap::{
    client::JmapClientStdError,
    rfc8620::{
        blob_download::JmapBlobDownloadError,
        blob_upload::JmapBlobUploadError,
        error::{JmapMethodError, JmapRequestError},
        send::JmapSendError,
        session_get::JmapSessionGetError,
    },
    rfc8621::{
        email::{
            copy::JmapEmailCopyItemError, import::JmapEmailImportItemError,
            set::JmapEmailSetItemError,
        },
        email_submission::JmapEmailSubmissionSetItemError,
        identity::set::JmapIdentitySetItemError,
        mailbox::set::JmapMailboxSetItemError,
    },
};
use pimalaya_cli::sanitize;
use serde::Serialize;

use crate::{
    error::{ErrorCode, ItemError},
    shared::refusal::Refusal,
};

/// Returns the optional human-readable description carried by a JMAP set error.
pub trait JmapSetError {
    fn description(&self) -> Option<&str>;
    fn properties(&self) -> &[String];
    fn type_name(&self) -> &str;
}

/// The code of a refusal by its SetError type (RFC 8620 §5.3, RFC 8621
/// §4.6): `forbidden` is `permission-denied`, `overQuota`
/// `quota-exceeded`, `rateLimit` `rate-limited`.
fn set_error_code(type_name: &str) -> Option<ErrorCode> {
    match type_name {
        "forbidden" => Some(ErrorCode::PermissionDenied),
        "overQuota" => Some(ErrorCode::QuotaExceeded),
        "rateLimit" => Some(ErrorCode::RateLimited),
        _ => None,
    }
}

/// The one write of a whole command the server refused, worded by the
/// command, its SetError kept for the code and the refusal it reads as.
#[derive(Debug)]
pub struct JmapSetRefusal {
    message: String,
    type_name: String,
    description: Option<String>,
}

impl fmt::Display for JmapSetRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for JmapSetRefusal {}

/// The failure of a whole command whose one write the server refused,
/// worded `message`, coded by the type of `err`.
pub fn set_error<E: JmapSetError>(message: String, err: &E) -> Error {
    JmapSetRefusal {
        message,
        type_name: err.type_name().to_owned(),
        description: err.description().map(ToOwned::to_owned),
    }
    .into()
}

/// The code of a JMAP refusal in an error chain: an HTTP 429 is
/// `rate-limited` and a 403 `permission-denied`, whichever request it
/// answered, as is a method error `forbidden` (RFC 8620 §3.6.2), and a
/// refused write is coded by its SetError type.
pub(crate) fn refusal_code(cause: &(dyn std::error::Error + 'static)) -> Option<ErrorCode> {
    let cause = unwrapped(cause);
    if let Some(err) = cause.downcast_ref::<JmapSetRefusal>() {
        return set_error_code(&err.type_name);
    }

    let status = if let Some(err) = cause.downcast_ref::<JmapSendError>() {
        match err {
            JmapSendError::HttpStatus(status) | JmapSendError::Request { status, .. } => *status,
            _ => return None,
        }
    } else if let Some(err) = cause.downcast_ref::<JmapMethodError>() {
        return matches!(err, JmapMethodError::Forbidden { .. })
            .then_some(ErrorCode::PermissionDenied);
    } else if let Some(JmapSessionGetError::HttpStatus(status)) = cause.downcast_ref() {
        *status
    } else if let Some(JmapBlobDownloadError::HttpStatus(status)) = cause.downcast_ref() {
        *status
    } else if let Some(JmapBlobUploadError::HttpStatus(status)) = cause.downcast_ref() {
        *status
    } else {
        return None;
    };

    match status {
        429 => Some(ErrorCode::RateLimited),
        403 => Some(ErrorCode::PermissionDenied),
        _ => None,
    }
}

/// The refusal of a JMAP error in a chain, for the `jmap` commands: a
/// non-2xx answer by its status (and its problem type, for a request the
/// server refused whole), a method error or a refused write by its type
/// and description.
pub(crate) fn refusal(cause: &(dyn std::error::Error + 'static)) -> Option<Refusal> {
    let cause = unwrapped(cause);
    if let Some(err) = cause.downcast_ref::<JmapSetRefusal>() {
        let message = err
            .description
            .clone()
            .unwrap_or_else(|| err.message.clone());
        return Some(described(None, Some(err.type_name.clone()), message));
    }

    if let Some(err) = cause.downcast_ref::<JmapMethodError>() {
        return Some(typed(None, err));
    }

    let status = if let Some(err) = cause.downcast_ref::<JmapSendError>() {
        match err {
            JmapSendError::Request { status, error } => {
                return Some(request_refusal(*status, error));
            }
            JmapSendError::HttpStatus(status) => *status,
            _ => return None,
        }
    } else if let Some(JmapSessionGetError::HttpStatus(status)) = cause.downcast_ref() {
        *status
    } else if let Some(JmapBlobDownloadError::HttpStatus(status)) = cause.downcast_ref() {
        *status
    } else if let Some(JmapBlobUploadError::HttpStatus(status)) = cause.downcast_ref() {
        *status
    } else {
        return None;
    };

    Some(described(Some(status), None, cause.to_string()))
}

/// The error io-jmap's client error wraps transparently, `cause` itself
/// otherwise: a transparent wrapper hands the chain the source of what
/// it wraps, so the chain never lists the send, session or blob error.
fn unwrapped<'a>(
    cause: &'a (dyn std::error::Error + 'static),
) -> &'a (dyn std::error::Error + 'static) {
    match cause.downcast_ref::<JmapClientStdError>() {
        Some(JmapClientStdError::Send(err)) => err,
        Some(JmapClientStdError::SessionGet(err)) => err,
        Some(JmapClientStdError::BlobUpload(err)) => err,
        Some(JmapClientStdError::BlobDownload(err)) => err,
        _ => cause,
    }
}

/// The refusal of a request the server refused whole: its problem type,
/// the URN namespace stripped, and its detail.
fn request_refusal(status: u16, err: &JmapRequestError) -> Refusal {
    let (reason, detail) = match err {
        JmapRequestError::UnknownCapability { detail } => ("unknownCapability", detail),
        JmapRequestError::NotJson { detail } => ("notJSON", detail),
        JmapRequestError::NotRequest { detail } => ("notRequest", detail),
        JmapRequestError::Limit { detail, .. } => ("limit", detail),
        JmapRequestError::Other { type_name, detail } => (type_name.as_str(), detail),
    };
    let reason = reason.trim_start_matches("urn:ietf:params:jmap:error:");
    let message = detail.clone().unwrap_or_else(|| err.to_string());

    described(Some(status), Some(reason.to_owned()), message)
}

/// The refusal a method error states: its `type`, then its
/// `description`, the type standing in for a missing one.
fn typed(status: Option<u16>, err: &impl Serialize) -> Refusal {
    let value = serde_json::to_value(err).unwrap_or_default();
    let field = |key: &str| value.get(key).and_then(|value| value.as_str());
    let reason = field("type");
    let message = field("description").or(reason).unwrap_or_default();

    described(status, reason.map(ToOwned::to_owned), message.to_owned())
}

fn described(status: Option<u16>, reason: Option<String>, message: String) -> Refusal {
    Refusal {
        status,
        reason,
        message,
        ..Default::default()
    }
}

/// Renders the suffix a failed JMAP set is reported with: its error type,
/// the properties it names, then its description.
pub fn format_set_error<E: JmapSetError>(err: &E) -> String {
    let mut msg = format!(": {}", err.type_name());

    if !err.properties().is_empty() {
        msg.push_str(" (`");
        msg.push_str(&err.properties().join("`, `"));
        msg.push_str("`)");
    }

    if let Some(desc) = err.description() {
        msg.push_str(": ");
        msg.push_str(desc.trim_end_matches(['.', '\n']));
    }

    // The type, properties and description come from the server.
    sanitize::text(&msg).into_owned()
}

/// The item error of the email `id` a set refused, coded by its type
/// (RFC 8620 §5.3, RFC 8621 §4.6), the type kept in `sources`.
pub fn set_item_error(id: &str, err: &JmapEmailSetItemError) -> ItemError {
    item_error(id, err, Some(ErrorCode::MessageNotFound))
}

/// The item error of the object `id` a set refused, coded by its type, a
/// `notFound` by `not_found`, the type kept in `sources`.
pub fn item_error<E: JmapSetError>(id: &str, err: &E, not_found: Option<ErrorCode>) -> ItemError {
    let code = match err.type_name() {
        "notFound" => not_found,
        type_name => set_error_code(type_name),
    };

    ItemError {
        id: Some(id.to_owned()),
        code,
        error: format!("JMAP server refused the write{}", format_set_error(err)),
        sources: vec![sanitize::text(err.type_name()).into_owned()],
    }
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
            | Self::Singleton { description }
            | Self::Forbidden { description }
            | Self::OverQuota { description }
            | Self::TooLarge { description }
            | Self::RateLimit { description } => description.as_deref(),
            Self::Other { description, .. } => description.as_deref(),
        }
    }

    fn properties(&self) -> &[String] {
        match self {
            Self::InvalidProperties { properties, .. } => properties,
            _ => &[],
        }
    }

    fn type_name(&self) -> &str {
        match self {
            Self::MailboxHasChild { .. } => "mailboxHasChild",
            Self::MailboxHasEmail { .. } => "mailboxHasEmail",
            Self::NotFound { .. } => "notFound",
            Self::InvalidPatch { .. } => "invalidPatch",
            Self::WillDestroy { .. } => "willDestroy",
            Self::InvalidProperties { .. } => "invalidProperties",
            Self::Singleton { .. } => "singleton",
            Self::Forbidden { .. } => "forbidden",
            Self::OverQuota { .. } => "overQuota",
            Self::TooLarge { .. } => "tooLarge",
            Self::RateLimit { .. } => "rateLimit",
            Self::Other { type_name, .. } => type_name,
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
            | Self::Singleton { description }
            | Self::Forbidden { description }
            | Self::OverQuota { description }
            | Self::TooLarge { description }
            | Self::RateLimit { description } => description.as_deref(),
            Self::Other { description, .. } => description.as_deref(),
        }
    }

    fn properties(&self) -> &[String] {
        match self {
            Self::InvalidProperties { properties, .. } => properties,
            _ => &[],
        }
    }

    fn type_name(&self) -> &str {
        match self {
            Self::TooManyKeywords { .. } => "tooManyKeywords",
            Self::TooManyMailboxes { .. } => "tooManyMailboxes",
            Self::BlobNotFound { .. } => "blobNotFound",
            Self::NotFound { .. } => "notFound",
            Self::InvalidPatch { .. } => "invalidPatch",
            Self::WillDestroy { .. } => "willDestroy",
            Self::InvalidProperties { .. } => "invalidProperties",
            Self::Singleton { .. } => "singleton",
            Self::Forbidden { .. } => "forbidden",
            Self::OverQuota { .. } => "overQuota",
            Self::TooLarge { .. } => "tooLarge",
            Self::RateLimit { .. } => "rateLimit",
            Self::Other { type_name, .. } => type_name,
        }
    }
}

impl JmapSetError for JmapEmailImportItemError {
    fn description(&self) -> Option<&str> {
        match self {
            Self::AlreadyExists { description, .. }
            | Self::InvalidEmail { description }
            | Self::NotFound { description }
            | Self::InvalidProperties { description, .. }
            | Self::Forbidden { description }
            | Self::OverQuota { description }
            | Self::TooLarge { description }
            | Self::RateLimit { description } => description.as_deref(),
            Self::Other { description, .. } => description.as_deref(),
        }
    }

    fn properties(&self) -> &[String] {
        match self {
            Self::InvalidProperties { properties, .. } => properties,
            _ => &[],
        }
    }

    fn type_name(&self) -> &str {
        match self {
            Self::AlreadyExists { .. } => "alreadyExists",
            Self::InvalidEmail { .. } => "invalidEmail",
            Self::NotFound { .. } => "notFound",
            Self::InvalidProperties { .. } => "invalidProperties",
            Self::Forbidden { .. } => "forbidden",
            Self::OverQuota { .. } => "overQuota",
            Self::TooLarge { .. } => "tooLarge",
            Self::RateLimit { .. } => "rateLimit",
            Self::Other { type_name, .. } => type_name,
        }
    }
}

impl JmapSetError for JmapEmailCopyItemError {
    fn description(&self) -> Option<&str> {
        match self {
            Self::AlreadyExists { description, .. }
            | Self::NotFound { description }
            | Self::InvalidProperties { description, .. }
            | Self::Forbidden { description }
            | Self::OverQuota { description }
            | Self::TooLarge { description }
            | Self::RateLimit { description } => description.as_deref(),
            Self::Other { description, .. } => description.as_deref(),
        }
    }

    fn properties(&self) -> &[String] {
        match self {
            Self::InvalidProperties { properties, .. } => properties,
            _ => &[],
        }
    }

    fn type_name(&self) -> &str {
        match self {
            Self::AlreadyExists { .. } => "alreadyExists",
            Self::NotFound { .. } => "notFound",
            Self::InvalidProperties { .. } => "invalidProperties",
            Self::Forbidden { .. } => "forbidden",
            Self::OverQuota { .. } => "overQuota",
            Self::TooLarge { .. } => "tooLarge",
            Self::RateLimit { .. } => "rateLimit",
            Self::Other { type_name, .. } => type_name,
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
            Self::Other { description, .. } => description.as_deref(),
        }
    }

    fn properties(&self) -> &[String] {
        match self {
            Self::InvalidProperties { properties, .. } => properties,
            _ => &[],
        }
    }

    fn type_name(&self) -> &str {
        match self {
            Self::NotFound { .. } => "notFound",
            Self::InvalidPatch { .. } => "invalidPatch",
            Self::WillDestroy { .. } => "willDestroy",
            Self::InvalidProperties { .. } => "invalidProperties",
            Self::Singleton { .. } => "singleton",
            Self::Other { type_name, .. } => type_name,
        }
    }
}

impl JmapSetError for JmapEmailSubmissionSetItemError {
    fn description(&self) -> Option<&str> {
        match self {
            Self::TooManyRecipients { description, .. }
            | Self::NoRecipients { description }
            | Self::InvalidRecipients { description, .. }
            | Self::ForbiddenMailFrom { description }
            | Self::ForbiddenFrom { description }
            | Self::ForbiddenToSend { description }
            | Self::CannotUnsendMessage { description }
            | Self::InvalidEmail { description }
            | Self::TooLarge { description, .. }
            | Self::Forbidden { description }
            | Self::OverQuota { description }
            | Self::RateLimit { description }
            | Self::NotFound { description }
            | Self::InvalidProperties { description, .. } => description.as_deref(),
            Self::Other { description, .. } => description.as_deref(),
        }
    }

    fn properties(&self) -> &[String] {
        match self {
            Self::InvalidProperties { properties, .. } => properties,
            _ => &[],
        }
    }

    fn type_name(&self) -> &str {
        match self {
            Self::TooManyRecipients { .. } => "tooManyRecipients",
            Self::NoRecipients { .. } => "noRecipients",
            Self::InvalidRecipients { .. } => "invalidRecipients",
            Self::ForbiddenMailFrom { .. } => "forbiddenMailFrom",
            Self::ForbiddenFrom { .. } => "forbiddenFrom",
            Self::ForbiddenToSend { .. } => "forbiddenToSend",
            Self::CannotUnsendMessage { .. } => "cannotUnsendMessage",
            Self::InvalidEmail { .. } => "invalidEmail",
            Self::TooLarge { .. } => "tooLarge",
            Self::Forbidden { .. } => "forbidden",
            Self::OverQuota { .. } => "overQuota",
            Self::RateLimit { .. } => "rateLimit",
            Self::NotFound { .. } => "notFound",
            Self::InvalidProperties { .. } => "invalidProperties",
            Self::Other { type_name, .. } => type_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Error;
    use io_jmap::{
        rfc8620::{error::JmapMethodError, get::JmapGetError, send::JmapSendError},
        rfc8621::{email::get::JmapEmailGetError, mailbox::set::JmapMailboxSetItemError},
    };
    use serde_json::json;

    use crate::{
        error::{self, ErrorCode},
        jmap::error::set_error,
    };

    /// A whole command the server refused is coded by the HTTP status of
    /// its answer, or by its method error, through the context a command
    /// adds.
    #[test]
    fn a_whole_refusal_is_coded() {
        let code = |err: JmapEmailGetError| {
            error::code(&Error::from(err).context("Cannot read the message"))
        };

        let throttled = JmapEmailGetError::Send(JmapSendError::HttpStatus(429));
        assert_eq!(code(throttled), Some(ErrorCode::RateLimited));

        let denied = JmapEmailGetError::Send(JmapSendError::HttpStatus(403));
        assert_eq!(code(denied), Some(ErrorCode::PermissionDenied));

        let forbidden = JmapMethodError::Forbidden { description: None };
        let forbidden = JmapEmailGetError::Get(JmapGetError::Method(forbidden));
        assert_eq!(code(forbidden), Some(ErrorCode::PermissionDenied));

        let failed = JmapEmailGetError::Send(JmapSendError::HttpStatus(500));
        assert_eq!(code(failed), None);
    }

    /// A write a whole command makes, refused, is coded by its SetError
    /// type.
    #[test]
    fn a_refused_write_is_coded_by_its_type() {
        let code = |kind: &str| {
            let err: JmapMailboxSetItemError =
                serde_json::from_value(json!({"type": kind})).unwrap();
            error::code(&set_error(
                String::from("Create JMAP mailbox `X` error"),
                &err,
            ))
        };

        assert_eq!(code("forbidden"), Some(ErrorCode::PermissionDenied));
        assert_eq!(code("overQuota"), Some(ErrorCode::QuotaExceeded));
        assert_eq!(code("rateLimit"), Some(ErrorCode::RateLimited));
        assert_eq!(code("invalidProperties"), None);
    }
}
