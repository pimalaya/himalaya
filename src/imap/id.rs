use std::{collections::HashMap, fmt};

use anyhow::{Result, anyhow};
use clap::Parser;
use comfy_table::{Cell, Row, Table};
use io_imap::{
    rfc2971::id::ImapServerIdOptions,
    types::{
        IntoStatic,
        core::{IString, NString},
    },
};
use pimalaya_cli::printer::Printer;
use serde::Serialize;

use crate::account::context::Account;
use crate::{config::ImapIdConfig, imap::client::ImapClient};

/// Get information about the IMAP server.
///
/// This command allows you to exchange parameters with the IMAP server
/// accordingly to the [RFC 2971]. Some providers like mail.qq enforce sending
/// ID command before selecting a mailbox.
///
/// [RFC 2971]: https://www.rfc-editor.org/rfc/rfc2971.html
#[derive(Debug, Parser)]
pub struct ImapIdCommand {
    #[arg(short, long, num_args = 1..)]
    #[arg(value_name = "KEY:VAL", value_parser = parameter_parser)]
    parameter: Option<Vec<(IString<'static>, NString<'static>)>>,
}

impl ImapIdCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        account: &mut Account,
        client: &mut ImapClient,
    ) -> Result<()> {
        let mut params: HashMap<IString<'static>, NString<'static>> = HashMap::new();
        for key in ["name", "version", "vendor", "support-url"] {
            let (k, v) = build_canned_pair(key)?;
            params.insert(k, v);
        }

        if let Some(more) = self.parameter {
            params.extend(more);
        }

        let params = client.id(ImapServerIdOptions {
            parameters: Some(params.into_iter().collect()),
        })?;

        let table = ServerIdTable {
            preset: account.table_preset().to_string(),
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

/// Renderable table of the IMAP server ID parameters.
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

/// Resolves an [`ImapIdConfig`] into the wire-level parameter list
/// passed to the io-imap auth coroutines.
///
/// [`None`] when `auto = false`; otherwise a vec where each entry
/// maps the user-supplied key to either himalaya's canned value
/// (when the user set `true` and the key is well-known) or `NIL`.
/// Unknown keys with `true` log a warning and fall back to `NIL`.
pub fn resolve_auto_id_params(
    config: &ImapIdConfig,
) -> Result<Option<Vec<(IString<'static>, NString<'static>)>>> {
    if !config.auto {
        return Ok(None);
    }

    let mut params = Vec::with_capacity(config.fields.len());
    for (key, &use_canned) in &config.fields {
        let ikey = IString::try_from(key.clone())
            .map_err(|err| anyhow!("Invalid IMAP ID parameter key `{key}`: {err}"))?
            .into_static();

        let nval = if use_canned {
            match canned_value(key) {
                Some(value) => NString::try_from(value)
                    .map_err(|err| {
                        anyhow!("Invalid canned IMAP ID value `{value}` for `{key}`: {err}")
                    })?
                    .into_static(),
                None => {
                    log::warn!("imap.id.fields.{key} = true: no canned value defined, sending NIL");
                    NString::NIL
                }
            }
        } else {
            NString::NIL
        };

        params.push((ikey, nval));
    }
    Ok(Some(params))
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

fn canned_value(key: &str) -> Option<&'static str> {
    match key {
        "name" => Some(env!("CARGO_PKG_NAME")),
        "version" => Some(env!("CARGO_PKG_VERSION")),
        "vendor" => Some("Pimalaya"),
        "support-url" => Some("https://github.com/pimalaya/himalaya"),
        _ => None,
    }
}

fn build_canned_pair(key: &str) -> Result<(IString<'static>, NString<'static>)> {
    let ikey = IString::try_from(key)
        .map_err(|err| anyhow!("Invalid IMAP ID parameter key `{key}`: {err}"))?
        .into_static();
    let value =
        canned_value(key).ok_or_else(|| anyhow!("No canned IMAP ID value defined for `{key}`"))?;
    let nval = NString::try_from(value)
        .map_err(|err| anyhow!("Invalid canned IMAP ID value `{value}` for `{key}`: {err}"))?
        .into_static();
    Ok((ikey, nval))
}
