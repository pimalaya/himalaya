use crate::msg::Flags;

use super::maildir_flag;

pub fn from_maildir_entry(entry: &maildir::MailEntry) -> Flags {
    entry.flags().chars().map(maildir_flag::from_char).collect()
}

pub fn to_normalized_string(flags: &Flags) -> String {
    String::from_iter(flags.iter().filter_map(maildir_flag::to_normalized_char))
}
