use tui_rs::widgets::{Block, Borders, BorderType};
use tui_rs::style::{Style, Color};

use crate::config::tui::BlockDataConfig;

#[derive(Clone)]
pub struct BlockData  {
    pub title: String,
    pub border_style: Style,
    pub style: Style,
    pub borders: Borders,
    pub border_type: BorderType,
}

impl BlockData {
    pub fn new(title: String, config: &BlockDataConfig) -> Self {

        // ---------------------
        // Parsing settings
        // ---------------------
        let border_type = if let Some(border_type) = &config.border_type {
            match border_type.as_ref() {
                "Plain" => BorderType::Plain,
                "Rounded" => BorderType::Rounded,
                "Double" => BorderType::Double,
                "Thick" => BorderType::Thick,
                _ => BorderType::Rounded,
            }
        } else {
            BorderType::Rounded
        };

        let borders = if let Some(config_borders) = &config.borders {
            let mut borders = Borders::NONE;

            if config_borders.contains('r') {
                borders |= Borders::RIGHT;
            }

            if config_borders.contains('l') {
                borders |= Borders::LEFT;
            }

            if config_borders.contains('t') {
                borders |= Borders::TOP;
            }

            if config_borders.contains('b') {
                borders |= Borders::BOTTOM;
            }

            borders
        } else {
            Borders::ALL
        };

        let border_style = if let Some(border_color) = &config.border_color {
            let border_style = Style::default();

            match border_color.as_ref() {
                "Black"        => border_style.fg(Color::Black),
                "Red"          => border_style.fg(Color::Red),
                "Green"        => border_style.fg(Color::Green),
                "Yellow"       => border_style.fg(Color::Yellow),
                "Blue"         => border_style.fg(Color::Blue),
                "Magenta"      => border_style.fg(Color::Magenta),
                "Cyan"         => border_style.fg(Color::Cyan),
                "Gray"         => border_style.fg(Color::Gray),
                "DarkGray"     => border_style.fg(Color::DarkGray),
                "LightRed"     => border_style.fg(Color::LightRed),
                "LightGreen"   => border_style.fg(Color::LightGreen),
                "LightYellow"  => border_style.fg(Color::LightYellow),
                "LightBlue"    => border_style.fg(Color::LightBlue),
                "LightMagenta" => border_style.fg(Color::LightMagenta),
                "LightCyan"    => border_style.fg(Color::LightCyan),
                "White"        => border_style.fg(Color::White),
                _ => border_style,
            }

        } else {
            Style::default()
        };

        // -------------------
        // Creating block
        // ------------------- */
        BlockData {
            title,
            border_style,
            style: Style::default(),
            borders,
            border_type,
        }
    }
}

impl From<BlockData> for Block<'static> {
    fn from(block_data: BlockData) -> Block<'static> {
        Block::default()
            .title(block_data.title)
            .border_style(block_data.border_style)
            .style(block_data.style)
            .borders(block_data.borders)
            .border_type(block_data.border_type)
    }
}
