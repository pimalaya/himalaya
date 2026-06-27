use clap::Parser;

const INBOX: &str = "Inbox";

/// CLI argument carrying the name of an m2dir folder.
#[derive(Debug, Parser)]
pub struct M2dirNameArg {
    /// Name of the m2dir folder, relative to the m2store root.
    #[arg(name = "m2dir_name", value_name = "NAME")]
    pub inner: String,
}

/// CLI flag selecting an m2dir folder by name.
#[derive(Debug, Parser)]
pub struct M2dirNameFlag {
    /// Name of the m2dir folder, relative to the m2store root.
    #[arg(name = "m2dir_source_name", long = "m2dir", short = 'm')]
    #[arg(value_name = "NAME", default_value = INBOX)]
    pub inner: String,
}

/// CLI argument carrying a single message identifier.
#[derive(Debug, Parser)]
pub struct MessageIdArg {
    /// Identifier of the message.
    #[arg(name = "message_id", value_name = "ID")]
    pub inner: String,
}

/// CLI argument carrying one or more message identifiers.
#[derive(Debug, Parser)]
pub struct MessageIdsArg {
    /// Identifier(s) of message(s).
    #[arg(name = "message_ids", value_name = "ID")]
    #[arg(num_args = 1..)]
    pub inner: Vec<String>,
}
