use std::fmt;

use anyhow::{Result, bail};
use clap::Parser;
use mail_parser::{Message, MessageParser};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::m2dir::{
    arg::{M2dirNameFlag, MessageIdArg},
    client::M2dirClient,
};

/// Get headers of an m2dir message.
///
/// Resolves the message identified by `ID` inside the given m2dir
/// folder, parses its headers and prints them.
#[derive(Debug, Parser)]
pub struct M2dirMessageGetCommand {
    #[command(flatten)]
    pub m2dir: M2dirNameFlag,
    #[command(flatten)]
    pub id: MessageIdArg,
}

impl M2dirMessageGetCommand {
    pub fn execute(self, printer: &mut impl Printer, client: &mut M2dirClient) -> Result<()> {
        let store = client.open_store()?;
        let path = store.resolve_folder_path(&self.m2dir.inner)?;
        let m2dir = client.open_m2dir(path)?;
        let (entry, bytes) = client.get(m2dir, &self.id.inner)?;

        let Some(parsed) = MessageParser::new().parse_headers(&bytes) else {
            let path = entry.path();
            bail!("Invalid MIME message at {path}");
        };

        printer.out(MessageView(parsed))
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct MessageView<'a>(Message<'a>);

impl fmt::Display for MessageView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for header in self.0.headers() {
            writeln!(f, "{}: {:?}", header.name(), header.value())?;
        }

        Ok(())
    }
}
