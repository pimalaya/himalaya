mod config;
mod imap;
mod table;

use clap::{App, Arg, SubCommand};

fn mailbox_arg() -> Arg<'static, 'static> {
    Arg::with_name("mailbox")
        .help("Name of the targeted mailbox")
        .value_name("MAILBOX")
        .required(true)
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
        .subcommand(
            SubCommand::with_name("query")
                .about("Prints emails filtered by the given IMAP query")
                .arg(mailbox_arg())
                .arg(
                    Arg::with_name("query")
                        .help("IMAP query (see https://tools.ietf.org/html/rfc3501#section-6.4.4)")
                        .value_name("COMMANDS")
                        .multiple(true)
                        .required(true),
                ),
        )
        .subcommand(SubCommand::with_name("list").about("Lists all available mailboxes"))
        .subcommand(
            SubCommand::with_name("read")
                .about("Reads an email by its UID")
                .arg(mailbox_arg())
                .arg(uid_arg()),
        )
        .subcommand(SubCommand::with_name("write").about("Writes a new email"))
        .subcommand(
            SubCommand::with_name("forward")
                .about("Forwards an email by its UID")
                .arg(mailbox_arg())
                .arg(uid_arg()),
        )
        .subcommand(
            SubCommand::with_name("reply")
                .about("Replies to an email by its UID")
                .arg(mailbox_arg())
                .arg(uid_arg())
                .arg(
                    Arg::with_name("reply all")
                        .help("Replies to all recipients")
                        .short("a")
                        .long("all"),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("query") {
        let mbox = matches.value_of("mailbox").unwrap_or("inbox");

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

    if let Some(_) = matches.subcommand_matches("list") {
        imap::list_mailboxes(&mut imap_sess).unwrap();
    }
}
