mod app;
mod config;
mod imap;
mod input;
mod output;
mod smtp;
mod table;
mod flag {
    pub(crate) mod cli;
    pub(crate) mod model;
}
mod msg {
    pub(crate) mod cli;
    pub(crate) mod model;
}
mod mbox {
    pub(crate) mod cli;
    pub(crate) mod model;
}

use crate::app::App;

fn main() {
    if let Err(ref errs) = App::new().run() {
        let mut errs = errs.iter();
        match errs.next() {
            None => (),
            Some(err) => {
                eprintln!("{}", err);
                errs.for_each(|err| eprintln!(" ↳ {}", err));
            }
        }
    }
}
