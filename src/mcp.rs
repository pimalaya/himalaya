//! # MCP command
//!
//! The `mcp` subcommand, a Model Context Protocol server over stdin and
//! stdout exposing the shared API as tools, one per leaf command.
//!
//! A tool's input schema is generated from its clap command, and a call is
//! turned back into an argv parsed by that same command, so the CLI, the
//! [`crate::repl`] and MCP share one grammar. Commands run over the
//! [`Session`] the server keeps, and print into an [`McpPrinter`] whose one
//! JSON value becomes the call's structured result.
//!
//! Refs: <https://modelcontextprotocol.io/specification/2025-06-18>

use std::{
    fmt,
    io::{self, BufRead, Write},
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result, anyhow};
use clap::{Arg, ArgAction, CommandFactory, Parser, ValueHint};
use pimalaya_cli::{error::ErrorReport, printer::Printer};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::{
    capability::{self, Capability, Resolution, Support},
    cli::{Cli, Command, resolve_account},
    error::ErrorCode,
    json_schema, repl,
    session::Session,
};

/// The protocol revision the server speaks.
const PROTOCOL_VERSION: &str = "2025-06-18";

/// The shared API leaves exposed as tools, with what each may do.
///
/// A leaf missing here is not exposed, so a new command is listed only
/// once it is classified; `message parse` (reads a local file the caller
/// names) and `attachment download` (writes files on disk) are left out.
const TOOLS: &[McpTool] = &[
    McpTool::read("mailbox", "list"),
    McpTool::read("envelope", "list"),
    McpTool::read("envelope", "search"),
    McpTool::write("flag", "add", false),
    McpTool::write("flag", "set", true),
    McpTool::write("flag", "remove", true),
    McpTool::write("message", "add", false),
    McpTool::write("message", "copy", false).untyped(),
    McpTool::write("message", "move", true).untyped(),
    McpTool::write("message", "delete", true),
    // NOTE: `--raw` prints the message bytes instead of the read view its
    // output schema describes.
    McpTool::read("message", "read").without(&["raw"]),
    McpTool::send("message", "send"),
    // NOTE: the composers print the composed message, or a confirmation
    // once `save` or `send` is given, so they declare no output schema.
    McpTool::read("message", "compose").untyped(),
    McpTool::read("message", "reply").untyped(),
    McpTool::read("message", "forward").untyped(),
    McpTool::read("attachment", "list"),
];

/// The arguments turning a tool into one that changes the store, listed
/// only under `--allow-write`: saving a composed message, flagging a read
/// one as seen.
const WRITE_ARGS: &[&str] = &["save", "seen"];

/// The arguments turning a tool into one that sends mail, listed only
/// under `--allow-send`.
const SEND_ARGS: &[&str] = &["send"];

/// Whether local file paths given as command input are read as such.
static PATHS_CLOSED: AtomicBool = AtomicBool::new(false);

/// Start an MCP server over stdio for one account.
///
/// Each tool is a shared API command (`envelope_list`, `message_read`)
/// run over clients connected once for the server's lifetime; the
/// account and `--backend` are the server's. Without flags only the
/// tools that read are listed.
#[derive(Debug, Parser)]
pub struct McpCommand {
    /// List the tools that change the store too.
    ///
    /// Adds the flag, add, copy, move and delete tools, and the `save`
    /// argument of the composers and of `message_send`.
    #[arg(long)]
    pub allow_write: bool,
    /// List the tools that send mail too.
    ///
    /// Adds `message_send` and the `send` argument of the composers and
    /// of `message_add`. An agent reading mail can be told by a message
    /// to send mail, so sending is never on by default.
    #[arg(long)]
    pub allow_send: bool,
}

