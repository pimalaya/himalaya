mod config;
mod email;
mod imap;
mod input;
mod mailbox;
mod msg;
mod smtp;
mod table;

use clap::{App, AppSettings, Arg, SubCommand};
use std::{fmt, fs, process::exit, result};

use crate::config::Config;
use crate::imap::ImapConnector;
use crate::msg::Msg;
use crate::table::DisplayTable;

#[derive(Debug)]
pub enum Error {
    ConfigError(config::Error),
    InputError(input::Error),
    MsgError(msg::Error),
    ImapError(imap::Error),
    SmtpError(smtp::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ConfigError(err) => err.fmt(f),
            Error::InputError(err) => err.fmt(f),
            Error::MsgError(err) => err.fmt(f),
            Error::ImapError(err) => err.fmt(f),
            Error::SmtpError(err) => err.fmt(f),
        }
    }
}

impl From<config::Error> for Error {
    fn from(err: config::Error) -> Error {
        Error::ConfigError(err)
    }
}

impl From<input::Error> for Error {
    fn from(err: input::Error) -> Error {
        Error::InputError(err)
    }
}

impl From<msg::Error> for Error {
    fn from(err: msg::Error) -> Error {
        Error::MsgError(err)
    }
}

impl From<imap::Error> for Error {
    fn from(err: imap::Error) -> Error {
        Error::ImapError(err)
    }
}

impl From<smtp::Error> for Error {
    fn from(err: smtp::Error) -> Error {
        Error::SmtpError(err)
    }
}

// Result wrapper

type Result<T> = result::Result<T, Error>;

// Run

fn mailbox_arg() -> Arg<'static, 'static> {
    Arg::with_name("mailbox")
        .short("m")
        .long("mailbox")
        .help("Name of the mailbox")
        .value_name("STRING")
        .default_value("INBOX")
}

fn uid_arg() -> Arg<'static, 'static> {
    Arg::with_name("uid")
        .help("UID of the email")
        .value_name("UID")
        .required(true)
}

