pub mod compl;
pub mod config;
pub mod output;
pub mod ui;

pub mod backends {
    pub use backend::*;
    pub mod backend;

    pub use self::imap::*;
    pub mod imap {
        pub mod imap_arg;

        pub use imap_backend::*;
        pub mod imap_backend;

        pub mod imap_handler;

        pub use imap_mbox::*;
        pub mod imap_mbox;

        pub use imap_mbox_attr::*;
        pub mod imap_mbox_attr;

        pub use imap_envelope::*;
        pub mod imap_envelope;

        pub use imap_flag::*;
        pub mod imap_flag;

        pub mod msg_sort_criterion;
    }

    pub use self::maildir::*;
    pub mod maildir {
        pub mod maildir_backend;
        pub use maildir_backend::*;

        pub mod maildir_mbox;
        pub use maildir_mbox::*;

        pub mod maildir_envelope;
        pub use maildir_envelope::*;

        pub mod maildir_flag;
        pub use maildir_flag::*;
    }
}

pub mod smtp {
    pub mod smtp_service;
    pub use smtp_service::*;
}

pub mod mbox {
    pub mod mbox;
    pub use mbox::*;

    pub mod mbox_arg;
    pub mod mbox_handler;
}

pub mod msg {
    pub mod envelope;
    pub use envelope::*;

    pub mod flag;
    pub use flag::*;
}

pub mod domain {
    pub mod msg;
    pub use msg::*;
}
