pub mod mbox {
    pub mod mbox;
    pub use mbox::*;

    pub mod mboxes;
    pub use mboxes::*;

    pub mod mbox_args;
    pub mod mbox_handlers;
}

#[cfg(feature = "imap-backend")]
pub mod imap {
    pub mod imap_args;
    pub mod imap_handlers;

    pub mod imap_envelopes;
    pub use imap_envelopes::*;
}

pub mod msg {
    pub mod envelope;
    pub use envelope::*;

    pub mod envelopes;
    pub use envelopes::*;

    pub mod msg_args;

    pub mod msg_handlers;

    pub mod flag_args;
    pub mod flag_handlers;

    pub mod tpl_args;

    pub mod tpl_handlers;
}

pub mod smtp {
    pub mod smtp_service;
    pub use smtp_service::*;
}

pub mod config {
    pub mod config_args;

    pub mod account_args;
    pub mod account_handlers;

    pub mod account;
    pub use account::*;
}

pub mod compl;
pub mod output;
pub mod ui;
