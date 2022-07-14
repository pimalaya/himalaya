use crate::msg::Flag;

pub fn from_imap_flag(imap_flag: &imap::types::Flag<'_>) -> Flag {
    match imap_flag {
        imap::types::Flag::Seen => Flag::Seen,
        imap::types::Flag::Answered => Flag::Answered,
        imap::types::Flag::Flagged => Flag::Flagged,
        imap::types::Flag::Deleted => Flag::Deleted,
        imap::types::Flag::Draft => Flag::Draft,
        imap::types::Flag::Recent => Flag::Recent,
        imap::types::Flag::MayCreate => Flag::Custom(String::from("MayCreate")),
        imap::types::Flag::Custom(flag) => Flag::Custom(flag.to_string()),
        flag => Flag::Custom(flag.to_string()),
    }
}
