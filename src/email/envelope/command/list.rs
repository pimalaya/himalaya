use anyhow::Result;
use ariadne::{Color, Label, Report, ReportKind, Source};
use clap::Parser;
use email::{backend::feature::BackendFeatureSource, envelope::list::ListEnvelopesOptions};
use log::info;

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
    /// TODO
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

        let filter = match self.query.map(|filter| filter.join(" ").parse()) {
            Some(Ok(filter)) => Some(filter),
            Some(Err(err)) => {
                if let email::envelope::list::Error::ParseFilterError(errs, query) = &err {
                    errs.into_iter().for_each(|e| {
                        Report::build(ReportKind::Error, "query", e.span().start)
                            .with_message(e.to_string())
                            .with_label(
                                Label::new(("query", e.span().into_range()))
                                    .with_message(e.reason().to_string())
                                    .with_color(Color::Red),
                            )
                            .finish()
                            .eprint(("query", Source::from(&query)))
                            .unwrap()
                    });
                };

                Err(err)?;
                None
            }
            None => None,
        };

        let opts = ListEnvelopesOptions {
            page,
            page_size,
            filter,
            sort: Default::default(),
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
