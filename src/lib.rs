pub mod compl;
pub mod config;
pub mod output;
pub mod ui;

pub mod backends {
    pub use backend::*;
    pub mod backend;

    pub use self::imap::ImapBackend;
    pub mod imap {
        pub mod imap_backend;
        pub use imap_backend::*;

        mod msg_sort_criterion;
        use msg_sort_criterion::*;
    }

    pub use self::maildir::MaildirBackend;
    pub mod maildir {
        pub mod maildir_backend;
        pub use maildir_backend::*;

        mod msg_flag;
        use msg_flag::*;
    }
}

pub use mbox::*;
pub mod mbox {
    pub mod mbox_arg;
    pub mod mbox_handler;

    pub use mbox_attr::*;
    pub mod mbox_attr;

    pub use attrs_entity::*;
    pub mod attrs_entity;

    pub use mbox::*;
    pub mod mbox;

    pub use mboxes_entity::*;
    pub mod mboxes_entity;
}

pub mod domain {
    pub use self::imap::*;
    pub mod imap;

    pub use msg::*;
    pub mod msg;

    pub use smtp::*;
    pub mod smtp;
}
