use himalaya_lib::msg::Flag;

pub fn from_char(c: char) -> Flag {
    match c {
        'R' => Flag::Answered,
        'S' => Flag::Seen,
        'T' => Flag::Deleted,
        'D' => Flag::Draft,
        'F' => Flag::Flagged,
        'P' => Flag::Custom(String::from("Passed")),
        flag => Flag::Custom(flag.to_string()),
    }
}
