use clap::Parser;
use email::account::config::DEFAULT_INBOX_FOLDER;

/// The folder name argument parser
#[derive(Debug, Parser)]
pub struct FolderNameArg {
    /// The name of the folder
    #[arg(name = "folder-name", value_name = "FOLDER")]
    pub name: String,
}

/// The optional folder name argument parser
#[derive(Debug, Parser)]
pub struct FolderNameOptionalArg {
    /// The name of the folder
    #[arg(name = "folder-name", value_name = "FOLDER", default_value = DEFAULT_INBOX_FOLDER)]
    pub name: String,
}

/// The source folder name argument parser
#[derive(Debug, Parser)]
pub struct SourceFolderNameArg {
    /// The name of the source folder
    #[arg(name = "from-folder-name", value_name = "FROM")]
    pub name: String,
}

/// The target folder name argument parser
#[derive(Debug, Parser)]
pub struct TargetFolderNameArg {
    /// The name of the target folder
    #[arg(name = "to-folder-name", value_name = "TO")]
    pub name: String,
}
