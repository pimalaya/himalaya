//! # HTTP refusals
//!
//! What a `gmail`, `msgraph` or `jmap` command prints when the server
//! refuses it: an HTTP 4xx or 5xx carrying the provider's error, or a JMAP
//! method or set error. The refusal is the command's result, under
//! `refusal`, and the command exits 3, the server having answered. A
//! command that gets no answer at all (network, TLS, a body that does not
//! parse, a local error) still fails with the failure object.

use std::fmt;

use anyhow::{Error, Result};
use pimalaya_cli::{error::PartialOutcome, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::error::{self, ErrorCode};

/// The result of a protocol command the server refused.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RefusalOutput {
    /// What the server answered.
    pub refusal: Refusal,
}

/// A refusal, as the provider's error states it.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Refusal {
    /// The HTTP status, `null` for a refusal answered inside a 2xx (a
    /// JMAP method or set error).
    pub status: Option<u16>,
    /// The provider's name for the refusal, in its spelling: Gmail's
    /// first reason (`notFound`), Graph's `error.code`
    /// (`ErrorItemNotFound`), the JMAP error type (`invalidArguments`),
    /// `null` when the provider names none.
    pub reason: Option<String>,
    /// The provider's message.
    pub message: String,
    /// Himalaya's code of the refusal, as a failure would carry it
    /// (`permission-denied`, `quota-exceeded`, `rate-limited`), `null`
    /// when none applies.
    pub code: Option<ErrorCode>,
    /// The seconds the server asked to wait before a retry, `null` when
    /// it asked none.
    pub retry_after: Option<u64>,
}

impl fmt::Display for RefusalOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let refusal = &self.refusal;

        write!(f, "Server refused the request")?;

        if let Some(status) = refusal.status {
            write!(f, " (HTTP {status})")?;
        }

        if let Some(reason) = &refusal.reason {
            write!(f, ": {}", sanitize::text(reason))?;
        }

        writeln!(f, ": {}", sanitize::text(&refusal.message))
    }
}

/// The schema of a protocol command whose server may refuse it: its own
/// result, or the refusal, told apart by `refusal`.
#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum Refusable<T> {
    /// The server accepted the command.
    Done(T),
    /// The server refused it.
    Refused(RefusalOutput),
}

/// A reader of one protocol's typed errors, turning the refusal it knows
/// into a [`Refusal`].
pub type Reader = fn(&(dyn std::error::Error + 'static)) -> Option<Refusal>;

/// Ends a protocol command: its error unchanged, unless a reader finds a
/// refusal in its chain, which then prints as the result and ends the
/// command with the partial-outcome status.
pub fn print(printer: &mut impl Printer, readers: &[Reader], result: Result<()>) -> Result<()> {
    let err = match result {
        Ok(()) => return Ok(()),
        Err(err) => err,
    };

    let Some(refusal) = refusal_of(readers, &err) else {
        return Err(err);
    };

    printer.out(RefusalOutput { refusal })?;
    Err(PartialOutcome::default().into())
}

/// The refusal of `err`'s chain, the first cause a reader knows, its
/// code the one the failure would carry.
pub fn refusal_of(readers: &[Reader], err: &Error) -> Option<Refusal> {
    if pimalaya_cli::error::partial_outcome_of(err).is_some() {
        return None;
    }

    let refusal = err
        .chain()
        .find_map(|cause| readers.iter().find_map(|reader| reader(cause)))?;

    Some(Refusal {
        code: error::code(err),
        ..refusal
    })
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use crate::shared::refusal::{Reader, Refusal, print};

    #[derive(Debug)]
    struct Refused;

    impl std::fmt::Display for Refused {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("refused")
        }
    }

    impl std::error::Error for Refused {}

    fn reader(cause: &(dyn std::error::Error + 'static)) -> Option<Refusal> {
        cause.downcast_ref::<Refused>().map(|_| Refusal {
            status: Some(404),
            reason: Some("notFound".into()),
            message: "Not found".into(),
            ..Default::default()
        })
    }

    /// A refusal under context prints as the result and exits 3; any other
    /// error stays the failure.
    #[test]
    fn a_refusal_is_the_result() {
        let readers: &[Reader] = &[reader];
        let mut printer = CapturePrinter::default();

        let err = anyhow::Error::from(Refused).context("Cannot get label");
        let err = print(&mut printer, readers, Err(err)).unwrap_err();
        assert!(pimalaya_cli::error::partial_outcome_of(&err).is_some());

        let refusal = json!({
            "refusal": {
                "status": 404,
                "reason": "notFound",
                "message": "Not found",
                "code": null,
                "retryAfter": null,
            }
        });
        assert_eq!(printer.into_single().unwrap(), refusal);

        let mut printer = CapturePrinter::default();
        let err = print(&mut printer, readers, Err(anyhow!("boom"))).unwrap_err();
        assert_eq!(err.to_string(), "boom");
        assert!(printer.into_single().is_err());
    }
}
