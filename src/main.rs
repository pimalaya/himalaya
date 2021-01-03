mod config;
mod imap;
mod smtp;
mod table;

use clap::{App, Arg, SubCommand};
use std::io::prelude::*;
use std::{env, fs, process};

fn nem_email_tpl() -> String {
    ["To: ", "Subject: ", ""].join("\r\n")
}

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

fn main() {
    let config = config::read_file();
    let mut imap_sess = imap::login(&config);

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
        imap::list_mailboxes(&mut imap_sess).unwrap();
    }

    if let Some(matches) = matches.subcommand_matches("search") {
        let mbox = matches.value_of("mailbox").unwrap();

        if let Some(matches) = matches.values_of("query") {
            let query = matches
                .fold((false, vec![]), |(escape, mut cmds), cmd| {
                    match (cmd, escape) {
                        // Next command needs to be escaped
                        ("subject", _) | ("body", _) | ("text", _) => {
                            cmds.push(cmd.to_string());
                            (true, cmds)
                        }
                        // Escaped commands
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

            imap::read_emails(&mut imap_sess, &mbox, &query).unwrap();
        }
    }

    if let Some(matches) = matches.subcommand_matches("read") {
        let mbox = matches.value_of("mailbox").unwrap();
        let mime = matches.value_of("mime-type").unwrap();

        if let Some(uid) = matches.value_of("uid") {
            imap::read_email(&mut imap_sess, mbox, uid, mime).unwrap();
        }
    }

    if let Some(_) = matches.subcommand_matches("write") {
        let mut draft_path = env::temp_dir();
        draft_path.push("himalaya-draft.mail");

        fs::File::create(&draft_path)
            .expect("Could not create draft file")
            .write(nem_email_tpl().as_bytes())
            .expect("Could not write into draft file");

        process::Command::new(env!("EDITOR"))
            .arg(&draft_path)
            .status()
            .expect("Could not start $EDITOR");

        let mut draft = String::new();
        fs::File::open(&draft_path)
            .expect("Could not open draft file")
            .read_to_string(&mut draft)
            .expect("Could not read draft file");

        fs::remove_file(&draft_path).expect("Could not remove draft file");

        smtp::send(&config, &draft.as_bytes());
    }
}
