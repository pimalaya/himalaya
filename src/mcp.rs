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
//! A tool may also take a [`ServerArg`], an input the CLI says another
//! way (a path the server keeps closed): declared on the tool in
//! [`TOOLS`], taken off the call before the argv is rebuilt, validated
//! here, and handed to the parsed command through a field clap skips.
//!
//! Refs: <https://modelcontextprotocol.io/specification/2025-06-18>

use std::{
    fmt,
    io::{self, BufRead, Write},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

use anyhow::{Context, Result, anyhow};
use clap::{Arg, ArgAction, CommandFactory, Parser, ValueHint};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use pimalaya_cli::{
    error::{ErrorReport, partial_outcome_of},
    output::Bytes,
    printer::Printer,
};
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
/// names) is left out.
const TOOLS: &[McpTool] = &[
    McpTool::read("mailbox", "list"),
    McpTool::write("mailbox", "create", false),
    McpTool::write("mailbox", "rename", true),
    McpTool::write("mailbox", "delete", true),
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
    McpTool::read("message", "compose")
        .untyped()
        .server(COMPOSER_ARGS),
    McpTool::read("message", "reply")
        .untyped()
        .server(COMPOSER_ARGS),
    McpTool::read("message", "forward")
        .untyped()
        .server(COMPOSER_ARGS),
    McpTool::read("attachment", "list"),
    // NOTE: always run with `--stdout`, which writes no file and reads
    // exactly one part, whose bytes travel as an embedded resource.
    McpTool::read("attachment", "download")
        .fixed(&["--stdout"])
        .one(&["attachment-id"])
        .part(),
    McpTool::read("vacation", "get"),
    // NOTE: an auto-reply once on answers incoming mail, so setting it
    // is sending, listed under `--allow-send` alone.
    McpTool::send("vacation", "set"),
];

/// The arguments turning a tool into one that changes the store, listed
/// only under `--allow-write`: saving a composed message, flagging a read
/// one as seen.
const WRITE_ARGS: &[&str] = &["save", "seen"];

/// The arguments turning a tool into one that sends mail, listed only
/// under `--allow-send`.
const SEND_ARGS: &[&str] = &["send"];

/// The server-only arguments of the composers: the attachments given
/// inline, which `--attach` takes as paths.
const COMPOSER_ARGS: &[ServerArg] = &[ServerArg {
    name: "attach-data",
    schema: attach_data_schema,
    hand: hand_attach_data,
}];

/// The default of `--max-attachment-size`, 10 MiB.
const DEFAULT_MAX_ATTACHMENT_SIZE: u64 = 10 * 1024 * 1024;

/// Whether local file paths given as command input are read as such.
static PATHS_CLOSED: AtomicBool = AtomicBool::new(false);

/// The decoded size a part read inside the server may reach, unbounded
/// (`u64::MAX`) outside it.
static ATTACHMENT_CAP: AtomicU64 = AtomicU64::new(u64::MAX);

/// The characters a segment of a part URI keeps as they are: RFC 3986's
/// unreserved ones.
const URI_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

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
    /// Adds the mailbox create, rename and delete tools, the flag, add,
    /// copy, move and delete tools, and the `save` argument of the
    /// composers and of `message_send`.
    #[arg(long)]
    pub allow_write: bool,
    /// List the tools that send mail too.
    ///
    /// Adds `message_send`, `vacation_set`, an auto-reply answering mail,
    /// and the `send` argument of the composers and of `message_add`. An agent reading mail can be told by a message
    /// to send mail, so sending is never on by default.
    #[arg(long)]
    pub allow_send: bool,
    /// Largest attachment a tool takes or reads, in bytes.
    ///
    /// An attachment over it is refused with `attachment-too-large`, one
    /// given inline before it is decoded, one read before it is encoded
    /// so it never reaches the model. A server flag rather than a tool
    /// argument, so the model cannot raise it.
    #[arg(long, value_name = "BYTES", default_value_t = DEFAULT_MAX_ATTACHMENT_SIZE)]
    pub max_attachment_size: u64,
}

