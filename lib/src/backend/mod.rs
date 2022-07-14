pub mod backend;
pub use backend::*;

pub mod id_mapper;
pub use id_mapper::*;

#[cfg(feature = "imap-backend")]
pub mod imap {
    pub mod imap_backend;
    pub use imap_backend::*;

    pub mod imap_envelopes;
    pub use imap_envelopes::*;

    pub mod imap_envelope;
    pub use imap_envelope::*;

    pub mod imap_flags;
    pub use imap_flags::*;

    pub mod imap_flag;
    pub use imap_flag::*;

    pub mod msg_sort_criterion;

    pub mod error;
    pub use error::*;
}

#[cfg(feature = "imap-backend")]
pub use self::imap::*;

#[cfg(feature = "maildir-backend")]
pub mod maildir {
    pub mod maildir_backend;
    pub use maildir_backend::*;

    pub mod maildir_envelopes;
    pub use maildir_envelopes::*;

    pub mod maildir_envelope;
    pub use maildir_envelope::*;

    pub mod maildir_flags;
    pub use maildir_flags::*;

    pub mod maildir_flag;
    pub use maildir_flag::*;

    pub mod error;
    pub use error::*;
}

#[cfg(feature = "maildir-backend")]
pub use self::maildir::*;

#[cfg(feature = "notmuch-backend")]
pub mod notmuch {
    pub mod notmuch_backend;
    pub use notmuch_backend::*;

    pub mod notmuch_envelopes;
    pub use notmuch_envelopes::*;

    pub mod notmuch_envelope;
    pub use notmuch_envelope::*;

    pub mod error;
    pub use error::*;
}

#[cfg(feature = "notmuch-backend")]
pub use self::notmuch::*;
