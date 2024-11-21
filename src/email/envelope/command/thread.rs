use ariadne::{Label, Report, ReportKind, Source};
use clap::Parser;
use color_eyre::Result;
use email::{
    backend::feature::BackendFeatureSource, config::Config, email::search_query,
    envelope::list::ListEnvelopesOptions, search_query::SearchEmailsQuery,
};
use pimalaya_tui::{
    himalaya::{backend::BackendBuilder, config::EnvelopesTree},
    terminal::{cli::printer::Printer, config::TomlConfig as _},
};
use std::{process::exit, sync::Arc};
use tracing::info;

use crate::{
    account::arg::name::AccountNameFlag, config::TomlConfig,
    folder::arg::name::FolderNameOptionalFlag,
};

/// Thread all envelopes.
///
/// This command allows you to thread all envelopes included in the
/// given folder.
#[derive(Debug, Parser)]
pub struct ThreadEnvelopesCommand {
    #[command(flatten)]
    pub folder: FolderNameOptionalFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,

    /// Show only threads that contain the given envelope identifier.
    #[arg(long, short)]
    pub id: Option<usize>,

    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    pub query: Option<Vec<String>>,
}

impl ThreadEnvelopesCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing thread envelopes command");

        let (toml_account_config, account_config) = config
            .clone()
            .into_account_configs(self.account.name.as_deref(), |c: &Config, name| {
                c.account(name).ok()
            })?;

        let account_config = Arc::new(account_config);
        let folder = &self.folder.name;

        let backend = BackendBuilder::new(
            Arc::new(toml_account_config),
            account_config.clone(),
            |builder| {
                builder
                    .without_features()
                    .with_thread_envelopes(BackendFeatureSource::Context)
            },
        )
        .without_sending_backend()
        .build()
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
                                .with_color(ariadne::Color::Red),
                        )
                        .finish()
                        .eprint((source, Source::from(&query)))
                        .unwrap();
                }

                exit(0)
            }
        };

        let opts = ListEnvelopesOptions {
            page: 0,
            page_size: 0,
            query,
        };

        let envelopes = match self.id {
            Some(id) => backend.thread_envelope(folder, id, opts).await,
            None => backend.thread_envelopes(folder, opts).await,
        }?;

        let tree = EnvelopesTree::new(account_config, envelopes);

        printer.out(tree)?;

        Ok(())
    }
}

// #[cfg(test)]
// mod test {
//     use email::{account::config::AccountConfig, envelope::ThreadedEnvelope};
//     use petgraph::graphmap::DiGraphMap;

//     use super::write_tree;

//     macro_rules! e {
//         ($id:literal) => {
//             ThreadedEnvelope {
//                 id: $id,
//                 message_id: $id,
//                 from: "",
//                 subject: "",
//                 date: Default::default(),
//             }
//         };
//     }

//     #[test]
//     fn tree_1() {
//         let config = AccountConfig::default();
//         let mut buf = Vec::new();
//         let mut graph = DiGraphMap::new();
//         graph.add_edge(e!("0"), e!("1"), 0);
//         graph.add_edge(e!("0"), e!("2"), 0);
//         graph.add_edge(e!("0"), e!("3"), 0);

//         write_tree(&config, &mut buf, &graph, e!("0"), String::new(), 0).unwrap();
//         let buf = String::from_utf8_lossy(&buf);

//         let expected = "
// 0
// ├─ 1
// ├─ 2
// └─ 3
// ";
//         assert_eq!(expected.trim_start(), buf)
//     }

//     #[test]
//     fn tree_2() {
//         let config = AccountConfig::default();
//         let mut buf = Vec::new();
//         let mut graph = DiGraphMap::new();
//         graph.add_edge(e!("0"), e!("1"), 0);
//         graph.add_edge(e!("1"), e!("2"), 1);
//         graph.add_edge(e!("1"), e!("3"), 1);

//         write_tree(&config, &mut buf, &graph, e!("0"), String::new(), 0).unwrap();
//         let buf = String::from_utf8_lossy(&buf);

//         let expected = "
// 0
// └─ 1
//    ├─ 2
//    └─ 3
// ";
//         assert_eq!(expected.trim_start(), buf)
//     }

//     #[test]
//     fn tree_3() {
//         let config = AccountConfig::default();
//         let mut buf = Vec::new();
//         let mut graph = DiGraphMap::new();
//         graph.add_edge(e!("0"), e!("1"), 0);
//         graph.add_edge(e!("1"), e!("2"), 1);
//         graph.add_edge(e!("2"), e!("22"), 2);
//         graph.add_edge(e!("1"), e!("3"), 1);
//         graph.add_edge(e!("0"), e!("4"), 0);
//         graph.add_edge(e!("4"), e!("5"), 1);
//         graph.add_edge(e!("5"), e!("6"), 2);

//         write_tree(&config, &mut buf, &graph, e!("0"), String::new(), 0).unwrap();
//         let buf = String::from_utf8_lossy(&buf);

//         let expected = "
// 0
// ├─ 1
// │  ├─ 2
// │  │  └─ 22
// │  └─ 3
// └─ 4
//    └─ 5
//       └─ 6
// ";
//         assert_eq!(expected.trim_start(), buf)
//     }
// }