impl McpCommand {
    /// Serves JSON-RPC requests off stdin until EOF, answering each on
    /// stdout.
    pub fn execute(self, _printer: &mut impl Printer, session: &mut Session) -> Result<()> {
        let account = session.account;

        // NOTE: resolved once up front, so a missing configuration or
        // account fails the server rather than its first call, and the
        // tools listed are the ones the account's backends serve.
        let (_, name, account_config) = resolve_account(account.config_paths, account.name)?;
        let resolution = Resolution::of(&name, &account_config, account.backend);

        repl::claim_stdin();
        PATHS_CLOSED.store(true, Ordering::Relaxed);
        ATTACHMENT_CAP.store(self.max_attachment_size, Ordering::Relaxed);

        let mut server = McpServer {
            session,
            account: name,
            tools: McpToolSpec::all(self.allow_write, self.allow_send, Some(&resolution)),
            limits: McpLimits {
                max_attachment_size: self.max_attachment_size,
            },
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

/// The decoded size a part read as command output may reach, `None`
/// outside an MCP server.
pub fn max_attachment_size() -> Option<u64> {
    Some(ATTACHMENT_CAP.load(Ordering::Relaxed)).filter(|max| *max != u64::MAX)
}

/// A shared API leaf as a tool, and what it may do.
struct McpTool {
    group: &'static str,
    leaf: &'static str,
    kind: McpToolKind,
    destructive: bool,
    typed: bool,
    without: &'static [&'static str],
    server: &'static [ServerArg],
    fixed: &'static [&'static str],
    one: &'static [&'static str],
    part: bool,
}

/// An argument a tool takes that its command's grammar does not, for
/// what the CLI says through a path or stdin the server keeps closed.
///
/// It is a property of the tool's `inputSchema`, listed whenever the tool
/// is, taken off the call before the argv is rebuilt so clap never sees
/// it, then validated by `hand`, which sets it on the parsed command
/// through a field clap skips (`#[arg(skip)]`), so no command line can
/// reach it.
struct ServerArg {
    /// The property name, spelled as a CLI flag would be.
    name: &'static str,
    /// The JSON Schema of the property.
    schema: fn() -> Value,
    /// Validates the value and sets it on the parsed command, a bad value
    /// failing with a stable code before the command runs.
    hand: fn(&Value, &McpLimits, &mut Command) -> Result<()>,
}

/// The bounds the server puts on what a call carries.
#[cfg_attr(not(any(backend, feature = "smtp")), allow(dead_code))]
struct McpLimits {
    /// The largest attachment a call may carry, in bytes.
    max_attachment_size: u64,
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
            server: &[],
            fixed: &[],
            one: &[],
            part: false,
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

    const fn server(self, server: &'static [ServerArg]) -> Self {
        Self { server, ..self }
    }

    /// Arguments appended to every call's argv, before its positionals:
    /// the command runs as with them typed, and an argument they set is
    /// no property, so the call cannot unset it.
    const fn fixed(self, fixed: &'static [&'static str]) -> Self {
        Self { fixed, ..self }
    }

    /// Repeatable arguments this tool takes exactly one value of.
    const fn one(self, one: &'static [&'static str]) -> Self {
        Self { one, ..self }
    }

    /// The command prints one part, `{id, mime, filename, size, data}`,
    /// whose `data` becomes an embedded resource.
    const fn part(self) -> Self {
        Self { part: true, ..self }
    }

    /// Whether `arg` is set by one of the fixed arguments.
    fn fixes(&self, arg: &Arg) -> bool {
        arg.get_long().is_some_and(|long| {
            self.fixed.iter().any(|fixed| {
                fixed
                    .strip_prefix("--")
                    .and_then(|fixed| fixed.split('=').next())
                    == Some(long)
            })
        })
    }
}