fn run() -> Result<()> {
    let matches = App::new("Himalaya")
        .version("0.1.0")
        .about("📫 Minimalist CLI email client")
        .author("soywod <clement.douin@posteo.net>")
        .setting(AppSettings::ArgRequiredElseHelp)
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
                .about("Reads text bodies of an email")
                .arg(uid_arg())
                .arg(mailbox_arg())
                .arg(
                    Arg::with_name("mime-type")
                        .help("MIME type to use")
                        .short("t")
                        .long("mime-type")
                        .value_name("STRING")
                        .possible_values(&["plain", "html"])
                        .default_value("plain"),
                ),
        )
        .subcommand(
            SubCommand::with_name("attachments")
                .about("Downloads all attachments from an email")
                .arg(uid_arg())
                .arg(mailbox_arg()),
        )
        .subcommand(SubCommand::with_name("write").about("Writes a new email"))
        .subcommand(
            SubCommand::with_name("reply")
                .about("Answers to an email")
                .arg(uid_arg())
                .arg(mailbox_arg())
                .arg(
                    Arg::with_name("reply-all")
                        .help("Including all recipients")
                        .short("a")
                        .long("all"),
                ),
        )
        .subcommand(
            SubCommand::with_name("forward")
                .about("Forwards an email")
                .arg(uid_arg())
                .arg(mailbox_arg()),
        )
        .get_matches();

    if let Some(_) = matches.subcommand_matches("list") {
        let config = Config::new_from_file()?;
        let mboxes = ImapConnector::new(&config.imap)?.list_mboxes()?.to_table();

        println!("{}", mboxes);
    }

    if let Some(matches) = matches.subcommand_matches("search") {
        let config = Config::new_from_file()?;
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

            let emails = ImapConnector::new(&config.imap)?
                .read_emails(&mbox, &query)?
                .to_table();

            println!("{}", emails);
        }
    }

    if let Some(matches) = matches.subcommand_matches("read") {
        let config = Config::new_from_file()?;
        let mbox = matches.value_of("mailbox").unwrap();
        let uid = matches.value_of("uid").unwrap();
        let mime = format!("text/{}", matches.value_of("mime-type").unwrap());
        let body = ImapConnector::new(&config.imap)?.read_email_body(&mbox, &uid, &mime)?;

        println!("{}", body);
    }

    if let Some(matches) = matches.subcommand_matches("attachments") {
        let config = Config::new_from_file()?;
        let mbox = matches.value_of("mailbox").unwrap();
        let uid = matches.value_of("uid").unwrap();
        let mut imap_conn = ImapConnector::new(&config.imap)?;

        let msg = imap_conn.read_msg(&mbox, &uid)?;
        let msg = Msg::from(&msg)?;
        let parts = msg.extract_parts()?;

        if parts.is_empty() {
            println!("No attachment found for message {}", uid);
        } else {
            println!("{} attachment(s) found for message {}", parts.len(), uid);
            msg.extract_parts()?.iter().for_each(|(filename, bytes)| {
                let filepath = config.downloads_filepath(&filename);
                println!("Downloading {} …", filename);
                fs::write(filepath, bytes).unwrap()
            });
            println!("Done!");
        }
    }

    if let Some(_) = matches.subcommand_matches("write") {
        let config = Config::new_from_file()?;
        let mut imap_conn = ImapConnector::new(&config.imap)?;
        let tpl = Msg::build_new_tpl(&config)?;
        let content = input::open_editor_with_tpl(&tpl.as_bytes())?;
        let msg = Msg::from(content.as_bytes())?;

        input::ask_for_confirmation("Send the message?")?;

        println!("Sending …");
        smtp::send(&config.smtp, &msg.to_sendable_msg()?)?;
        imap_conn.append_msg("Sent", &msg.to_vec()?)?;
        println!("Done!");
    }

    if let Some(matches) = matches.subcommand_matches("reply") {
        let config = Config::new_from_file()?;
        let mbox = matches.value_of("mailbox").unwrap();
        let uid = matches.value_of("uid").unwrap();
        let mut imap_conn = ImapConnector::new(&config.imap)?;

        let msg = imap_conn.read_msg(&mbox, &uid)?;
        let msg = Msg::from(&msg)?;

        let tpl = if matches.is_present("reply-all") {
            msg.build_reply_all_tpl(&config)?
        } else {
            msg.build_reply_tpl(&config)?
        };

        let content = input::open_editor_with_tpl(&tpl.as_bytes())?;
        let msg = Msg::from(content.as_bytes())?;

        input::ask_for_confirmation("Send the message?")?;

        println!("Sending …");
        smtp::send(&config.smtp, &msg.to_sendable_msg()?)?;
        imap_conn.append_msg("Sent", &msg.to_vec()?)?;
        println!("Done!");
    }

    if let Some(matches) = matches.subcommand_matches("forward") {
        let config = Config::new_from_file()?;
        let mbox = matches.value_of("mailbox").unwrap();
        let uid = matches.value_of("uid").unwrap();
        let mut imap_conn = ImapConnector::new(&config.imap)?;

        let msg = imap_conn.read_msg(&mbox, &uid)?;
        let msg = Msg::from(&msg)?;

        let tpl = msg.build_forward_tpl(&config)?;
        let content = input::open_editor_with_tpl(&tpl.as_bytes())?;
        let msg = Msg::from(content.as_bytes())?;

        input::ask_for_confirmation("Send the message?")?;

        println!("Sending …");
        smtp::send(&config.smtp, &msg.to_sendable_msg()?)?;
        imap_conn.append_msg("Sent", &msg.to_vec()?)?;
        println!("Done!");
    }

    Ok(())
}

// Main

fn main() {
    if let Err(err) = run() {
        eprintln!("Error {}", err);
        exit(1);
    }
}