impl McpCommand {
    /// Serves JSON-RPC requests off stdin until EOF, answering each on
    /// stdout.
    pub fn execute(self, printer: &mut impl Printer, session: &mut Session) -> Result<()> {
        let account = session.account;

        // NOTE: resolved once up front, so a missing configuration or
        // account fails the server rather than its first call, and the
        // tools listed are the ones the account's backends serve.
        let (_, name, account_config) =
            resolve_account(printer, account.config_paths, account.name)?;
        let resolution = Resolution::of(&name, &account_config, account.backend);

        repl::claim_stdin();
        PATHS_CLOSED.store(true, Ordering::Relaxed);

        let mut server = McpServer {
            session,
            tools: McpToolSpec::all(self.allow_write, self.allow_send, Some(&resolution)),
        };

        let mut stdout = io::stdout();

        for line in io::stdin().lock().lines() {
            let line = line.context("Cannot read a request from stdin")?;

            if line.trim().is_empty() {
                continue;
            }

            if let Some(response) = server.answer(&line) {
                serde_json::to_writer(&mut stdout, &response)?;
                writeln!(stdout)?;
                stdout.flush()?;
            }
        }

        Ok(())
    }
}

/// Whether a local file path given as command input must be taken as the
/// input itself, as it is inside an MCP server.
pub fn paths_closed() -> bool {
    PATHS_CLOSED.load(Ordering::Relaxed)
}

/// A shared API leaf as a tool, and what it may do.
struct McpTool {
    group: &'static str,
    leaf: &'static str,
    kind: McpToolKind,
    destructive: bool,
    typed: bool,
    without: &'static [&'static str],
}

/// What a tool may do, deciding which flag lists it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum McpToolKind {
    Read,
    Write,
    Send,
}

impl McpTool {
    const fn read(group: &'static str, leaf: &'static str) -> Self {
        Self::new(group, leaf, McpToolKind::Read, false)
    }

    const fn write(group: &'static str, leaf: &'static str, destructive: bool) -> Self {
        Self::new(group, leaf, McpToolKind::Write, destructive)
    }

    const fn send(group: &'static str, leaf: &'static str) -> Self {
        Self::new(group, leaf, McpToolKind::Send, false)
    }

    const fn new(
        group: &'static str,
        leaf: &'static str,
        kind: McpToolKind,
        destructive: bool,
    ) -> Self {
        Self {
            group,
            leaf,
            kind,
            destructive,
            typed: true,
            without: &[],
        }
    }

    const fn untyped(self) -> Self {
        Self {
            typed: false,
            ..self
        }
    }

    const fn without(self, without: &'static [&'static str]) -> Self {
        Self { without, ..self }
    }
}

/// A listed tool: its definition and the arguments a call maps onto.
struct McpToolSpec {
    group: &'static str,
    leaf: &'static str,
    args: Vec<McpToolArg>,
    definition: Value,
}

/// One argument of a tool, and how it goes back into an argv.
struct McpToolArg {
    name: String,
    long: Option<String>,
    index: Option<usize>,
    boolean: bool,
    multiple: bool,
}

