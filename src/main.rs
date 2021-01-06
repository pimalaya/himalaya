mod config;
mod editor;
mod email;
mod imap;
mod mailbox;
mod smtp;
mod table;

use clap::{App, Arg, SubCommand};
use std::{error, fmt, process::exit, result};

use crate::config::Config;
use crate::imap::ImapConnector;
use crate::table::DisplayTable;

#[derive(Debug)]
pub enum Error {
    EditorError(editor::Error),
    ImapError(imap::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::EditorError(err) => err.fmt(f),
            Error::ImapError(err) => err.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::EditorError(ref err) => Some(err),
            Error::ImapError(ref err) => Some(err),
        }
    }
}

impl From<editor::Error> for Error {
    fn from(err: editor::Error) -> Error {
        Error::EditorError(err)
    }
}

impl From<imap::Error> for Error {
    fn from(err: imap::Error) -> Error {
        Error::ImapError(err)
    }
}

// Result wrapper

type Result<T> = result::Result<T, Error>;

// Run

fn mailbox_arg() -> Arg<'static, 'static> {
    Arg::with_name("mailbox")
        .short("m")
        .long("mailbox")
        .help("Name of the targeted mailbox")
        .value_name("STRING")
        .default_value("INBOX")
}

fn uid_arg() -> Arg<'static, 'static> {
    Arg::with_name("uid")
        .help("UID of the targeted email")
        .value_name("UID")
        .required(true)
}

fn run() -> Result<()> {
    let matches = App::new("Himalaya")
        .version("0.1.0")
        .about("📫 Minimalist CLI email client")
        .author("soywod <clement.douin@posteo.net>")
        .subcommand(SubCommand::with_name("list").about("Lists all available mailboxes"))
        .subcommand(
            SubCommand::with_name("search")
                .about("Lists emails matching the given IMAP query")
                .arg(mailbox_arg())
                .arg(
                    Arg::with_name("query")
                        .help("IMAP query (see https://tools.ietf.org/html/rfc3501#section-6.4.4)")
                        .value_name("QUERY")
                        .multiple(true)
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("read")
                .about("Reads an email by its UID")
                .arg(uid_arg())
                .arg(mailbox_arg())
                .arg(
                    Arg::with_name("mime-type")
                        .help("MIME type to use")
                        .short("t")
                        .long("mime-type")
                        .value_name("STRING")
                        .possible_values(&["text/plain", "text/html"])
                        .default_value("text/plain"),
                ),
        )
        .subcommand(SubCommand::with_name("write").about("Writes a new email"))
        .subcommand(
            SubCommand::with_name("forward")
                .about("Forwards an email by its UID")
                .arg(uid_arg())
                .arg(mailbox_arg()),
        )
        .subcommand(
            SubCommand::with_name("reply")
                .about("Replies to an email by its UID")
                .arg(uid_arg())
                .arg(mailbox_arg())
                .arg(
                    Arg::with_name("reply all")
                        .help("Replies to all recipients")
                        .short("a")
                        .long("all"),
                ),
        )
        .get_matches();

    if let Some(_) = matches.subcommand_matches("list") {
        let config = Config::new_from_file();
        let mboxes = ImapConnector::new(config.imap)?
            .list_mailboxes()?
            .to_table();

        println!("{}", mboxes);
    }

    if let Some(matches) = matches.subcommand_matches("search") {
        let config = Config::new_from_file();
        let mbox = matches.value_of("mailbox").unwrap();

        if let Some(matches) = matches.values_of("query") {
            let query = matches
                .fold((false, vec![]), |(escape, mut cmds), cmd| {
                    match (cmd, escape) {
                        // Next command is an arg and needs to be escaped
                        ("subject", _) | ("body", _) | ("text", _) => {
                            cmds.push(cmd.to_string());
                            (true, cmds)
                        }
                        // Escaped arg commands
                        (_, true) => {
                            cmds.push(format!("\"{}\"", cmd));
                            (false, cmds)
                        }
                        // Regular commands
                        (_, false) => {
                            cmds.push(cmd.to_string());
                            (false, cmds)
                        }
                    }
                })
                .1
                .join(" ");

            let emails = ImapConnector::new(config.imap)?
                .read_emails(&mbox, &query)?
                .to_table();

            println!("{}", emails);
        }
    }

    if let Some(matches) = matches.subcommand_matches("read") {
        let config = Config::new_from_file();
        let mbox = matches.value_of("mailbox").unwrap();
        let uid = matches.value_of("uid").unwrap();
        let mime = matches.value_of("mime-type").unwrap();
        let email = ImapConnector::new(config.imap)?.read_email(&mbox, &uid, &mime)?;

        println!("{}", email);
    }

    if let Some(_) = matches.subcommand_matches("write") {
        let config = Config::new_from_file();
        let draft = editor::open_with_new_template()?;

        smtp::send(&config, draft.as_bytes());
    }

    Ok(())
}

// Main

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        exit(1);
    }
}
