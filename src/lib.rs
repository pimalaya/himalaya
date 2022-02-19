pub mod compl;
pub mod config;
pub mod output;
pub mod ui;

pub mod backends {
    pub mod backend;
    pub mod imap {
        pub mod imap_backend;
        pub mod imap_mbox;
        pub mod imap_mbox_attr;
        pub mod msg_sort_criterion;

        pub use imap_backend::*;
        pub use imap_mbox::*;
        pub use imap_mbox_attr::*;
    }
    pub mod maildir {
        pub mod maildir_backend;
        pub mod maildir_mbox;
        pub mod msg_flag;

        pub use maildir_backend::*;
        pub use maildir_mbox::*;
    }

    pub use self::imap::*;
    pub use self::maildir::*;
    pub use backend::*;
}

pub mod mbox {
    pub mod mbox;
    pub mod mbox_arg;
    pub mod mbox_attr;
    pub mod mbox_handler;

    pub use mbox::*;
    pub use mbox_attr::*;
}

pub mod domain {
    pub mod imap;
    pub mod msg;
    pub mod smtp;

    pub use self::imap::*;
    pub use msg::*;
    pub use smtp::*;
}
