use tui_rs::widgets::{Block, Borders, BorderType};
use tui_rs::style::Style;

#[derive(Clone)]
pub struct BlockData  {
    pub title: String,
    pub border_style: Style,
    pub style: Style,
    pub borders: Borders,
    pub border_type: BorderType,
}

impl BlockData {
    pub fn new(title: String) -> Self {
        BlockData {
            title,
            border_style: Style::default(),
            style: Style::default(),
            borders: Borders::ALL,
            border_type: BorderType::Rounded,
        }
    }
}

impl From<BlockData> for Block<'_> {
    fn from(block_data: BlockData) -> Block<'static> {
        Block::default()
            .title(block_data.title)
            .border_style(block_data.border_style)
            .style(block_data.style)
            .borders(block_data.borders)
            .border_type(block_data.border_type)
    }
}
