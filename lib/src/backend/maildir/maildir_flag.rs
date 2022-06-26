use crate::msg::Flag;

pub fn from_char(c: char) -> Flag {
    match c {
        'r' | 'R' => Flag::Answered,
        's' | 'S' => Flag::Seen,
        't' | 'T' => Flag::Deleted,
        'd' | 'D' => Flag::Draft,
        'f' | 'F' => Flag::Flagged,
        'p' | 'P' => Flag::Custom(String::from("Passed")),
        flag => Flag::Custom(flag.to_string()),
    }
}

pub fn to_normalized_char(flag: &Flag) -> Option<char> {
    match flag {
        Flag::Answered => Some('R'),
        Flag::Seen => Some('S'),
        Flag::Deleted => Some('T'),
        Flag::Draft => Some('D'),
        Flag::Flagged => Some('F'),
        _ => None,
    }
}
