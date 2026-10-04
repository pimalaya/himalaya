//! # Error codes
//!
//! A stable, machine-readable name for the failures a caller is expected
//! to act on, printed as `code` beside the message under `--json`.
//!
//! The message stays for humans and may be reworded at any time; the
//! code is the contract. A failure with no code prints as before, with
//! no `code` field.

use std::fmt;

use anyhow::Error;
use pimalaya_cli::{error::ErrorReport, printer::Printer};
use schemars::JsonSchema;
use serde::Serialize;

/// The stable codes, each serialised in kebab-case.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    /// The message is listed but its body is not local yet: a sync will
    /// bring it, so the caller waits rather than reports a loss.
    BodyPending,
}

/// A failure carrying an [`ErrorCode`], anywhere in an error chain.
#[derive(Debug)]
pub struct CodedError {
    code: ErrorCode,
    message: String,
}

impl CodedError {
    /// Builds a failure with its code and its human message.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for CodedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CodedError {}

/// The code of the outermost [`CodedError`] in the chain, if any.
pub fn code_of(err: &Error) -> Option<ErrorCode> {
    err.chain()
        .find_map(|cause| cause.downcast_ref::<CodedError>())
        .map(|coded| coded.code)
}

/// The printed failure: pimalaya-cli's report, plus the code when the
/// chain carries one.
#[derive(Serialize)]
struct CodedReport {
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<ErrorCode>,
    #[serde(flatten)]
    report: ErrorReport,
}

impl fmt::Display for CodedReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.report.fmt(f)
    }
}

/// Unwraps a result, or prints the error to the printer and exits with
/// status 1, as [`ErrorReport::eval`] does, with the code added.
pub fn eval<T>(printer: &mut impl Printer, result: Result<T, Error>) -> T {
    match result {
        Ok(res) => res,
        Err(err) => {
            let code = code_of(&err);
            let _ = printer.out(CodedReport {
                code,
                report: ErrorReport::from(err),
            });
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Context;

    use super::*;

    fn pending() -> Result<(), Error> {
        Err(CodedError::new(ErrorCode::BodyPending, "not downloaded yet").into())
    }

    #[test]
    fn a_coded_error_keeps_its_code_under_context() {
        let err = pending().context("Read message 3").unwrap_err();
        assert_eq!(code_of(&err), Some(ErrorCode::BodyPending));
    }

    #[test]
    fn the_json_report_carries_the_code_beside_the_message() {
        let err = pending().unwrap_err();
        let json = serde_json::to_value(CodedReport {
            code: code_of(&err),
            report: ErrorReport::from(err),
        })
        .unwrap();
        assert_eq!(json["code"], "body-pending");
        assert_eq!(json["error"], "not downloaded yet");
    }

    #[test]
    fn an_uncoded_error_prints_no_code() {
        let err = anyhow::anyhow!("boom");
        assert_eq!(code_of(&err), None);
        let json = serde_json::to_value(CodedReport {
            code: None,
            report: ErrorReport::from(err),
        })
        .unwrap();
        assert!(json.get("code").is_none());
    }
}
