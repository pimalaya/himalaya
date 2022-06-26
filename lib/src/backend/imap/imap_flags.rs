use crate::{
    backend::from_imap_flag,
    msg::{Flag, Flags},
};

pub fn into_imap_flags<'a>(flags: &'a Flags) -> Vec<imap::types::Flag<'a>> {
    flags
        .iter()
        .map(|flag| match flag {
            Flag::Seen => imap::types::Flag::Seen,
            Flag::Answered => imap::types::Flag::Answered,
            Flag::Flagged => imap::types::Flag::Flagged,
            Flag::Deleted => imap::types::Flag::Deleted,
            Flag::Draft => imap::types::Flag::Draft,
            Flag::Recent => imap::types::Flag::Recent,
            Flag::Custom(flag) => imap::types::Flag::Custom(flag.into()),
        })
        .collect()
}

pub fn from_imap_flags(imap_flags: &[imap::types::Flag<'_>]) -> Flags {
    imap_flags.iter().map(from_imap_flag).collect()
}
