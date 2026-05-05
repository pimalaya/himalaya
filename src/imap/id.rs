use std::{collections::HashMap, fmt};

use anyhow::Result;
use clap::Parser;
use comfy_table::{Cell, Row, Table};
use io_imap::types::{
    core::{IString, NString},
    IntoStatic,
};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::imap::account::ImapAccount;

/// Get information about the IMAP server.
///
/// This command allows you to exchange parameters with the IMAP
/// server accordingly to the [RFC 2971]. Some providers like mail.qq
/// enforce sending ID command before selecting a mailbox.
///
/// [RFC 2971]: https://www.rfc-editor.org/rfc/rfc2971.html
#[derive(Debug, Parser)]
pub struct ImapIdCommand {
    #[arg(short, long, num_args = 1..)]
    #[arg(value_name = "KEY:VAL", value_parser = parameter_parser)]
    parameter: Option<Vec<(IString<'static>, NString<'static>)>>,
}

impl ImapIdCommand {
    pub fn execute(self, printer: &mut impl Printer, account: ImapAccount) -> Result<()> {
        let mut client = account.new_imap_client()?;
        let mut params = HashMap::new();

        params.extend([
            (
                IString::try_from("name").unwrap(),
                NString::try_from(env!("CARGO_PKG_NAME")).unwrap(),
            ),
            (
                IString::try_from("version").unwrap(),
                NString::try_from(env!("CARGO_PKG_VERSION")).unwrap(),
            ),
            (
                IString::try_from("vendor").unwrap(),
                NString::try_from("Pimalaya").unwrap(),
            ),
            (
                IString::try_from("support-url").unwrap(),
                NString::try_from("https://github.com/pimalaya/himalaya").unwrap(),
            ),
        ]);

        if let Some(more) = self.parameter {
            params.extend(more);
        }

        let params = client.id(Some(params.into_iter().collect()))?;

        let table = ServerIdTable {
            preset: account.table_preset,
            server_id: params
                .unwrap_or_default()
                .into_iter()
                .filter_map(|(key, val)| {
                    Some((
                        String::from_utf8(key.into_inner().into_owned()).ok()?,
                        match val.into_option() {
                            Some(val) => Some(String::from_utf8(val.into_owned()).ok()?),
                            None => None,
                        },
                    ))
                })
                .collect(),
        };

        printer.out(table)
    }
}

fn parameter_parser(param: &str) -> Result<(IString<'static>, NString<'static>), String> {
    let Some((key, val)) = param.split_once(':') else {
        return Err(format!("Invalid parameter `{param}`: missing `:`"));
    };

    let Ok(ikey) = IString::try_from(key.trim()) else {
        return Err(format!("Invalid parameter key `{key}`"));
    };

    let nval = if val.trim().is_empty() {
        NString::NIL
    } else {
        let Ok(nval) = NString::try_from(val.trim()) else {
            return Err(format!("Invalid parameter value `{val}` for `{key}`"));
        };

        nval
    };

    Ok((ikey.into_static(), nval.into_static()))
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ServerIdTable {
    #[serde(skip)]
    pub preset: String,
    pub server_id: HashMap<String, Option<String>>,
}

impl fmt::Display for ServerIdTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_preset(&self.preset)
            .set_header(Row::from([Cell::new("PARAMETER"), Cell::new("VALUE")]));

        for (key, val) in &self.server_id {
            table.add_row(Row::from([
                Cell::new(key),
                match val {
                    Some(val) => Cell::new(val),
                    None => Cell::new(""),
                },
            ]));
        }

        writeln!(f)?;
        writeln!(f, "{table}")
    }
}
