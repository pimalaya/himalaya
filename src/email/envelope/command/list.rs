use ariadne::{Color, Label, Report, ReportKind, Source};
use clap::Parser;
use color_eyre::Result;
use email::{
    backend::feature::BackendFeatureSource, email::search_query,
    envelope::list::ListEnvelopesOptions, search_query::SearchEmailsQuery,
};
use std::process::exit;
use tracing::info;

#[cfg(feature = "account-sync")]
use crate::cache::arg::disable::CacheDisableFlag;
use crate::{
    account::arg::name::AccountNameFlag,
    backend::Backend,
    config::TomlConfig,
    folder::arg::name::FolderNameOptionalFlag,
    printer::{PrintTableOpts, Printer},
    ui::arg::max_width::TableMaxWidthFlag,
};

/// List all envelopes.
///
/// This command allows you to list all envelopes included in the
/// given folder.
#[derive(Debug, Parser)]
pub struct ListEnvelopesCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    /// The page number.
    ///
    /// The page number starts from 1 (which is the default). Giving a
    /// page number to big will result in a out of bound error.
    #[arg(long, short, value_name = "NUMBER", default_value = "1")]
    pub page: usize,

    /// The page size.
    ///
    /// Determine the amount of envelopes a page should contain.
    #[arg(long, short = 's', value_name = "NUMBER")]
    pub page_size: Option<usize>,

    #[command(flatten)]
    pub table: TableMaxWidthFlag,

    #[cfg(feature = "account-sync")]
    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,

    /// The list envelopes filter and sort query.
    ///
    /// The query can be a filter query, a sort query or both
    /// together.
    ///
    /// A filter query is composed of operators and conditions. There
    /// is 3 operators and 8 conditions:
    ///
    ///  • not <condition> → filter envelopes that do not match the
    /// condition
    ///
    ///  • <condition> and <condition> → filter envelopes that match
    /// both conditions
    ///
    ///  • <condition> or <condition> → filter envelopes that match
    /// one of the conditions
    ///
    ///  ◦ date <yyyy-mm-dd> → filter envelopes that match the given
    /// date
    ///
    ///  ◦ before <yyyy-mm-dd> → filter envelopes with date strictly
    /// before the given one
    ///
    ///  ◦ after <yyyy-mm-dd> → filter envelopes with date stricly
    /// after the given one
    ///
    ///  ◦ from <pattern> → filter envelopes with senders matching the
    /// given pattern
    ///
    ///  ◦ to <pattern> → filter envelopes with recipients matching
    /// the given pattern
    ///
    ///  ◦ subject <pattern> → filter envelopes with subject matching
    /// the given pattern
    ///
    ///  ◦ body <pattern> → filter envelopes with text bodies matching
    /// the given pattern
    ///
    ///  ◦ flag <flag> → filter envelopes matching the given flag
    ///
    /// A sort query starts by "order by", and is composed of kinds
    /// and orders. There is 4 kinds and 2 orders:
    ///
    ///  • date [order] → sort envelopes by date
    ///
    ///  • from [order] → sort envelopes by sender
    ///
    ///  • to [order] → sort envelopes by recipient
    ///
    ///  • subject [order] → sort envelopes by subject
    ///
    ///  ◦ <kind> asc → sort envelopes by the given kind in ascending
    /// order
    ///
    ///  ◦ <kind> desc → sort envelopes by the given kind in
    /// descending order
    ///
    /// Examples:
    ///
    /// subject foo and body bar → filter envelopes containing "foo"
    /// in their subject and "bar" in their text bodies
    ///
    /// order by date desc subject → sort envelopes by descending date
    /// (most recent first), then by ascending subject
    ///
    /// subject foo and body bar order by date desc subject →
    /// combination of the 2 previous examples
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    pub query: Option<Vec<String>>,
}

impl Default for ListEnvelopesCommand {
    fn default() -> Self {
        Self {
            folder: Default::default(),
            page: 1,
            page_size: Default::default(),
            table: Default::default(),
            #[cfg(feature = "account-sync")]
            cache: Default::default(),
            account: Default::default(),
            query: Default::default(),
        }
    }
}

impl ListEnvelopesCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing list envelopes command");

        let (toml_account_config, account_config) = config.clone().into_account_configs(
            self.account.name.as_deref(),
            #[cfg(feature = "account-sync")]
            self.cache.disable,
        )?;

        let folder = &self.folder.name;
        let page = 1.max(self.page) - 1;
        let page_size = self
            .page_size
            .unwrap_or_else(|| account_config.get_envelope_list_page_size());

        let list_envelopes_kind = toml_account_config.list_envelopes_kind();

        let backend = Backend::new(
            toml_account_config.clone(),
            account_config.clone(),
            list_envelopes_kind,
            |builder| builder.set_list_envelopes(BackendFeatureSource::Context),
        )
        .await?;

        let query = self
            .query
            .map(|query| query.join(" ").parse::<SearchEmailsQuery>());
        let query = match query {
            None => None,
            Some(Ok(query)) => Some(query),
            Some(Err(main_err)) => {
                let source = "query";
                let search_query::error::Error::ParseError(errs, query) = &main_err;
                for err in errs {
                    Report::build(ReportKind::Error, source, err.span().start)
                        .with_message(main_err.to_string())
                        .with_label(
                            Label::new((source, err.span().into_range()))
                                .with_message(err.reason().to_string())
                                .with_color(Color::Red),
                        )
                        .finish()
                        .eprint((source, Source::from(&query)))
                        .unwrap();
                }

                exit(0)
            }
        };

        let opts = ListEnvelopesOptions {
            page,
            page_size,
            query,
        };

        let envelopes = backend.list_envelopes(folder, opts).await?;

        printer.print_table(
            Box::new(envelopes),
            PrintTableOpts {
                format: &account_config.get_message_read_format(),
                max_width: self.table.max_width,
            },
        )?;

        Ok(())
    }
}
