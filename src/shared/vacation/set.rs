//! # Vacation set
//!
//! The `vacation set` command, changing the account's auto-reply.

use std::fmt;

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::{ArgGroup, Parser};
use pimalaya_cli::printer::Printer;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account,
    email::vacation::{Vacation, VacationPatch, parse_date},
    shared::vacation::{cli::VacationClient, get::vacation_fields},
};

/// Change the auto-reply, then print it as it stands.
///
/// Only what is given changes, the rest kept as it is; a `--no-` flag
/// clears its field. The auto-reply
/// itself is turned on with `--enable` and off with `--disable`, never by
/// accident. Once on, it answers incoming mail, so an agent reaching this
/// command can make the account send mail.
///
/// Nothing given is a usage error, there being nothing to change.
#[derive(Debug, Parser)]
#[command(group(
    ArgGroup::new("change")
        .required(true)
        .multiple(true)
        .args([
            "enable",
            "disable",
            "from",
            "to",
            "subject",
            "text",
            "html",
            "no_from",
            "no_to",
            "no_subject",
            "no_text",
            "no_html",
        ]),
))]
pub struct VacationSetCommand {
    /// Turn the auto-reply on.
    #[arg(long, conflicts_with = "disable")]
    pub enable: bool,
    /// Turn the auto-reply off.
    #[arg(long)]
    pub disable: bool,
    /// When it starts answering: RFC 3339, or YYYY-MM-DD for midnight
    /// UTC.
    #[arg(long, value_name = "DATE", value_parser = parse_date)]
    pub from: Option<DateTime<Utc>>,
    /// When it stops answering: RFC 3339, or YYYY-MM-DD for midnight UTC.
    #[arg(long, value_name = "DATE", value_parser = parse_date)]
    pub to: Option<DateTime<Utc>>,
    /// The subject of the reply.
    #[arg(long, value_name = "TEXT")]
    pub subject: Option<String>,
    /// The plain text body of the reply.
    #[arg(long, value_name = "TEXT")]
    pub text: Option<String>,
    /// The HTML body of the reply.
    #[arg(long, value_name = "HTML")]
    pub html: Option<String>,
    /// Clear the start date, answering from now on.
    #[arg(long, conflicts_with = "from")]
    pub no_from: bool,
    /// Clear the end date, answering until turned off.
    #[arg(long, conflicts_with = "to")]
    pub no_to: bool,
    /// Clear the subject, the backend's default answering.
    #[arg(long, conflicts_with = "subject")]
    pub no_subject: bool,
    /// Clear the plain text body.
    #[arg(long, conflicts_with = "text")]
    pub no_text: bool,
    /// Clear the HTML body.
    #[arg(long, conflicts_with = "html")]
    pub no_html: bool,
}

impl VacationSetCommand {
    /// Applies the changes and prints the auto-reply as it then stands.
    pub fn execute(
        self,
        printer: &mut impl Printer,
        _account: &mut Account,
        client: VacationClient<'_>,
    ) -> Result<()> {
        printer.out(VacationSetOutput {
            vacation: client.set(self.patch())?,
        })
    }

    /// The change the flags ask for: a setter sets its field, its `--no-`
    /// clears it, neither leaves it.
    fn patch(self) -> VacationPatch {
        fn field<T>(value: Option<T>, clear: bool) -> Option<Option<T>> {
            value.map(Some).or(clear.then_some(None))
        }

        let enabled = match (self.enable, self.disable) {
            (true, _) => Some(true),
            (_, true) => Some(false),
            _ => None,
        };

        VacationPatch {
            enabled,
            from: field(self.from, self.no_from),
            to: field(self.to, self.no_to),
            subject: field(self.subject, self.no_subject),
            text: field(self.text, self.no_text),
            html: field(self.html, self.no_html),
        }
    }
}

/// The `vacation set` output, the auto-reply as it stands once changed.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct VacationSetOutput {
    /// The auto-reply.
    pub vacation: Vacation,
}

impl fmt::Display for VacationSetOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", vacation_fields(&self.vacation))
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::shared::vacation::set::VacationSetCommand;

    fn patch(args: &[&str]) -> crate::email::vacation::VacationPatch {
        let args = ["set"].iter().chain(args);
        VacationSetCommand::try_parse_from(args).unwrap().patch()
    }

    /// A setter sets its field, its `--no-` clears it, and a field named
    /// by neither stays as it is.
    #[test]
    fn a_no_flag_clears_its_field() {
        let patch = patch(&["--no-subject", "--text", "away", "--no-to"]);

        assert_eq!(patch.subject, Some(None));
        assert_eq!(patch.text, Some(Some(String::from("away"))));
        assert_eq!(patch.to, Some(None));
        assert_eq!(patch.from, None);
        assert_eq!(patch.html, None);
        assert_eq!(patch.enabled, None);
    }

    /// A field cannot be set and cleared at once, and a `--no-` flag alone
    /// is a change.
    #[test]
    fn a_setter_and_its_no_flag_conflict() {
        let both = ["set", "--subject", "x", "--no-subject"];
        assert!(VacationSetCommand::try_parse_from(both).is_err());
        assert!(VacationSetCommand::try_parse_from(["set", "--no-html"]).is_ok());
    }
}
