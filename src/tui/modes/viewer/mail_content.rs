// use tui_rs::widgets::{Block, List, ListItem, ListState};
use tui_rs::layout::Alignment;
use tui_rs::text::{Span, Spans};
use tui_rs::widgets::{Block, Paragraph, Wrap};

use crate::config::tui::block_data::BlockDataConfig;
use crate::tui::modes::block_data::BlockData;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

// ===========
// Struct
// ===========
pub struct MailContent {
    content:        String,
    parser:         SyntaxSet,
    theme:          ThemeSet,
    pub block_data: BlockData,
    pub x_offset:   u16,
    pub y_offset:   u16,
}

impl MailContent {
    pub fn new(config: &BlockDataConfig) -> Self {
        let parser = SyntaxSet::load_defaults_newlines();
        let theme = ThemeSet::load_defaults();

        Self {
            block_data: BlockData::new(String::from("Mail Content"), config),
            content: String::new(),
            theme,
            parser,
            x_offset: 0,
            y_offset: 0,
        }
    }

    pub fn set_content(&mut self, new_content: &str) {
        // Since we load a new mail, we should reset the offset
        self.x_offset = 0;
        self.y_offset = 0;
        self.content = new_content.to_string();
    }

    pub fn add_offset(&mut self, x: u16, y: u16) {
        self.x_offset = self.x_offset.saturating_add(x);
        self.y_offset = self.y_offset.saturating_add(y);
    }

    pub fn sub_offset(&mut self, x: u16, y: u16) {
        self.x_offset = self.x_offset.saturating_sub(x);
        self.y_offset = self.y_offset.saturating_sub(y);
    }

    pub fn widget(&self) -> Paragraph<'static> {
        let block = Block::from(self.block_data.clone());
        let mut content: Vec<Spans> = Vec::new();

        // -----------------------
        // Highlight document
        // -----------------------
        // for line in self.content.lines() {
        //     content.push(Spans::from(Span::raw(line.to_string())));
        // }
        let syntax = self.parser.find_syntax_by_extension("md").unwrap();
        let mut highlighter = HighlightLines::new(
            syntax,
            &self.theme.themes["Solarized (light)"],
        );

        for line in LinesWithEndings::from(&self.content) {
            let mut converted_line: Vec<Span> = Vec::new();

            let ranges: Vec<(Style, &str)> =
                highlighter.highlight(line, &self.parser);

            for piece in ranges {
                let red = piece.0.foreground.r;
                let green = piece.0.foreground.g;
                let blue = piece.0.foreground.b;
                let text_part = piece.1.trim().to_string();

                converted_line.push(Span::styled(
                    text_part,
                    tui_rs::style::Style::default()
                        .fg(tui_rs::style::Color::Rgb(red, green, blue)),
                ));
            }

            content.push(Spans::from(converted_line));
        }

        Paragraph::new(content)
            .block(block)
            .alignment(Alignment::Left)
            .style(tui_rs::style::Style::default())
            .wrap(Wrap { trim: true })
            .scroll((self.x_offset, self.y_offset))
    }
}
