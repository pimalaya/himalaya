//! # ManageSieve outcome
//!
//! A typed `sieve` command the server refuses prints its `NO` or `BYE`
//! completion, response code and text, as the result and exits 3, the
//! server having answered. No completion at all (network, TLS, unparsable
//! response) still fails with the failure object.

use std::fmt;

use anyhow::{Error, Result};
use io_managesieve::{
    client::ManagesieveClientError,
    rfc5804::{
        capability::ManagesieveCapabilityGetError, checkscript::ManagesieveScriptCheckError,
        deletescript::ManagesieveScriptDeleteError, getscript::ManagesieveScriptGetError,
        havespace::ManagesieveHaveSpaceError, listscripts::ManagesieveScriptListError,
        putscript::ManagesieveScriptPutError, renamescript::ManagesieveScriptRenameError,
        response::ManagesieveCompletion, setactive::ManagesieveScriptActivateError,
    },
};
use pimalaya_cli::{error::PartialOutcome, printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

/// The result of a `sieve` command the server refused: its completion,
/// as `sieve raw` prints one.
#[derive(Debug, Eq, PartialEq, Serialize, JsonSchema)]
pub struct SieveRefusalOutput {
    /// `NO` or `BYE`.
    pub status: &'static str,
    /// The response code, such as `NONEXISTENT` or `QUOTA/MAXSIZE`,
    /// `null` when the server sent none.
    pub code: Option<String>,
    /// The text of the completion, `null` when the server sent none.
    pub text: Option<String>,
}

impl From<&ManagesieveCompletion> for SieveRefusalOutput {
    fn from(completion: &ManagesieveCompletion) -> Self {
        Self {
            status: completion.status.as_str(),
            code: completion.code.as_ref().map(ToString::to_string),
            text: completion.text.clone(),
        }
    }
}

impl fmt::Display for SieveRefusalOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ManageSieve server answered {}", self.status)?;

        if let Some(code) = &self.code {
            write!(f, " ({})", sanitize::text(code))?;
        }

        match &self.text {
            Some(text) => writeln!(f, ": {}", sanitize::text(text)),
            None => writeln!(f),
        }
    }
}

/// The schema of a `sieve` command: its own result, or the refusal, told
/// apart by `status`.
#[derive(JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum SieveOutcome<T> {
    /// The server accepted the command.
    Done(T),
    /// The server refused it.
    Refused(SieveRefusalOutput),
}

/// Ends a `sieve` command: a server refusal prints as the result with the
/// partial-outcome status, any other error is returned unchanged.
pub fn print(printer: &mut impl Printer, result: Result<()>) -> Result<()> {
    let err = match result {
        Ok(()) => return Ok(()),
        Err(err) => err,
    };

    let Some(refusal) = refusal_of(&err) else {
        return Err(err);
    };

    printer.out(refusal)?;
    Err(PartialOutcome::default().into())
}

/// The completion a typed ManageSieve call was refused with, if that is
/// how it failed.
fn refusal_of(err: &Error) -> Option<SieveRefusalOutput> {
    let err = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<ManagesieveClientError>())?;

    macro_rules! refusal {
        ($($variant:ident($ty:ident)),* $(,)?) => {
            match err {
                $(ManagesieveClientError::$variant($ty::Rejected(completion)) => {
                    Some(SieveRefusalOutput::from(completion))
                })*
                _ => None,
            }
        };
    }

    refusal!(
        Capability(ManagesieveCapabilityGetError),
        HaveSpace(ManagesieveHaveSpaceError),
        ScriptList(ManagesieveScriptListError),
        ScriptGet(ManagesieveScriptGetError),
        ScriptPut(ManagesieveScriptPutError),
        ScriptCheck(ManagesieveScriptCheckError),
        ScriptActivate(ManagesieveScriptActivateError),
        ScriptDelete(ManagesieveScriptDeleteError),
        ScriptRename(ManagesieveScriptRenameError),
    )
}

#[cfg(test)]
mod tests {
    use anyhow::{Error, anyhow};
    use io_managesieve::{
        client::ManagesieveClientError,
        rfc5804::{
            deletescript::ManagesieveScriptDeleteError,
            response::{ManagesieveCompletion, ManagesieveStatus},
        },
    };
    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::json;

    use crate::sieve::outcome::print;

    /// A `NO` the server answered prints as the result and exits 3; an
    /// error with no completion stays the failure.
    #[test]
    fn a_refused_command_prints_its_completion() {
        let completion = ManagesieveCompletion {
            status: ManagesieveStatus::No,
            code: None,
            text: Some("Script is active".into()),
        };
        let err = ManagesieveScriptDeleteError::Rejected(completion);
        let err = Error::from(ManagesieveClientError::ScriptDelete(err));
        let mut printer = CapturePrinter::default();

        let err = print(&mut printer, Err(err.context("Cannot delete"))).unwrap_err();
        assert!(pimalaya_cli::error::partial_outcome_of(&err).is_some());
        let refusal = json!({"status": "NO", "code": null, "text": "Script is active"});
        assert_eq!(printer.into_single().unwrap(), refusal);

        let mut printer = CapturePrinter::default();
        let err = print(&mut printer, Err(anyhow!("boom"))).unwrap_err();
        assert_eq!(err.to_string(), "boom");
    }
}