impl McpToolSpec {
    /// The tools the gates allow, from the commands this build carries.
    ///
    /// With a resolution, a tool the account's backends cannot serve at
    /// all is left out, as the static capability tier knows without
    /// connecting; one only the server decides stays listed and fails
    /// with `capability-missing` when called.
    fn all(allow_write: bool, allow_send: bool, resolution: Option<&Resolution>) -> Vec<Self> {
        let mut cli = Cli::command();
        cli.build();

        let schemas = json_schema::registry().schemas;
        let mut tools = Vec::new();

        for tool in TOOLS {
            let listed = match tool.kind {
                McpToolKind::Read => true,
                McpToolKind::Write => allow_write,
                McpToolKind::Send => allow_send,
            };

            if !listed {
                continue;
            }

            let served = capability::leaf(tool.group, tool.leaf)
                .zip(resolution)
                .is_none_or(|(leaf, resolution)| leaf.is_served(resolution));

            if !served {
                continue;
            }

            let Some(cmd) = cli
                .find_subcommand(tool.group)
                .and_then(|group| group.find_subcommand(tool.leaf))
            else {
                continue;
            };

            let mut args = Vec::new();
            let mut properties = Map::new();
            let mut required = Vec::new();

            for arg in cmd.get_arguments() {
                // NOTE: named as the CLI spells it, the long flag or the
                // value name of a positional, never clap's field id.
                let name = match arg.get_long() {
                    Some(long) => long.to_string(),
                    None => arg
                        .get_value_names()
                        .and_then(|names| names.first())
                        .map(|name| name.as_str().to_lowercase())
                        .unwrap_or_else(|| arg.get_id().to_string()),
                };

                let gated = (SEND_ARGS.contains(&name.as_str()) && !allow_send)
                    || (WRITE_ARGS.contains(&name.as_str()) && !allow_write);

                if gated || tool.without.contains(&name.as_str()) || !exposes(arg) {
                    continue;
                }

                let spec = McpToolArg {
                    name,
                    long: arg.get_long().map(ToString::to_string),
                    index: arg.get_index(),
                    boolean: matches!(arg.get_action(), ArgAction::SetTrue | ArgAction::Count),
                    multiple: matches!(arg.get_action(), ArgAction::Append)
                        || arg.get_num_args().is_some_and(|n| n.max_values() > 1),
                };

                properties.insert(spec.name.clone(), spec.schema(arg));

                // NOTE: a positional taking at least one value is required
                // without being marked so.
                let at_least_one =
                    arg.is_positional() && arg.get_num_args().is_some_and(|n| n.min_values() > 0);

                if arg.is_required_set() || at_least_one {
                    required.push(Value::from(spec.name.as_str()));
                }

                args.push(spec);
            }

            let writes = |arg: &McpToolArg| {
                WRITE_ARGS.contains(&arg.name.as_str()) || SEND_ARGS.contains(&arg.name.as_str())
            };
            let read_only = tool.kind == McpToolKind::Read && !args.iter().any(writes);
            let sends = tool.kind == McpToolKind::Send
                || args
                    .iter()
                    .any(|arg| SEND_ARGS.contains(&arg.name.as_str()));

            let mut description = cmd
                .get_long_about()
                .or(cmd.get_about())
                .map(ToString::to_string)
                .unwrap_or_default();

            if (tool.group, tool.leaf) == ("envelope", "search")
                && let Some(resolution) = resolution
            {
                description.push_str(&search_options(resolution));
            }

            let mut definition = json!({
                "name": format!("{}_{}", tool.group, tool.leaf),
                "description": description,
                "inputSchema": {
                    "type": "object",
                    "properties": properties,
                    "required": required,
                    "additionalProperties": false,
                },
                "annotations": {
                    "readOnlyHint": read_only,
                    "destructiveHint": tool.destructive,
                    "openWorldHint": sends,
                },
            });

            // NOTE: MCP wants an object root, which a tagged or untagged
            // enum payload would not have.
            let output_schema = schemas
                .get(&format!("himalaya-{}-{}", tool.group, tool.leaf))
                .filter(|schema| tool.typed && schema["type"] == "object");

            if let Some(schema) = output_schema {
                definition["outputSchema"] = schema.clone();
            }

            tools.push(Self {
                group: tool.group,
                leaf: tool.leaf,
                args,
                definition,
            });
        }

        tools
    }

    /// Turns a call's arguments back into the argv of the command,
    /// refusing a key the tool does not take.
    ///
    /// Option values are glued to their flag and positionals follow a
    /// `--`, so a value starting with a dash is never read as a flag.
    fn argv(&self, arguments: &Map<String, Value>) -> Result<Vec<String>> {
        if let Some(key) = arguments
            .keys()
            .find(|key| !self.args.iter().any(|arg| &arg.name == *key))
        {
            let known: Vec<&str> = self.args.iter().map(|arg| arg.name.as_str()).collect();
            return Err(anyhow!(
                "Unknown argument `{key}`, expected one of: {}",
                known.join(", ")
            ));
        }

        let mut argv = vec![self.group.to_string(), self.leaf.to_string()];
        let mut positionals = Vec::new();

        for arg in &self.args {
            let Some(value) = arguments.get(&arg.name).filter(|value| !value.is_null()) else {
                continue;
            };

            if arg.boolean {
                let enabled = value
                    .as_bool()
                    .ok_or_else(|| anyhow!("Argument `{}` must be a boolean", arg.name))?;

                if enabled && let Some(long) = &arg.long {
                    argv.push(format!("--{long}"));
                }

                continue;
            }

            let values = match value {
                Value::Array(items) if arg.multiple => items.iter().map(scalar).collect(),
                value if !arg.multiple => scalar(value).map(|value| vec![value]),
                _ => None,
            };

            let Some(values) = values else {
                let expected = if arg.multiple {
                    "an array of strings"
                } else {
                    "a string"
                };
                return Err(anyhow!("Argument `{}` must be {expected}", arg.name));
            };

            match (&arg.long, arg.index) {
                (Some(long), _) => {
                    argv.extend(values.iter().map(|value| format!("--{long}={value}")))
                }
                (None, Some(index)) => positionals.push((index, values)),
                (None, None) => (),
            }
        }

        if !positionals.is_empty() {
            positionals.sort_by_key(|(index, _)| *index);
            argv.push(String::from("--"));
            argv.extend(positionals.into_iter().flat_map(|(_, values)| values));
        }

        Ok(argv)
    }
}

