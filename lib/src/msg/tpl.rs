//! Module related to message template CLI.
//!
//! This module provides subcommands, arguments and a command matcher related to message template.

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct TplOverride<'a> {
    pub subject: Option<&'a str>,
    pub from: Option<Vec<&'a str>>,
    pub to: Option<Vec<&'a str>>,
    pub cc: Option<Vec<&'a str>>,
    pub bcc: Option<Vec<&'a str>>,
    pub headers: Option<Vec<&'a str>>,
    pub body: Option<&'a str>,
    pub sig: Option<&'a str>,
}
