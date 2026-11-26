//! # ManageSieve deactivate
//!
//! The `sieve deactivate` command, an RFC 5804 `SETACTIVE` naming no
//! script.

use std::fmt;

use anyhow::Result;
use clap::Parser;
use io_managesieve::client::ManagesieveClient as _;
use pimalaya_cli::{printer::Printer, sanitize};
use schemars::JsonSchema;
use serde::Serialize;

use crate::sieve::client::SieveClient;

/// Disable the currently active server-side Sieve script.
///
/// Filtering stops until a script is activated again; doing it twice is
/// not an error. Prints the script that was active, `null` when none was.
#[derive(Debug, Parser)]
pub struct SieveScriptDeactivateCommand;

impl SieveScriptDeactivateCommand {
    /// Reads which script is active, then leaves the account with none.
    pub fn execute(self, printer: &mut impl Printer, client: &mut SieveClient) -> Result<()> {
        let id = client
            .list_scripts()?
            .into_iter()
            .find(|script| script.active)
            .map(|script| script.name);

        client.activate_script(None)?;
        printer.out(SieveDeactivateOutput { id })
    }
}

/// The `sieve deactivate` output.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SieveDeactivateOutput {
    /// The name of the script active before the command, `null` when none
    /// was.
    id: Option<String>,
}

impl fmt::Display for SieveDeactivateOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.id {
            Some(id) => {
                let id = sanitize::text(id);
                writeln!(f, "Sieve script `{id}` successfully deactivated")
            }
            None => writeln!(f, "No Sieve script was active"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, BufReader, Write},
        net::TcpListener,
        thread,
    };

    use pimalaya_cli::printer::CapturePrinter;
    use serde_json::{Value, json};

    use crate::{config::sieve::SieveConfig, sieve::client::SieveClient};

    use super::SieveScriptDeactivateCommand;

    /// Runs the command against a server listing `scripts`, returning
    /// what it printed and the commands the server read.
    fn deactivate(scripts: &'static str) -> (Value, Vec<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut out = stream;
            let mut commands = Vec::new();
            let greeting = "\"IMPLEMENTATION\" \"fake\"\r\n\"SIEVE\" \"fileinto\"\r\n\
                            \"VERSION\" \"1.0\"\r\nOK\r\n";
            out.write_all(greeting.as_bytes()).unwrap();

            let mut line = String::new();
            while reader.read_line(&mut line).unwrap() > 0 {
                let command = line.trim_end().to_owned();
                line.clear();
                let reply = match command.as_str() {
                    "LISTSCRIPTS" => format!("{scripts}OK\r\n"),
                    "CAPABILITY" => greeting.to_owned(),
                    _ => String::from("OK\r\n"),
                };
                out.write_all(reply.as_bytes()).unwrap();
                commands.push(command);
            }
            commands
        });

        let toml = format!("server = \"sieve://{addr}\"\nstarttls = false");
        let config: SieveConfig = toml::from_str(&toml).unwrap();
        let mut secrets = pimalaya_config::secret::SecretResolver::new();
        let mut client = SieveClient::new(config, &mut secrets).unwrap();
        let mut printer = CapturePrinter::default();
        SieveScriptDeactivateCommand
            .execute(&mut printer, &mut client)
            .unwrap();
        drop(client);

        (printer.into_single().unwrap(), server.join().unwrap())
    }

    /// The script active before is printed, then deactivated; with none
    /// active the id is `null`.
    #[test]
    fn the_script_active_before_is_printed() {
        let (output, commands) = deactivate("\"spare\"\r\n\"main\" ACTIVE\r\n");
        assert_eq!(output, json!({"id": "main"}));
        let setactive = commands.iter().position(|c| c == "SETACTIVE \"\"");
        let listscripts = commands.iter().position(|c| c == "LISTSCRIPTS");
        assert!(
            listscripts < setactive && setactive.is_some(),
            "{commands:?}"
        );

        let (output, _) = deactivate("\"spare\"\r\n");
        assert_eq!(output, json!({"id": null}));
    }
}
