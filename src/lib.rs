pub mod compl;
pub mod config;
pub mod domain;
pub mod output;
pub mod ui;

pub mod backends {
    pub mod backend;
    pub use backend::*;

    pub use self::imap::ImapBackend;
    pub mod imap {
        pub mod imap_backend;
        pub use imap_backend::*;

        mod msg_sort_criteria;
        use msg_sort_criteria::*;
    }

    pub use self::maildir::MaildirBackend;
    pub mod maildir {
        pub mod maildir_backend;
        pub use maildir_backend::*;

        mod msg_flags;
        use msg_flags::*;
    }
}
