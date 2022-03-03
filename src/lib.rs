pub mod mbox {
    pub mod mbox;
    pub use mbox::*;

    pub mod mbox_arg;
    pub mod mbox_handler;
}

pub mod msg {
    pub mod envelope;
    pub use envelope::*;

    pub mod msg_arg;

    pub mod msg_handler;
    pub mod msg_utils;

    pub mod flag_arg;
    pub mod flag_handler;

    pub mod tpl_arg;
    pub use tpl_arg::TplOverride;

    pub mod tpl_handler;

    pub mod msg_entity;
    pub use msg_entity::*;

    pub mod parts_entity;
    pub use parts_entity::*;

    pub mod addr_entity;
    pub use addr_entity::*;
}

pub mod backends {
    pub mod backend;
    pub use backend::*;

    pub mod id_mapper;
    pub use id_mapper::*;

    pub mod imap {
        pub mod imap_arg;

        pub mod imap_backend;
        pub use imap_backend::*;

        pub mod imap_handler;

        pub mod imap_mbox;
        pub use imap_mbox::*;

        pub mod imap_mbox_attr;
        pub use imap_mbox_attr::*;

        pub mod imap_envelope;
        pub use imap_envelope::*;

        pub mod imap_flag;
        pub use imap_flag::*;

        pub mod msg_sort_criterion;
    }
    pub use self::imap::*;

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
    pub use self::maildir::*;

    #[cfg(feature = "notmuch")]
    pub use self::notmuch::*;
    #[cfg(feature = "notmuch")]
    pub mod notmuch {
        pub mod notmuch_backend;
        pub use notmuch_backend::*;

        pub mod notmuch_mbox;
        pub use notmuch_mbox::*;

        pub mod notmuch_envelope;
        pub use notmuch_envelope::*;
    }
}

pub mod smtp {
    pub mod smtp_service;
    pub use smtp_service::*;
}

pub mod config {
    pub mod config_args;
    pub mod deserialized_config;
    pub use deserialized_config::*;

    pub mod account_args;
    pub mod account_config;
    pub use account_config::*;
    pub mod deserialized_account_config;
    pub use deserialized_account_config::*;

    pub mod format;
    pub use format::*;
}

pub mod compl;
pub mod output;
pub mod ui;
