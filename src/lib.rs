pub mod compl;
pub mod config;
pub mod output;
pub mod ui;

pub mod backends {
    pub mod backend;
    pub use backend::*;

    pub mod imap {
        pub mod imap_arg;
        pub mod imap_backend;
        pub use imap_backend::*;
        pub mod imap_handler;
        pub mod imap_mbox;
        pub use imap_mbox::*;
        pub mod imap_mbox_attr;
        pub use imap_mbox_attr::*;
        pub mod msg_sort_criterion;
    }
    pub use self::imap::*;

    pub mod maildir {
        pub mod maildir_backend;
        pub use maildir_backend::*;

        pub mod maildir_mbox;
        pub use maildir_mbox::*;

        pub mod msg_flag;
    }
    pub use self::maildir::*;
}

pub mod smtp {
    pub mod smtp_service;
    pub use smtp_service::*;
}

pub mod mbox {
    pub mod mbox;
    pub use mbox::*;

    pub mod mbox_attr;
    pub use mbox_attr::*;

    pub mod mbox_arg;
    pub mod mbox_handler;
}

pub mod domain {
    pub mod msg;
    pub use msg::*;
}