impl McpToolArg {
    /// The JSON Schema of the argument's property.
    fn schema(&self, arg: &Arg) -> Value {
        let enumerated: Vec<Value> = arg
            .get_possible_values()
            .iter()
            .filter(|value| !value.is_hide_set())
            .map(|value| Value::from(value.get_name()))
            .collect();

        let mut item = json!({ "type": "string" });

        if !enumerated.is_empty() {
            item["enum"] = Value::Array(enumerated);
        }

        let mut schema = match (self.boolean, self.multiple) {
            (true, _) => json!({ "type": "boolean" }),
            (false, true) => json!({ "type": "array", "items": item }),
            (false, false) => item,
        };

        if let Some(help) = arg.get_long_help().or(arg.get_help()) {
            schema["description"] = Value::from(help.to_string());
        }

        schema
    }
}

/// The server: the tools it lists and the session they run over.
struct McpServer<'a, 'b> {
    session: &'a mut Session<'b>,
    tools: Vec<McpToolSpec>,
}

impl McpServer<'_, '_> {
    /// Answers one JSON-RPC message, `None` for a notification or a
    /// response.
    fn answer(&mut self, line: &str) -> Option<Value> {
        let message: Value = match serde_json::from_str(line) {
            Ok(message) => message,
            Err(err) => return Some(failure(Value::Null, -32700, format!("Parse error: {err}"))),
        };

        let id = message.get("id").cloned();
        let params = message.get("params").cloned().unwrap_or(Value::Null);

        let Some(method) = message.get("method").and_then(Value::as_str) else {
            // NOTE: a response to a request the server never sends.
            return id.map(|id| failure(id, -32600, String::from("Invalid request")));
        };

        let id = id?;

        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "himalaya", "version": env!("CARGO_PKG_VERSION") },
            })),
            "ping" => Ok(json!({})),
            "tools/list" => {
                let tools: Vec<&Value> = self.tools.iter().map(|tool| &tool.definition).collect();
                Ok(json!({ "tools": tools }))
            }
            "tools/call" => self.call(&params),
            method => Err((-32601, format!("Method not found: {method}"))),
        };

        Some(match result {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, message)) => failure(id, code, message),
        })
    }

    /// Runs a tool call, a failure of the command being a result with
    /// `isError` rather than a protocol error, so the caller reads it.
    fn call(&mut self, params: &Value) -> Result<Value, (i64, String)> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or((-32602, String::from("Missing tool name")))?;

        let tool = self
            .tools
            .iter()
            .find(|tool| tool.definition["name"] == name)
            .ok_or_else(|| (-32602, format!("Unknown tool: {name}")))?;

        let empty = Map::new();
        let arguments = match params.get("arguments") {
            None | Some(Value::Null) => &empty,
            Some(Value::Object(arguments)) => arguments,
            Some(_) => return Err((-32602, String::from("Arguments must be an object"))),
        };

        let mut printer = McpPrinter::default();

        let result = tool
            .argv(arguments)
            .and_then(|argv| Ok(McpLine::try_parse_from(argv)?))
            .and_then(|line| line.cmd.execute(&mut printer, self.session));

        if let Err(err) = result {
            let report = serde_json::to_value(ErrorReport::<ErrorCode>::from(err))
                .unwrap_or_else(|err| Value::from(err.to_string()));

            return Ok(json!({
                "content": [{ "type": "text", "text": report.to_string() }],
                "isError": true,
            }));
        }

        let mut values = printer.values;

        let Some(value) = values.pop().filter(|_| values.is_empty()) else {
            let message = format!("The command printed {} values, one expected", values.len());
            return Ok(json!({
                "content": [{ "type": "text", "text": message }],
                "isError": true,
            }));
        };

        let mut result = json!({ "content": [{ "type": "text", "text": value.to_string() }] });

        if value.is_object() {
            result["structuredContent"] = value;
        }

        Ok(result)
    }
}

