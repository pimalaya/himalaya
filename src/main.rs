mod config;
mod imap;
mod input;
mod mbox;
mod msg;
mod output;
mod smtp;
mod table;

use clap::{App, AppSettings, Arg, SubCommand};
use std::{fmt, fs, process::exit, result};

use crate::config::Config;
use crate::imap::ImapConnector;
use crate::msg::{Attachments, Msg, ReadableMsg};
use crate::output::print;

const DEFAULT_PAGE_SIZE: usize = 10;
const DEFAULT_PAGE: usize = 0;

#[derive(Debug)]
pub enum Error {
    ConfigError(config::Error),
    InputError(input::Error),
    OutputError(output::Error),
    MsgError(msg::Error),
    ImapError(imap::Error),
    SmtpError(smtp::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ConfigError(err) => err.fmt(f),
            Error::InputError(err) => err.fmt(f),
            Error::OutputError(err) => err.fmt(f),
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

impl From<output::Error> for Error {
    fn from(err: output::Error) -> Error {
        Error::OutputError(err)
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

fn reply_all_arg() -> Arg<'static, 'static> {
    Arg::with_name("reply-all")
        .help("Includes all recipients")
        .short("a")
        .long("all")
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
            Arg::with_name("output")
                .long("output")
                .short("o")
                .help("Format of the output to print")
                .value_name("STRING")
                .possible_values(&["text", "json"])
                .default_value("text"),
        )
        .arg(
            Arg::with_name("account")
                .long("account")
                .short("a")
                .help("Name of the account to use")
                .value_name("STRING"),
        )
        .subcommand(
            SubCommand::with_name("mailboxes")
                .aliases(&["mboxes", "mbox", "mb", "m"])
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
                .aliases(&["attach", "att", "a"])
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
                .arg(reply_all_arg()),
        )
        .subcommand(
            SubCommand::with_name("forward")
                .aliases(&["fwd", "f"])
                .about("Forwards an email")
                .arg(uid_arg())
                .arg(mailbox_arg()),
        )
        .subcommand(
            SubCommand::with_name("send")
                .about("Sends a raw message")
                .arg(Arg::with_name("message").raw(true)),
        )
        .subcommand(
            SubCommand::with_name("save")
                .about("Saves a raw message in the given mailbox")
                .arg(mailbox_arg())
                .arg(Arg::with_name("message").raw(true)),
        )
        .subcommand(
            SubCommand::with_name("template")
                .aliases(&["tpl", "t"])
                .about("Generates a message template")
                .subcommand(
                    SubCommand::with_name("new")
                        .aliases(&["n"])
                        .about("Generates a new message template")
                        .arg(mailbox_arg()),
                )
                .subcommand(
                    SubCommand::with_name("reply")
                        .aliases(&["rep", "r"])
                        .about("Generates a reply message template")
                        .arg(uid_arg())
                        .arg(mailbox_arg())
                        .arg(reply_all_arg()),
                )
                .subcommand(
                    SubCommand::with_name("forward")
                        .aliases(&["fwd", "fw", "f"])
                        .about("Generates a forward message template")
                        .arg(uid_arg())
                        .arg(mailbox_arg()),
                ),
        )
        .get_matches();

    let account_name = matches.value_of("account");
    let output_type = matches.value_of("output").unwrap().to_owned();

    if let Some(_) = matches.subcommand_matches("mailboxes") {
        let config = Config::new_from_file()?;
        let account = config.find_account_by_name(account_name)?;
        let mut imap_conn = ImapConnector::new(&account)?;

        let mboxes = imap_conn.list_mboxes()?;
        print(&output_type, mboxes)?;

        imap_conn.logout();
    }

    if let Some(matches) = matches.subcommand_matches("list") {
        let config = Config::new_from_file()?;
        let account = config.find_account_by_name(account_name)?;
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
        print(&output_type, msgs)?;

        imap_conn.logout();
    }

    if let Some(matches) = matches.subcommand_matches("search") {
        let config = Config::new_from_file()?;
        let account = config.find_account_by_name(account_name)?;
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
        print(&output_type, msgs)?;

        imap_conn.logout();
    }

    if let Some(matches) = matches.subcommand_matches("read") {
        let config = Config::new_from_file()?;
        let account = config.find_account_by_name(account_name)?;
        let mut imap_conn = ImapConnector::new(&account)?;

        let mbox = matches.value_of("mailbox").unwrap();
        let uid = matches.value_of("uid").unwrap();
        let mime = format!("text/{}", matches.value_of("mime-type").unwrap());

        let msg = imap_conn.read_msg(&mbox, &uid)?;
        let msg = ReadableMsg::from_bytes(&mime, &msg)?;

        print(&output_type, msg)?;
        imap_conn.logout();
    }

    if let Some(matches) = matches.subcommand_matches("attachments") {
        let config = Config::new_from_file()?;
        let account = config.find_account_by_name(account_name)?;
        let mut imap_conn = ImapConnector::new(&account)?;

        let mbox = matches.value_of("mailbox").unwrap();
        let uid = matches.value_of("uid").unwrap();

        let msg = imap_conn.read_msg(&mbox, &uid)?;
        let attachments = Attachments::from_bytes(&msg)?;

        match output_type.as_str() {
            "text" => {
                println!(
                    "{} attachment(s) found for message {}",
                    attachments.0.len(),
                    uid
                );

                attachments.0.iter().for_each(|attachment| {
                    let filepath = config.downloads_filepath(&account, &attachment.filename);
                    println!("Downloading {}…", &attachment.filename);
                    fs::write(filepath, &attachment.raw).unwrap()
                });

                println!("Done!");
            }
            "json" => {
                attachments.0.iter().for_each(|attachment| {
                    let filepath = config.downloads_filepath(&account, &attachment.filename);
                    fs::write(filepath, &attachment.raw).unwrap()
                });

                print!("{{}}");
            }
            _ => (),
        }

        imap_conn.logout();
    }

    if let Some(_) = matches.subcommand_matches("write") {
        let config = Config::new_from_file()?;
        let account = config.find_account_by_name(account_name)?;
        let mut imap_conn = ImapConnector::new(&account)?;
        let tpl = Msg::build_new_tpl(&config, &account)?;
        let content = input::open_editor_with_tpl(tpl.to_string().as_bytes())?;
        let msg = Msg::from(content);

        input::ask_for_confirmation("Send the message?")?;

        println!("Sending…");
        smtp::send(&account, &msg.to_sendable_msg()?)?;
        imap_conn.append_msg("Sent", &msg.to_vec()?)?;
        println!("Done!");

        imap_conn.logout();
    }

    if let Some(matches) = matches.subcommand_matches("template") {
        let config = Config::new_from_file()?;
        let account = config.find_account_by_name(account_name)?;
        let mut imap_conn = ImapConnector::new(&account)?;

        if let Some(_) = matches.subcommand_matches("new") {
            let tpl = Msg::build_new_tpl(&config, &account)?;
            print(&output_type, &tpl)?;
        }

        if let Some(matches) = matches.subcommand_matches("reply") {
            let uid = matches.value_of("uid").unwrap();
            let mbox = matches.value_of("mailbox").unwrap();

            let msg = Msg::from(imap_conn.read_msg(&mbox, &uid)?);
            let tpl = if matches.is_present("reply-all") {
                msg.build_reply_all_tpl(&config, &account)?
            } else {
                msg.build_reply_tpl(&config, &account)?
            };

            print(&output_type, &tpl)?;
        }

        if let Some(matches) = matches.subcommand_matches("forward") {
            let uid = matches.value_of("uid").unwrap();
            let mbox = matches.value_of("mailbox").unwrap();

            let msg = Msg::from(imap_conn.read_msg(&mbox, &uid)?);
            let tpl = msg.build_forward_tpl(&config, &account)?;

            print(&output_type, &tpl)?;
        }
    }

    if let Some(matches) = matches.subcommand_matches("reply") {
        let config = Config::new_from_file()?;
        let account = config.find_account_by_name(account_name)?;
        let mut imap_conn = ImapConnector::new(&account)?;

        let mbox = matches.value_of("mailbox").unwrap();
        let uid = matches.value_of("uid").unwrap();

        let msg = Msg::from(imap_conn.read_msg(&mbox, &uid)?);
        let tpl = if matches.is_present("reply-all") {
            msg.build_reply_all_tpl(&config, &account)?
        } else {
            msg.build_reply_tpl(&config, &account)?
        };

        let content = input::open_editor_with_tpl(&tpl.to_string().as_bytes())?;
        let msg = Msg::from(content);

        input::ask_for_confirmation("Send the message?")?;

        println!("Sending…");
        smtp::send(&account, &msg.to_sendable_msg()?)?;
        imap_conn.append_msg("Sent", &msg.to_vec()?)?;
        println!("Done!");

        imap_conn.logout();
    }

    if let Some(matches) = matches.subcommand_matches("forward") {
        let config = Config::new_from_file()?;
        let account = config.find_account_by_name(account_name)?;
        let mut imap_conn = ImapConnector::new(&account)?;

        let mbox = matches.value_of("mailbox").unwrap();
        let uid = matches.value_of("uid").unwrap();

        let msg = Msg::from(imap_conn.read_msg(&mbox, &uid)?);
        let tpl = msg.build_forward_tpl(&config, &account)?;
        let content = input::open_editor_with_tpl(&tpl.to_string().as_bytes())?;
        let msg = Msg::from(content);

        input::ask_for_confirmation("Send the message?")?;

        println!("Sending…");
        smtp::send(&account, &msg.to_sendable_msg()?)?;
        imap_conn.append_msg("Sent", &msg.to_vec()?)?;
        println!("Done!");

        imap_conn.logout();
    }

    if let Some(matches) = matches.subcommand_matches("send") {
        let config = Config::new_from_file()?;
        let account = config.find_account_by_name(account_name)?;
        let mut imap_conn = ImapConnector::new(&account)?;

        let msg = matches.value_of("message").unwrap();
        let msg = Msg::from(msg.to_string());

        smtp::send(&account, &msg.to_sendable_msg()?)?;
        imap_conn.append_msg("Sent", &msg.to_vec()?)?;
        imap_conn.logout();
    }

    if let Some(matches) = matches.subcommand_matches("save") {
        let config = Config::new_from_file()?;
        let account = config.find_account_by_name(account_name)?;
        let mut imap_conn = ImapConnector::new(&account)?;

        let mbox = matches.value_of("mailbox").unwrap();
        let msg = matches.value_of("message").unwrap();
        let msg = Msg::from(msg.to_string());

        imap_conn.append_msg(mbox, &msg.to_vec()?)?;
        imap_conn.logout();
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
