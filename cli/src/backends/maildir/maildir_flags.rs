use himalaya_lib::msg::Flags;

use super::maildir_flag;

pub fn from_maildir_entry(entry: &maildir::MailEntry) -> Flags {
    entry.flags().chars().map(maildir_flag::from_char).collect()
}