/// One tool call parsed back, the CLI grammar minus the binary name and the
/// global flags.
#[derive(Debug, Parser)]
#[command(no_binary_name = true)]
struct McpLine {
    #[command(subcommand)]
    cmd: Command,
}

/// A [`Printer`] keeping each value a command prints as JSON.
#[derive(Default)]
struct McpPrinter {
    values: Vec<Value>,
}

impl Printer for McpPrinter {
    fn out<T: fmt::Display + Serialize>(&mut self, data: T) -> Result<()> {
        self.values.push(serde_json::to_value(&data)?);
        Ok(())
    }

    fn is_json(&self) -> bool {
        true
    }
}

/// Whether an argument becomes a tool property: not a global flag, not
/// help or version, not hidden, and not a path read from or written to.
fn exposes(arg: &Arg) -> bool {
    let builtin = matches!(
        arg.get_action(),
        ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
    );

    let path = matches!(
        arg.get_value_hint(),
        ValueHint::FilePath | ValueHint::DirPath | ValueHint::AnyPath | ValueHint::ExecutablePath
    ) || arg
        .get_value_names()
        .is_some_and(|names| names.iter().any(|name| name.as_str() == "PATH"));

    let reachable = arg.get_long().is_some() || arg.is_positional();

    // NOTE: the width only shapes a table, which JSON output never draws.
    let display = arg.get_long() == Some("max-width");

    !builtin && !path && !display && reachable && !arg.is_global_set() && !arg.is_hide_set()
}

/// A JSON scalar as the word an argv carries.
fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

/// What the account's backend answers of the query language, appended to
/// the `envelope_search` description so an agent writes a query it can
/// run: the clauses and sort keys answered fully, in part and why, and
/// not at all.
fn search_options(resolution: &Resolution) -> String {
    let Some(backend) = resolution.storage else {
        return String::new();
    };

    let mut full = Vec::new();
    let mut partial = Vec::new();
    let mut refused = Vec::new();

    let options = capability::leaf("envelope", "search")
        .map(|leaf| leaf.capabilities)
        .unwrap_or_default();

    for capability in options {
        let Capability::Search(option) = capability else {
            continue;
        };

        let word = option.word();
        match resolution.support(*capability) {
            Support::Full => full.push(word.to_string()),
            Support::Partial(detail) | Support::Server(detail) => {
                partial.push(format!("{word} ({detail})"))
            }
            Support::None(_) => refused.push(word.to_string()),
        }
    }

    let mut text = format!("\n\nThe account's backend ({backend}) answers");
    if !full.is_empty() {
        text.push_str(&format!(" fully: {}", full.join(", ")));
    }
    if !partial.is_empty() {
        text.push_str(&format!("; in part: {}", partial.join("; ")));
    }
    if !refused.is_empty() {
        text.push_str(&format!(
            "; not at all, refused as `search-unsupported`: {}",
            refused.join(", ")
        ));
    }
    text.push('.');

    text
}

/// A JSON-RPC error response.
fn failure(id: Value, code: i64, message: String) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use crate::{cli::Cli, mcp::*};

    /// The shared API leaves no tool exposes: `message parse` reads a local
    /// file the caller names, `attachment download` writes files on disk.
    const EXCLUDED: &[(&str, &str)] = &[("message", "parse"), ("attachment", "download")];

    /// The command groups of the shared API.
    const GROUPS: &[&str] = &["mailbox", "envelope", "flag", "message", "attachment"];

    #[test]
    fn every_shared_leaf_is_classified() {
        let mut cli = Cli::command();
        cli.build();

        for group in GROUPS {
            let Some(group) = cli.find_subcommand(group) else {
                continue;
            };

            for leaf in group
                .get_subcommands()
                .filter(|leaf| leaf.get_name() != "help")
            {
                let path = (group.get_name(), leaf.get_name());
                let classified = TOOLS.iter().any(|tool| (tool.group, tool.leaf) == path)
                    || EXCLUDED.contains(&path);

                assert!(classified, "{path:?} is neither a tool nor excluded");
            }
        }
    }
}
