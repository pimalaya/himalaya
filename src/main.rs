mod config;
mod imap;
mod io;
mod mbox;
mod msg;
mod smtp;
mod table;

use clap::{App, AppSettings, Arg, SubCommand};
use std::{fmt, fs, process::exit, result};

use crate::config::Config;
use crate::imap::ImapConnector;
use crate::msg::Msg;
use crate::table::DisplayTable;

const DEFAULT_PAGE_SIZE: usize = 10;
const DEFAULT_PAGE: usize = 0;

#[derive(Debug)]
pub enum Error {
    ConfigError(config::Error),
    IoError(io::Error),
    MsgError(msg::Error),
    ImapError(imap::Error),
    SmtpError(smtp::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ConfigError(err) => err.fmt(f),
            Error::IoError(err) => err.fmt(f),
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

impl From<crate::io::Error> for Error {
    fn from(err: crate::io::Error) -> Error {
        Error::IoError(err)
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

fn page_size_arg<'a>(default: &'a str) -> Arg<'a, 'a> {
    Arg::with_name("size")
        .help("Page size")
        .short("s")
        .long("size")
        .value_name("INT")
        .default_value(default)
}

fn page_arg<'a>(default: &'a str) -> Arg<'a, 'a> {
    Arg::with_name("page")
        .help("Page number")
        .short("p")
        .long("page")
        .value_name("INT")
        .default_value(default)
}

fn run() -> Result<()> {
    let default_page_size_str = &DEFAULT_PAGE_SIZE.to_string();
    let default_page_str = &DEFAULT_PAGE.to_string();

    let matches = App::new("Himalaya")
        .version("0.1.0")
        .about("📫 Minimalist CLI email client")
        .author("soywod <clement.douin@posteo.net>")
        .setting(AppSettings::ArgRequiredElseHelp)
        .arg(
            Arg::with_name("account")
                .long("account")
                .short("a")
                .help("Name of the config file to use")
                .value_name("STRING"),
        )
        .subcommand(
            SubCommand::with_name("mailboxes")
                .aliases(&["mboxes", "mb", "m"])
                .about("Lists all available mailboxes"),
        )
        .subcommand(
            SubCommand::with_name("list")
                .aliases(&["lst", "l"])
                .about("Lists emails sorted by arrival date")
                .arg(mailbox_arg())
                .arg(page_size_arg(default_page_size_str))
                .arg(page_arg(default_page_str)),
        )
        .subcommand(
            SubCommand::with_name("search")
                .aliases(&["query", "q", "s"])
                .about("Lists emails matching the given IMAP query")
                .arg(mailbox_arg())
                .arg(page_size_arg(default_page_size_str))
                .arg(page_arg(default_page_str))
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
                .aliases(&["r"])
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
                .aliases(&["attach", "a"])
                .about("Downloads all attachments from an email")
                .arg(uid_arg())
                .arg(mailbox_arg()),
        )
        .subcommand(SubCommand::with_name("write").about("Writes a new email"))
        .subcommand(
            SubCommand::with_name("reply")
                .aliases(&["rep", "re"])
                .about("Answers to an email")
                .arg(uid_arg())
                .arg(mailbox_arg())
                .arg(
                    Arg::with_name("reply-all")
                        .help("Includs all recipients")
                        .short("a")
                        .long("all"),
                ),
        )
        .subcommand(
            SubCommand::with_name("forward")
                .aliases(&["fwd", "f"])
                .about("Forwards an email")
                .arg(uid_arg())
                .arg(mailbox_arg()),
        )
        .get_matches();

    let account_name = matches.value_of("account");

    if let Some(_) = matches.subcommand_matches("mailboxes") {
        let config = Config::new_from_file()?;
        let account = config.get_account(account_name)?;
        let mut imap_conn = ImapConnector::new(&account)?;

        let mboxes = imap_conn.list_mboxes()?;
        println!("{}", mboxes.to_table());

        imap_conn.close();
    }

    if let Some(matches) = matches.subcommand_matches("list") {
        let config = Config::new_from_file()?;
        let account = config.get_account(account_name)?;
        let mut imap_conn = ImapConnector::new(&account)?;

        let mbox = matches.value_of("mailbox").unwrap();
        let page_size: u32 = matches
            .value_of("size")
            .unwrap()
            .parse()
            .unwrap_or(DEFAULT_PAGE_SIZE as u32);
        let page: u32 = matches
            .value_of("page")
            .unwrap()
            .parse()
            .unwrap_or(DEFAULT_PAGE as u32);

        let msgs = imap_conn.list_msgs(&mbox, &page_size, &page)?;
        println!("{}", msgs.to_table());

        imap_conn.close();
    }

    if let Some(matches) = matches.subcommand_matches("search") {
        let config = Config::new_from_file()?;
        let account = config.get_account(account_name)?;
        let mut imap_conn = ImapConnector::new(&account)?;

        let mbox = matches.value_of("mailbox").unwrap();
        let page_size: usize = matches
            .value_of("size")
            .unwrap()
            .parse()
            .unwrap_or(DEFAULT_PAGE_SIZE);
        let page: usize = matches
            .value_of("page")
            .unwrap()
            .parse()
            .unwrap_or(DEFAULT_PAGE);
        let query = matches
            .values_of("query")
            .unwrap_or_default()
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

        let msgs = imap_conn.search_msgs(&mbox, &query, &page_size, &page)?;
        println!("{}", msgs.to_table());

        imap_conn.close();
    }

    if let Some(matches) = matches.subcommand_matches("read") {
        let config = Config::new_from_file()?;
        let account = config.get_account(account_name)?;
        let mut imap_conn = ImapConnector::new(&account)?;

        let mbox = matches.value_of("mailbox").unwrap();
        let uid = matches.value_of("uid").unwrap();
        let mime = format!("text/{}", matches.value_of("mime-type").unwrap());

        let msg = Msg::from(imap_conn.read_msg(&mbox, &uid)?);
        let text_bodies = msg.text_bodies(&mime)?;
        println!("{}", text_bodies);

        imap_conn.close();
    }

    if let Some(matches) = matches.subcommand_matches("attachments") {
        let config = Config::new_from_file()?;
        let account = config.get_account(account_name)?;
        let mut imap_conn = ImapConnector::new(&account)?;

        let mbox = matches.value_of("mailbox").unwrap();
        let uid = matches.value_of("uid").unwrap();

        let msg = Msg::from(imap_conn.read_msg(&mbox, &uid)?);
        let parts = msg.extract_attachments()?;

        if parts.is_empty() {
            println!("No attachment found for message {}", uid);
        } else {
            println!("{} attachment(s) found for message {}", parts.len(), uid);
            parts.iter().for_each(|(filename, bytes)| {
                let filepath = config.downloads_filepath(&account, &filename);
                println!("Downloading {} …", filename);
                fs::write(filepath, bytes).unwrap()
            });
            println!("Done!");
        }

        imap_conn.close();
    }

    if let Some(_) = matches.subcommand_matches("write") {
        let config = Config::new_from_file()?;
        let account = config.get_account(account_name)?;
        let mut imap_conn = ImapConnector::new(&account)?;

        let tpl = Msg::build_new_tpl(&config, &account)?;
        let content = io::open_editor_with_tpl(&tpl.as_bytes())?;
        let msg = Msg::from(content);

        io::ask_for_confirmation("Send the message?")?;

        println!("Sending …");
        smtp::send(&account, &msg.to_sendable_msg()?)?;
        imap_conn.append_msg("Sent", &msg.to_vec()?)?;
        println!("Done!");

        imap_conn.close();
    }

    if let Some(matches) = matches.subcommand_matches("reply") {
        let config = Config::new_from_file()?;
        let account = config.get_account(account_name)?;
        let mut imap_conn = ImapConnector::new(&account)?;

        let mbox = matches.value_of("mailbox").unwrap();
        let uid = matches.value_of("uid").unwrap();

        let msg = Msg::from(imap_conn.read_msg(&mbox, &uid)?);
        let tpl = if matches.is_present("reply-all") {
            msg.build_reply_all_tpl(&config, &account)?
        } else {
            msg.build_reply_tpl(&config, &account)?
        };

        let content = io::open_editor_with_tpl(&tpl.as_bytes())?;
        let msg = Msg::from(content);

        io::ask_for_confirmation("Send the message?")?;

        println!("Sending …");
        smtp::send(&account, &msg.to_sendable_msg()?)?;
        imap_conn.append_msg("Sent", &msg.to_vec()?)?;
        println!("Done!");

        imap_conn.close();
    }

    if let Some(matches) = matches.subcommand_matches("forward") {
        let config = Config::new_from_file()?;
        let account = config.get_account(account_name)?;
        let mut imap_conn = ImapConnector::new(&account)?;

        let mbox = matches.value_of("mailbox").unwrap();
        let uid = matches.value_of("uid").unwrap();

        let msg = Msg::from(imap_conn.read_msg(&mbox, &uid)?);
        let tpl = msg.build_forward_tpl(&config, &account)?;
        let content = io::open_editor_with_tpl(&tpl.as_bytes())?;
        let msg = Msg::from(content);

        io::ask_for_confirmation("Send the message?")?;

        println!("Sending …");
        smtp::send(&account, &msg.to_sendable_msg()?)?;
        imap_conn.append_msg("Sent", &msg.to_vec()?)?;
        println!("Done!");

        imap_conn.close();
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
