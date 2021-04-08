use std::fmt;
use terminal_size::terminal_size;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Debug)]
pub struct Style(u8, u8, u8);

impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Style(color, bright, shade) = self;

        let bright_str: String = if *bright > 0 {
            String::from(";") + &bright.to_string()
        } else {
            String::from("")
        };

        let shade_str: String = if *shade > 0 {
            String::from(";") + &shade.to_string()
        } else {
            String::from("")
        };

        let mut style = String::from("\x1b[");
        style.push_str(&color.to_string());
        style.push_str(&bright_str);
        style.push_str(&shade_str);
        style.push_str("m");

        write!(f, "{}", style)
    }
}

#[derive(Debug)]
pub struct Cell {
    pub styles: Vec<Style>,
    pub value: String,
    pub flex: bool,
}

impl Cell {
    pub fn new(styles: &[Style], value: &str) -> Self {
        Self {
            styles: styles.to_vec(),
            value: value.trim().to_string(),
            flex: false,
        }
    }

    pub fn unicode_width(&self) -> usize {
        UnicodeWidthStr::width(self.value.as_str())
    }

    pub fn render(&self, col_size: usize) -> String {
        let style_begin = self
            .styles
            .iter()
            .map(|style| style.to_string())
            .collect::<Vec<_>>()
            .concat();
        let style_end = "\x1b[0m";
        let unicode_width = self.unicode_width();

        if col_size > 0 && unicode_width > col_size {
            String::from(style_begin + &self.value[0..=col_size - 2] + "… " + style_end)
        } else {
            let padding = if col_size == 0 {
                "".to_string()
            } else {
                " ".repeat(col_size - unicode_width + 1)
            };

            String::from(style_begin + &self.value + &padding + style_end)
        }
    }
}

#[derive(Debug)]
pub struct FlexCell;

impl FlexCell {
    pub fn new(styles: &[Style], value: &str) -> Cell {
        Cell {
            flex: true,
            ..Cell::new(styles, value)
        }
    }
}

pub trait DisplayRow {
    fn to_row(&self) -> Vec<Cell>;
}

pub trait DisplayTable<'a, T: DisplayRow + 'a> {
    fn header_row() -> Vec<Cell>;
    fn rows(&self) -> &Vec<T>;

    fn to_table(&self) -> String {
        let mut col_sizes = vec![];
        let head = Self::header_row();

        head.iter().for_each(|cell| {
            col_sizes.push(cell.unicode_width());
        });

        let mut table = self
            .rows()
            .iter()
            .map(|item| {
                let row = item.to_row();
                row.iter()
                    .enumerate()
                    .for_each(|(i, cell)| col_sizes[i] = col_sizes[i].max(cell.unicode_width()));
                row
            })
            .collect::<Vec<_>>();

        table.insert(0, head);

        let term_width = terminal_size().map(|size| size.0 .0).unwrap_or(0) as usize;
        let seps_width = 2 * col_sizes.len() - 1;
        let table_width = col_sizes.iter().sum::<usize>() + seps_width;
        let diff_width = if table_width < term_width {
            0
        } else {
            table_width - term_width
        };

        table.iter().fold(String::new(), |output, row| {
            let row_str = row
                .iter()
                .enumerate()
                .map(|(i, cell)| {
                    if cell.flex && col_sizes[i] > diff_width {
                        cell.render(col_sizes[i] - diff_width)
                    } else {
                        cell.render(col_sizes[i])
                    }
                })
                .collect::<Vec<_>>()
                .join(&Cell::new(&[ext(8)], "|").render(0));

            output + &row_str + "\n"
        })
    }
}

#[allow(dead_code)]
pub const RESET: Style = Style(0, 0, 0);

#[allow(dead_code)]
pub const BOLD: Style = Style(1, 0, 0);

#[allow(dead_code)]
pub const UNDERLINE: Style = Style(4, 0, 0);

#[allow(dead_code)]
pub const REVERSED: Style = Style(7, 0, 0);

#[allow(dead_code)]
pub const BLACK: Style = Style(30, 0, 0);

#[allow(dead_code)]
pub const RED: Style = Style(31, 0, 0);

#[allow(dead_code)]
pub const GREEN: Style = Style(32, 0, 0);

#[allow(dead_code)]
pub const YELLOW: Style = Style(33, 0, 0);

#[allow(dead_code)]
pub const BLUE: Style = Style(34, 0, 0);

#[allow(dead_code)]
pub const MAGENTA: Style = Style(35, 0, 0);

#[allow(dead_code)]
pub const CYAN: Style = Style(36, 0, 0);

#[allow(dead_code)]
pub const WHITE: Style = Style(37, 0, 0);

#[allow(dead_code)]
pub const BRIGHT_BLACK: Style = Style(30, 1, 0);

#[allow(dead_code)]
pub const BRIGHT_RED: Style = Style(31, 1, 0);

#[allow(dead_code)]
pub const BRIGHT_GREEN: Style = Style(32, 1, 0);

#[allow(dead_code)]
pub const BRIGHT_YELLOW: Style = Style(33, 1, 0);

#[allow(dead_code)]
pub const BRIGHT_BLUE: Style = Style(34, 1, 0);

#[allow(dead_code)]
pub const BRIGHT_MAGENTA: Style = Style(35, 1, 0);

#[allow(dead_code)]
pub const BRIGHT_CYAN: Style = Style(36, 1, 0);

#[allow(dead_code)]
pub const BRIGHT_WHITE: Style = Style(37, 1, 0);

#[allow(dead_code)]
pub const BG_BLACK: Style = Style(40, 0, 0);

#[allow(dead_code)]
pub const BG_RED: Style = Style(41, 0, 0);

#[allow(dead_code)]
pub const BG_GREEN: Style = Style(42, 0, 0);

#[allow(dead_code)]
pub const BG_YELLOW: Style = Style(43, 0, 0);

#[allow(dead_code)]
pub const BG_BLUE: Style = Style(44, 0, 0);

#[allow(dead_code)]
pub const BG_MAGENTA: Style = Style(45, 0, 0);

#[allow(dead_code)]
pub const BG_CYAN: Style = Style(46, 0, 0);

#[allow(dead_code)]
pub const BG_WHITE: Style = Style(47, 0, 0);

#[allow(dead_code)]
pub const BG_BRIGHT_BLACK: Style = Style(40, 1, 0);

#[allow(dead_code)]
pub const BG_BRIGHT_RED: Style = Style(41, 1, 0);

#[allow(dead_code)]
pub const BG_BRIGHT_GREEN: Style = Style(42, 1, 0);

#[allow(dead_code)]
pub const BG_BRIGHT_YELLOW: Style = Style(43, 1, 0);

#[allow(dead_code)]
pub const BG_BRIGHT_BLUE: Style = Style(44, 1, 0);

#[allow(dead_code)]
pub const BG_BRIGHT_MAGENTA: Style = Style(45, 1, 0);

#[allow(dead_code)]
pub const BG_BRIGHT_CYAN: Style = Style(46, 1, 0);

#[allow(dead_code)]
pub const BG_BRIGHT_WHITE: Style = Style(47, 1, 0);

#[allow(dead_code)]
fn ext(n: u8) -> Style {
    Style(38, 5, n)
}
