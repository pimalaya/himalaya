//! # ManageSieve activate
//!
//! The `sieve activate` command, RFC 5804 `SETACTIVE`.

use anyhow::Result;
use clap::Parser;
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::printer::Printer;

use crate::{receipt::IdReceipt, sieve::client::SieveClient};

/// Make one server-side Sieve script active.
#[derive(Debug, Parser)]
pub struct SieveScriptActivateCommand {
    /// The script name, never empty: RFC 5804 reads an empty one as a
    /// deactivation, which `sieve deactivate` sends.
    #[arg(value_name = "NAME", value_parser = clap::builder::NonEmptyStringValueParser::new())]
    pub name: String,
}

impl SieveScriptActivateCommand {
    /// Makes the named script the active one.
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        client.activate_script(Some(self.name.clone()))?;

        printer.out(IdReceipt {
            id: self.name,
            done: "Sieve script successfully activated",
        })
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::SieveScriptActivateCommand;

    /// RFC 5804 reads `SETACTIVE ""` as a deactivation, so an empty name
    /// (an unset shell variable) is refused before anything is sent.
    #[test]
    fn an_empty_name_is_refused() {
        let parse = |name| SieveScriptActivateCommand::try_parse_from(["activate", name]);

        let err = parse("").unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
        assert_eq!(parse("main").unwrap().name, "main");
    }
}
