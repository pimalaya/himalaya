pub mod config;
pub mod ctx;
pub mod flag;
pub mod imap;
pub mod input;
pub mod mbox;
pub mod msg;
pub mod output;
pub mod smtp;
pub mod table;

/// This module includes all relevant functions which are related to the CLI
/// call of himalaya. For example if you want to call `himalaya completion` than
/// you're actually refering to `src/cli/shell_completion` and then to its
/// functions.
///
/// Each module (file) in this directory can have these main functions which can
/// be used to create the subcommands and/or options for `himalaya` like
/// `himalaya --config <PATH>`:
///
/// - `matches`
/// - `subcmds`
/// - `options`
///
/// What they are doing is gonna be explained in the following sections.
///
/// # The `*_arg` functions
/// There are also other functions with the `_arg` prefix. These are used to add
/// some `<ARG>` arguments. They are not in the `options` function because they
/// are meant to be used multiple times in combination with subcommands. The
/// [`himalaya::cli::mbox::target_arg`] is for examply used for the `himalaya
/// copy` and `himalaya move` subcommand.
///
///
/// ## Example
/// ```rust
/// use himalaya::cli;
/// use clap;
///
/// fn main() {
///     let app = clap::App::new("Example")
///         .version("1.0")
///         .author("Someone")
///         // let's create a subcommand which needs the `<TARGET>` argument
///         .subcommands(clap::SubCommand::with_name("Test")
///             .about("A neat test subcommand")
///             // here we're saying that we need the `[TARGET]` option for this
///             // subcommand
///             .arg(cli::mbox::target_arg())
///         );
/// }
/// ```
///
/// # The `subcmds` function
/// The `subcmds` function always returns the some subcommands which are listed
/// under the `SUBCOMMANDS` section of `himalaya --help`. Here's a part of it:
///
/// ```no_exec
/// SUBCOMMANDS:
///     attachments    Downloads all message attachments
///     completion     Generates the completion script for the given shell
/// ```
///
/// The `completion` subcommand comes from the `shell_completion::subcmds` for
/// example. To sum it up: You can call the `subcmds` function to register some
/// subcmds for the CLI. Take a look into the example in the `matches` section
/// below to see how you can use it.
///
/// # The `matches` function
/// This function is used to check if the given subcommand is given by the user.
/// It'll return `true` if a subcmd of the file was used or not.
/// So in this case, if the user called `himalaya completion` than we're doing
/// the appropriate action: Generate the file for the shell and return `true`.
///
/// ## Example
/// ```rust
/// use clap;
/// use himalaya::cli;
///
/// fn main() {
///     let app = clap::App::new("Example")
///         .version("1.0")
///         .author("Someone")
///         // get the subcommands which are related for the shell-completion
///         .subcommands(cli::shell_completion::subcmds());
///
///     // look, which subcommands the user provided
///     let arg_matches = app.get_matches();
///
///     // Now look, if the user called himalaya like that: 
///     // `himalaya completion`. If yes, do what you need to do and enter the if
///     // clause
///     if cli::shell_completion::matches(app, &arg_matches) {
///         println!("User provided the 'completion' subcommand!");
///     } else {
///         println!("User didn't provide the 'completion' subcommand...");
///     }
/// }
/// ```
///
/// # The `options` function
/// This kind of function returns a vector with the available options which you
/// call like this: `himalaya --config <PATH>`.
///
/// ## Example
/// ```rust
/// use clap;
/// use himalaya::cli;
///
/// fn main() {
///     let app = clap::App::new("Example")
///         .version("1.0")
///         .author("Someone")
///         // register the possible options which are related to the config
///         // file
///         .args(&cli::config::options());
///
///     // look which options the user provided
///     let arg_matches = app.get_matches();
///
///     // Now get the path which should be provided after the `--config` option
///     let config_path: Option<PathBuf> = arg_matches
///             .value_of("config")
///             .map(|s| s.into());
/// }
/// ```
///
/// [`shell_completion`]: shell_completion/index.html
/// [`himalaya::cli::mbox::target_arg`]: mbox/fn.target_arg.html
pub mod cli;
