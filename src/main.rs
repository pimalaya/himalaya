mod config;
mod imap;
mod table;

use std::env;

fn main() {
    let mbox = env::args().nth(1).unwrap_or(String::from("inbox"));
    let args = env::args().skip(2).collect::<Vec<_>>().join(" ").to_owned();
    let config = config::read_file();
    let mut imap_sess = imap::login(&config);

    match args.as_str() {
        "read new" => imap::read_new_emails(&mut imap_sess, &mbox).unwrap(),
        _ => println!("Himalaya: command not found e"),
    }
}
