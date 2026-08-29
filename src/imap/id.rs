//! # IMAP id
//!
//! The `imap id` command, RFC 2971 `ID`, and the parameter resolution the
//! auth coroutines take.

use io_imap::client::ImapClient as _;
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
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    account::context::Account, config::ImapIdConfig, imap::client::ImapClient,
    shared::table::style_from_preset,
};

/// Exchange identification parameters with the server (ID, RFC 2971).
///
/// Some providers, mail.qq among them, want the exchange before a mailbox
/// can be selected at all.
#[derive(Debug, Parser)]
pub struct ImapIdCommand {
    /// Extra parameters to send, on top of himalaya's own.
    #[arg(short, long, num_args = 1..)]
    #[arg(value_name = "KEY:VAL", value_parser = parameter_parser)]
    parameter: Option<Vec<(IString<'static>, NString<'static>)>>,
}

impl ImapIdCommand {
    /// Sends the parameters and tables the ones the server answered.
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

/// The `imap id` output, a table of the parameters the server sent back.
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct ServerIdTable {
    /// The `comfy_table` preset string the table renders with.
    #[serde(skip)]
    pub preset: String,
    /// The parameters, a `NIL` value coming through as `None`.
    pub server_id: HashMap<String, Option<String>>,
}

impl fmt::Display for ServerIdTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new();

        table
            .load_style(style_from_preset(&self.preset))
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

/// Resolves the configured `imap.id.fields` into the parameter list the
/// io-imap auth coroutines send.
///
/// `None` when `auto` is off. A key set to `true` takes himalaya's canned
/// value, or `NIL` with a warning when there is none.
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

/// Parses a `KEY:VAL` parameter, an empty value meaning `NIL`.
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

/// himalaya's own value for a well-known `ID` key.
fn canned_value(key: &str) -> Option<&'static str> {
    match key {
        "name" => Some(env!("CARGO_PKG_NAME")),
        "version" => Some(env!("CARGO_PKG_VERSION")),
        "vendor" => Some("Pimalaya"),
        "support-url" => Some("https://github.com/pimalaya/himalaya"),
        _ => None,
    }
}

/// Builds the wire pair of a well-known key and its canned value.
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