/// A listed tool: its definition and the arguments a call maps onto.
struct McpToolSpec {
    group: &'static str,
    leaf: &'static str,
    args: Vec<McpToolArg>,
    server: &'static [ServerArg],
    fixed: &'static [&'static str],
    part: bool,
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

                if gated
                    || tool.without.contains(&name.as_str())
                    || tool.fixes(arg)
                    || !exposes(arg)
                {
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

                let one = spec.multiple && tool.one.contains(&spec.name.as_str());
                let mut schema = spec.schema(arg);

                if one {
                    schema["minItems"] = Value::from(1);
                    schema["maxItems"] = Value::from(1);
                }

                properties.insert(spec.name.clone(), schema);

                // NOTE: a positional taking at least one value is required
                // without being marked so.
                let at_least_one =
                    arg.is_positional() && arg.get_num_args().is_some_and(|n| n.min_values() > 0);

                if arg.is_required_set() || at_least_one || one {
                    required.push(Value::from(spec.name.as_str()));
                }

                args.push(spec);
            }

            // NOTE: what the grammar does not say, declared on the tool.
            for arg in tool.server {
                properties.insert(arg.name.to_string(), (arg.schema)());
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

            if !tool.fixed.is_empty() {
                description.push_str(&format!("\n\nAlways run with `{}`.", tool.fixed.join(" ")));
            }

            if tool.part {
                let max = max_attachment_size().unwrap_or(DEFAULT_MAX_ATTACHMENT_SIZE);
                description.push_str(&format!(
                    " The part's bytes come as an embedded resource rather than as `data`; \
                     a part over {max} bytes is refused as `attachment-too-large`."
                ));
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
            // enum payload would not have. A part tool prints one shape of
            // its two, the part, `data` moved out of it.
            let schema = schemas.get(&format!("himalaya-{}-{}", tool.group, tool.leaf));
            let output_schema = match schema {
                Some(schema) if tool.part => part_schema(schema),
                Some(schema) if tool.typed && schema["type"] == "object" => Some(schema.clone()),
                _ => None,
            };

            if let Some(schema) = output_schema {
                definition["outputSchema"] = schema;
            }

            tools.push(Self {
                group: tool.group,
                leaf: tool.leaf,
                args,
                server: tool.server,
                fixed: tool.fixed,
                part: tool.part,
                definition,
            });
        }

        tools
    }

    /// Splits a call's arguments into those of the command's grammar and
    /// the server-only ones, which never reach the argv.
    fn split<'a>(
        &self,
        arguments: &'a Map<String, Value>,
    ) -> (Map<String, Value>, Vec<(&'static ServerArg, &'a Value)>) {
        let mut grammar = Map::new();
        let mut server = Vec::new();

        for (key, value) in arguments {
            match self.server.iter().find(|arg| arg.name == key) {
                Some(arg) => server.push((arg, value)),
                None => {
                    grammar.insert(key.clone(), value.clone());
                }
            }
        }

        (grammar, server)
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
            let known: Vec<&str> = self
                .args
                .iter()
                .map(|arg| arg.name.as_str())
                .chain(self.server.iter().map(|arg| arg.name))
                .collect();
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

        // NOTE: before the `--`, after which a flag would be a value.
        argv.extend(self.fixed.iter().map(ToString::to_string));

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
    /// The name of the account served, the first segment of a part URI.
    account: String,
    tools: Vec<McpToolSpec>,
    limits: McpLimits,
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
        let (grammar, server) = tool.split(arguments);
        let limits = &self.limits;

        let result = tool
            .argv(&grammar)
            .and_then(|argv| Ok(McpLine::try_parse_from(argv)?))
            .and_then(|mut line| {
                for (arg, value) in server {
                    (arg.hand)(value, limits, &mut line.cmd)?;
                }
                Ok(line)
            })
            .and_then(|line| line.cmd.execute(&mut printer, self.session));

        // NOTE: a partial outcome printed its result, which stays the tool
        // result, flagged as an error.
        let partial = match result {
            Err(err) if partial_outcome_of(&err).is_some() => true,
            Err(err) => {
                let report = serde_json::to_value(ErrorReport::<ErrorCode>::from(err))
                    .unwrap_or_else(|err| Value::from(err.to_string()));

                return Ok(json!({
                    "content": [{ "type": "text", "text": report.to_string() }],
                    "isError": true,
                }));
            }
            Ok(()) => false,
        };

        let mut values = printer.values;

        let Some(value) = values.pop().filter(|_| values.is_empty()) else {
            let message = format!("The command printed {} values, one expected", values.len());
            return Ok(json!({
                "content": [{ "type": "text", "text": message }],
                "isError": true,
            }));
        };

        // NOTE: the events a command printed come first, one text content
        // each, the result last.
        let mut content: Vec<Value> = printer
            .events
            .iter()
            .map(|event| json!({ "type": "text", "text": event.to_string() }))
            .collect();

        // NOTE: a part's bytes travel once, as an embedded resource after
        // the part's JSON, never also as its `data`.
        let (value, resource) = if tool.part {
            let uri = part_uri(
                &self.account,
                arguments
                    .get("mailbox")
                    .and_then(Value::as_str)
                    .unwrap_or("inbox"),
                arguments
                    .get("message-id")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                value["id"].as_str().unwrap_or_default(),
            );

            let Some((value, resource)) = embedded_part(value, uri) else {
                let message = "The command printed no part to embed";
                return Ok(json!({
                    "content": [{ "type": "text", "text": message }],
                    "isError": true,
                }));
            };

            (value, Some(resource))
        } else {
            (value, None)
        };

        content.push(json!({ "type": "text", "text": value.to_string() }));
        content.extend(resource);

        let mut result = json!({ "content": content });

        if value.is_object() {
            result["structuredContent"] = value;
        }

        if partial {
            result["isError"] = Value::Bool(true);
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

/// A [`Printer`] keeping each value and event a command prints as JSON.
#[derive(Default)]
struct McpPrinter {
    values: Vec<Value>,
    events: Vec<Value>,
}

impl Printer for McpPrinter {
    fn out<T: fmt::Display + Serialize>(&mut self, data: T) -> Result<()> {
        self.values.push(serde_json::to_value(&data)?);
        Ok(())
    }

    fn event<T: fmt::Display + Serialize>(&mut self, event: T) -> Result<()> {
        self.events.push(serde_json::to_value(&event)?);
        Ok(())
    }

    fn out_bytes(&mut self, bytes: Bytes) -> Result<()> {
        self.out(bytes)
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

    !builtin && !path && reachable && !arg.is_global_set() && !arg.is_hide_set()
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

/// The JSON Schema of `attach-data`: attachments given inline, each named,
/// its bytes in base64 or its text as is.
fn attach_data_schema() -> Value {
    json!({
        "type": "array",
        "description": "Files to attach, given inline rather than by path. Each needs a `filename` and exactly one of `data` (base64) or `text` (UTF-8, attached as is); `mimeType` is guessed from the filename when left out. An attachment larger than the server's limit is refused.",
        "items": {
            "type": "object",
            "properties": {
                "filename": {
                    "type": "string",
                    "description": "Name the attachment carries, which may hold any character but a control one.",
                },
                "mimeType": {
                    "type": "string",
                    "description": "MIME type as `type/subtype`, guessed from the filename when left out.",
                },
                "data": {
                    "type": "string",
                    "description": "Content in base64 (RFC 4648 standard alphabet, padding optional, no whitespace).",
                },
                "text": {
                    "type": "string",
                    "description": "Content as UTF-8 text, attached as is.",
                },
            },
            "required": ["filename"],
            "additionalProperties": false,
        },
    })
}

/// Sets `attach-data` on the composer it was given to, once every item is
/// valid.
#[cfg(any(backend, feature = "smtp"))]
fn hand_attach_data(value: &Value, limits: &McpLimits, cmd: &mut Command) -> Result<()> {
    use crate::shared::message::cli::MessageCommand;

    let attachments = inline::attachments(value, limits.max_attachment_size)?;

    match cmd {
        Command::Message(MessageCommand::Compose(cmd)) => cmd.attach_data = attachments,
        Command::Message(MessageCommand::Reply(cmd)) => cmd.attach_data = attachments,
        Command::Message(MessageCommand::Forward(cmd)) => cmd.attach_data = attachments,
        // NOTE: only the composers declare it.
        _ => return Err(anyhow!("`attach-data` is not taken by this tool")),
    }

    Ok(())
}

/// No composer is compiled in, so no tool declaring `attach-data` is
/// listed.
#[cfg(not(any(backend, feature = "smtp")))]
fn hand_attach_data(_: &Value, _: &McpLimits, _: &mut Command) -> Result<()> {
    Err(anyhow!("`attach-data` is not taken by this build"))
}

/// The attachments `attach-data` carries, validated, each refused before
/// it is decoded when over the cap.
#[cfg(any(backend, feature = "smtp"))]
mod inline {
    use anyhow::Result;
    use base64::{
        Engine, alphabet,
        engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
    };
    use pimalaya_cli::error::CodedError;
    use serde_json::{Map, Value};

    use crate::{error::ErrorCode, shared::message::builder::InlineAttachment};

    /// The RFC 4648 standard alphabet, padding optional.
    const BASE64: GeneralPurpose = GeneralPurpose::new(
        &alphabet::STANDARD,
        GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
    );

    /// The keys an item takes.
    const KEYS: &[&str] = &["filename", "mimeType", "data", "text"];

    /// Validates and decodes every item, the first bad one refusing the
    /// whole call.
    pub fn attachments(value: &Value, max: u64) -> Result<Vec<InlineAttachment>> {
        let Value::Array(items) = value else {
            return Err(invalid(String::from(
                "`attach-data` must be an array of objects",
            )));
        };

        items
            .iter()
            .enumerate()
            .map(|(index, item)| attachment(index, item, max))
            .collect()
    }

    /// One item, refused naming its index and its file name.
    fn attachment(index: usize, item: &Value, max: u64) -> Result<InlineAttachment> {
        let mut label = format!("`attach-data[{index}]`");

        let Value::Object(item) = item else {
            return Err(invalid(format!(
                "{label} must be an object {{filename, mimeType, data, text}}"
            )));
        };

        let filename = match item.get("filename") {
            Some(Value::String(filename)) => filename,
            Some(_) => return Err(invalid(format!("{label}: `filename` must be a string"))),
            None => return Err(invalid(format!("{label}: `filename` is missing"))),
        };

        if !filename.is_empty() {
            label.push_str(&format!(" (`{}`)", filename.escape_debug()));
        }

        if let Some(key) = item.keys().find(|key| !KEYS.contains(&key.as_str())) {
            return Err(invalid(format!(
                "{label}: unknown key `{key}`, expected one of: {}",
                KEYS.join(", ")
            )));
        }

        if filename.is_empty() {
            return Err(invalid(format!("{label}: `filename` is empty")));
        }

        if filename.chars().any(char::is_control) {
            return Err(invalid(format!(
                "{label}: `filename` holds a control character"
            )));
        }

        let mime = match item.get("mimeType") {
            None | Some(Value::Null) => None,
            Some(Value::String(mime)) => Some(mime_type(mime).ok_or_else(|| {
                invalid(format!(
                    "{label}: `mimeType` `{}` is no bare `type/subtype`",
                    mime.escape_debug()
                ))
            })?),
            Some(_) => return Err(invalid(format!("{label}: `mimeType` must be a string"))),
        };

        let content = content(&label, item, max)?;

        Ok(InlineAttachment {
            filename: filename.clone(),
            mime,
            content,
        })
    }

    /// The bytes of an item, its size checked against the cap before
    /// anything is decoded.
    fn content(label: &str, item: &Map<String, Value>, max: u64) -> Result<Vec<u8>> {
        let string = |key: &str| match item.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(value)) => Ok(Some(value)),
            Some(_) => Err(invalid(format!("{label}: `{key}` must be a string"))),
        };

        let (size, data, text) = match (string("data")?, string("text")?) {
            (Some(data), None) => (estimated_size(data), Some(data), None),
            (None, Some(text)) => (text.len() as u64, None, Some(text)),
            (Some(_), Some(_)) => {
                return Err(invalid(format!(
                    "{label}: give one of `data` and `text`, not both"
                )));
            }
            (None, None) => {
                return Err(invalid(format!(
                    "{label}: give one of `data` (base64) and `text`"
                )));
            }
        };

        if size > max {
            let message = format!(
                "{label} holds {size} bytes, over the server's --max-attachment-size of {max}"
            );
            return Err(CodedError::new(ErrorCode::AttachmentTooLarge, message).into());
        }

        match (data, text) {
            (Some(data), _) => BASE64
                .decode(data)
                .map_err(|err| invalid(format!("{label}: `data` is not base64: {err}"))),
            (_, Some(text)) => Ok(text.as_bytes().to_vec()),
            (None, None) => unreachable!("one of them is given"),
        }
    }

    /// The size the base64 `data` decodes to, read off its length alone:
    /// three bytes per four characters, less the padding.
    pub fn estimated_size(data: &str) -> u64 {
        let padding = data
            .bytes()
            .rev()
            .take(2)
            .filter(|byte| *byte == b'=')
            .count();
        (data.len() as u64 * 3 / 4).saturating_sub(padding as u64)
    }

    /// A bare `type/subtype` of RFC 2045 tokens, lowercased.
    fn mime_type(mime: &str) -> Option<String> {
        let token = |part: &str| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_graphic() && !b"()<>@,;:\\\"/[]?=".contains(&byte))
        };

        let (top, sub) = mime.split_once('/')?;
        (token(top) && token(sub)).then(|| mime.to_ascii_lowercase())
    }

    fn invalid(message: String) -> anyhow::Error {
        CodedError::new(ErrorCode::AttachmentInvalid, message).into()
    }
}

