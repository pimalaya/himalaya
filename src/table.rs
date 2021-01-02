use std::fmt;

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
    styles: Vec<Style>,
    value: String,
}

impl Clone for Cell {
    fn clone(&self) -> Self {
        Cell::new(self.styles.clone(), self.value.clone())
    }
}

impl Cell {
    pub fn new(styles: Vec<Style>, value: String) -> Cell {
        Cell { styles, value }
    }

    pub fn printable_value_len(&self) -> usize {
        self.value.chars().collect::<Vec<_>>().len()
    }

    pub fn render(&self, col_size: usize) -> String {
        let style_start = self
            .styles
            .iter()
            .map(|style| format!("{}", style))
            .collect::<Vec<_>>()
            .concat();

        let padding = if col_size == 0 {
            "".to_string()
        } else {
            " ".repeat(col_size - self.printable_value_len() + 1)
        };

        String::from(style_start + &self.value + &padding + "\x1b[0m")
    }
}

type Matrix<T> = Vec<Vec<T>>;

pub fn transpose<T: Clone>(m: Matrix<T>) -> Matrix<T> {
    let mut tm: Matrix<T> = vec![];
    let col_size = m.iter().next().unwrap_or(&vec![]).len();

    for idx in 0..col_size {
        let col = m
            .iter()
            .map(|row| row.get(idx).unwrap().clone())
            .collect::<Vec<_>>();

        tm.push(col)
    }

    tm
}

fn render_cols(cells: Matrix<Cell>) -> Matrix<String> {
    fn render_tcols(tcells: &Vec<Cell>) -> Vec<String> {
        let col_size = tcells
            .iter()
            .map(|cell| cell.printable_value_len())
            .max()
            .unwrap();
        tcells.iter().map(|tcell| tcell.render(col_size)).collect()
    };

    let tcells: Matrix<String> = transpose(cells).iter().map(render_tcols).collect();
    transpose(tcells)
}

fn render_rows(m: Matrix<String>) -> Vec<String> {
    m.iter()
        .map(|row| String::from(row.join(&sep()) + "\n"))
        .collect()
}

pub fn render(m: Matrix<Cell>) -> String {
    render_rows(render_cols(m)).concat()
}

pub fn sep() -> String {
    Cell::new(vec![ext(8)], "|".to_string()).render(0)
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
