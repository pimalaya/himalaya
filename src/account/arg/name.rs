use clap::Parser;

/// The account name argument parser
#[derive(Debug, Parser)]
pub struct AccountNameArg {
    /// The name of the account
    ///
    /// The account names are taken from the table at the root level
    /// of your TOML configuration file.
    #[arg(value_name = "ACCOUNT")]
    pub name: String,
}

/// The account name flag parser
#[derive(Debug, Parser)]
pub struct AccountNameFlag {
    /// Override the default account
    #[arg(
        long = "account",
        short = 'a',
        name = "account-name",
        value_name = "NAME",
        global = true
    )]
    pub name: Option<String>,
}