/// The output schema of a part tool: the registered `PartBytes`
/// definition without `data`, whose bytes travel as an embedded resource.
fn part_schema(schema: &Value) -> Option<Value> {
    let mut part = schema.get("$defs")?.get("PartBytes")?.clone();

    part.get_mut("properties")?
        .as_object_mut()?
        .remove("data")?;
    part.get_mut("required")?
        .as_array_mut()?
        .retain(|key| key != "data");

    if let Some(dialect) = schema.get("$schema") {
        part["$schema"] = dialect.clone();
    }

    Some(part)
}

/// The URI of a part the server read,
/// `himalaya://<account>/<mailbox>/<message-id>/<part-id>`, each segment
/// percent-encoded, so it maps back onto the call that read it.
fn part_uri(account: &str, mailbox: &str, message_id: &str, part_id: &str) -> String {
    let segment = |segment: &str| utf8_percent_encode(segment, URI_SEGMENT).to_string();
    format!(
        "himalaya://{}/{}/{}/{}",
        segment(account),
        segment(mailbox),
        segment(message_id),
        segment(part_id),
    )
}

/// A printed part split into its JSON without `data` and the embedded
/// resource carrying `data`, already base64, as its blob; `None` when the
/// value holds no `data` string.
fn embedded_part(mut part: Value, uri: String) -> Option<(Value, Value)> {
    let Value::String(blob) = part.as_object_mut()?.remove("data")? else {
        return None;
    };

    let resource = json!({
        "type": "resource",
        "resource": { "uri": uri, "mimeType": part["mime"], "blob": blob },
    });

    Some((part, resource))
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
    /// file the caller names.
    const EXCLUDED: &[(&str, &str)] = &[("message", "parse")];

    /// The command groups of the shared API.
    const GROUPS: &[&str] = &[
        "mailbox",
        "envelope",
        "flag",
        "message",
        "attachment",
        "vacation",
    ];

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

    /// The listed tool `name`, every gate open.
    #[cfg(any(backend, feature = "smtp"))]
    fn spec(name: &str) -> McpToolSpec {
        McpToolSpec::all(true, true, None)
            .into_iter()
            .find(|tool| tool.definition["name"] == name)
            .unwrap()
    }

    #[cfg(any(backend, feature = "smtp"))]
    #[test]
    fn a_server_only_argument_is_declared_on_its_tool_and_not_in_clap() {
        for name in ["message_compose", "message_reply", "message_forward"] {
            let tool = spec(name);
            let properties = &tool.definition["inputSchema"]["properties"];

            assert_eq!(properties["attach-data"], attach_data_schema(), "{name}");
            assert!(tool.args.iter().all(|arg| arg.name != "attach-data"));
        }

        for tool in McpToolSpec::all(true, true, None) {
            let declared = tool.server.iter().any(|arg| arg.name == "attach-data");
            let composer = (tool.group, tool.leaf) == ("message", "compose")
                || (tool.group, tool.leaf) == ("message", "reply")
                || (tool.group, tool.leaf) == ("message", "forward");
            assert_eq!(declared, composer, "{}", tool.definition["name"]);
        }
    }

    #[cfg(any(backend, feature = "smtp"))]
    #[test]
    fn a_server_only_argument_is_taken_off_before_the_argv_and_handed_to_the_command() {
        use crate::shared::message::{builder::InlineAttachment, cli::MessageCommand};

        let tool = spec("message_compose");
        let arguments = json!({
            "to": ["b@example.org"],
            "attach-data": [{ "filename": "a.txt", "text": "ok" }],
        });
        let (grammar, server) = tool.split(arguments.as_object().unwrap());

        assert!(grammar.get("attach-data").is_none());
        assert_eq!(server.len(), 1);

        let argv = tool.argv(&grammar).unwrap();
        assert!(
            argv.iter().all(|word| !word.contains("attach-data")),
            "{argv:?}"
        );

        let mut line = McpLine::try_parse_from(argv).unwrap();
        let limits = McpLimits {
            max_attachment_size: DEFAULT_MAX_ATTACHMENT_SIZE,
        };
        for (arg, value) in server {
            (arg.hand)(value, &limits, &mut line.cmd).unwrap();
        }

        let Command::Message(MessageCommand::Compose(cmd)) = line.cmd else {
            panic!("not a compose");
        };
        assert_eq!(
            cmd.attach_data,
            vec![InlineAttachment {
                filename: String::from("a.txt"),
                mime: None,
                content: b"ok".to_vec(),
            }]
        );

        // NOTE: the argv alone, as a command line would carry it, is
        // refused by clap.
        let refused = McpLine::try_parse_from(["message", "compose", "--attach-data=x"]);
        assert!(refused.is_err());
    }

    #[cfg(any(backend, feature = "smtp"))]
    #[test]
    fn a_fixed_argument_goes_before_the_positionals_and_is_no_property() {
        let tool = spec("attachment_download");
        let arguments = json!({
            "mailbox": "-odd",
            "message-id": "m2",
            "attachment-id": ["3"],
        });

        let argv = tool.argv(arguments.as_object().unwrap()).unwrap();
        assert_eq!(
            argv,
            [
                "attachment",
                "download",
                "--mailbox=-odd",
                "--stdout",
                "--",
                "m2",
                "3"
            ]
        );

        let properties = &tool.definition["inputSchema"]["properties"];
        assert!(properties.get("stdout").is_none(), "{properties}");
        assert!(properties.get("dir").is_none(), "{properties}");

        let refused = tool.argv(json!({ "stdout": true }).as_object().unwrap());
        assert!(refused.unwrap_err().to_string().contains("`stdout`"));
    }

    #[cfg(any(backend, feature = "smtp"))]
    #[test]
    fn an_argument_taking_one_value_is_a_required_array_of_one() {
        let tool = spec("attachment_download");
        let schema = &tool.definition["inputSchema"];

        assert_eq!(schema["properties"]["attachment-id"]["minItems"], json!(1));
        assert_eq!(schema["properties"]["attachment-id"]["maxItems"], json!(1));
        assert!(
            schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("attachment-id"))
        );
    }

    #[cfg(any(backend, feature = "smtp"))]
    #[test]
    fn the_download_output_schema_is_the_part_without_data() {
        let schema = &spec("attachment_download").definition["outputSchema"];

        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].get("data").is_none(), "{schema}");
        for key in ["id", "mime", "filename", "size"] {
            assert!(schema["properties"].get(key).is_some(), "{key}: {schema}");
        }
        assert!(
            !schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("data"))
        );
    }

    #[test]
    fn a_part_uri_percent_encodes_each_segment() {
        assert_eq!(part_uri("a", "inbox", "m2", "3"), "himalaya://a/inbox/m2/3");
        assert_eq!(
            part_uri("my work", "[Gmail]/All Mail", "<a/b@c>", "2"),
            "himalaya://my%20work/%5BGmail%5D%2FAll%20Mail/%3Ca%2Fb%40c%3E/2"
        );
    }

    #[test]
    fn a_part_moves_its_data_into_an_embedded_resource() {
        let part = json!({
            "id": "3",
            "mime": "application/pdf",
            "filename": "a.pdf",
            "size": 2,
            "data": "AAE=",
        });

        let (structured, resource) =
            embedded_part(part, String::from("himalaya://a/inbox/m2/3")).unwrap();

        assert_eq!(
            structured,
            json!({ "id": "3", "mime": "application/pdf", "filename": "a.pdf", "size": 2 })
        );
        assert_eq!(
            resource,
            json!({
                "type": "resource",
                "resource": {
                    "uri": "himalaya://a/inbox/m2/3",
                    "mimeType": "application/pdf",
                    "blob": "AAE=",
                },
            })
        );

        assert!(embedded_part(json!({ "id": "3" }), String::new()).is_none());
    }

    #[cfg(any(backend, feature = "smtp"))]
    #[test]
    fn an_unknown_key_names_the_server_only_arguments_too() {
        let tool = spec("message_compose");
        let arguments = json!({ "attachments": [] });
        let (grammar, _) = tool.split(arguments.as_object().unwrap());
        let err = tool.argv(&grammar).unwrap_err().to_string();

        assert!(err.contains("Unknown argument `attachments`"), "{err}");
        assert!(err.contains("attach-data"), "{err}");
    }

    #[cfg(any(backend, feature = "smtp"))]
    mod inline_attachments {
        use pimalaya_cli::error::code_of;
        use serde_json::json;

        use crate::{error::ErrorCode, mcp::inline};

        fn code(item: serde_json::Value, max: u64) -> Option<ErrorCode> {
            let err = inline::attachments(&json!([item]), max).unwrap_err();
            code_of::<ErrorCode>(&err)
        }

        #[test]
        fn the_size_of_base64_is_read_off_its_length() {
            assert_eq!(inline::estimated_size(""), 0);
            assert_eq!(inline::estimated_size("AAAA"), 3);
            assert_eq!(inline::estimated_size("AAAAAAAAAAA="), 8);
            assert_eq!(inline::estimated_size("AAAAAAAAAA=="), 7);
            assert_eq!(inline::estimated_size("iVBORw0KGgo"), 8);

            // NOTE: exactly 10 MiB of bytes is not over a 10 MiB cap.
            let encoded = "A".repeat(13_981_014) + "==";
            assert_eq!(inline::estimated_size(&encoded), 10 * 1024 * 1024);
        }

        #[test]
        fn data_and_text_decode_to_their_bytes() {
            let attachments = inline::attachments(
                &json!([
                    { "filename": "a.bin", "mimeType": "Application/X-Test", "data": "AP8=" },
                    { "filename": "b.txt", "text": "é" },
                ]),
                16,
            )
            .unwrap();

            assert_eq!(attachments[0].content, vec![0x00, 0xff]);
            assert_eq!(attachments[0].mime.as_deref(), Some("application/x-test"));
            assert_eq!(attachments[1].content, "é".as_bytes());
            assert_eq!(attachments[1].mime, None);
        }

        #[test]
        fn a_malformed_item_is_invalid() {
            let invalid = Some(ErrorCode::AttachmentInvalid);

            assert_eq!(
                code(json!({ "filename": "a", "data": "%%%%" }), 16),
                invalid
            );
            assert_eq!(
                code(json!({ "filename": "a", "data": "", "text": "" }), 16),
                invalid
            );
            assert_eq!(code(json!({ "filename": "a" }), 16), invalid);
            assert_eq!(code(json!({ "text": "x" }), 16), invalid);
            assert_eq!(code(json!({ "filename": 1, "text": "x" }), 16), invalid);
            assert_eq!(
                code(json!({ "filename": "a\u{7}", "text": "x" }), 16),
                invalid
            );
            assert_eq!(code(json!({ "filename": "a", "data": 1 }), 16), invalid);
            assert_eq!(
                code(json!({ "filename": "a", "text": "x", "x": 1 }), 16),
                invalid
            );
            for mime in [
                "text",
                "text/",
                "/plain",
                "text/plain; charset=utf-8",
                "te xt/plain",
            ] {
                let item = json!({ "filename": "a", "mimeType": mime, "text": "x" });
                assert_eq!(code(item, 16), invalid, "{mime}");
            }

            let err = inline::attachments(&json!({ "filename": "a" }), 16).unwrap_err();
            assert_eq!(code_of::<ErrorCode>(&err), invalid);
        }

        #[test]
        fn the_cap_is_checked_before_decoding() {
            let too_large = Some(ErrorCode::AttachmentTooLarge);

            assert_eq!(
                code(json!({ "filename": "a", "data": "!!!!!!!!" }), 5),
                too_large
            );
            assert_eq!(
                code(json!({ "filename": "a", "text": "123456" }), 5),
                too_large
            );
            assert_eq!(
                code(json!({ "filename": "a", "text": "ééé" }), 5),
                too_large
            );

            let err = inline::attachments(&json!([{ "filename": "a", "text": "123456" }]), 5)
                .unwrap_err()
                .to_string();
            assert!(err.contains("`attach-data[0]` (`a`)"), "{err}");
        }
    }
}
