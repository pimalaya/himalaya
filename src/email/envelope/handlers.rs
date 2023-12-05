use anyhow::Result;
use email::account::config::AccountConfig;
use log::{debug, trace};

use crate::{
    backend::Backend,
    printer::{PrintTableOpts, Printer},
};

pub async fn list<P: Printer>(
    config: &AccountConfig,
    printer: &mut P,
    backend: &Backend,
    folder: &str,
    max_width: Option<usize>,
    page_size: Option<usize>,
    page: usize,
) -> Result<()> {
    let page_size = page_size.unwrap_or(config.email_listing_page_size());
    debug!("page size: {}", page_size);

    let envelopes = backend.list_envelopes(&folder, page_size, page).await?;
    trace!("envelopes: {:?}", envelopes);

    printer.print_table(
        Box::new(envelopes),
        PrintTableOpts {
            format: &config.email_reading_format,
            max_width,
        },
    )
}
