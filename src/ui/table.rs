//! Toolbox for building responsive tables.
//! A table is composed of rows, a row is composed of cells.
//! The toolbox uses the [builder design pattern].
//!
//! [builder design pattern]: https://refactoring.guru/design-patterns/builder

use log::trace;
use std::fmt;
use terminal_size;
use unicode_width::UnicodeWidthStr;

/// Define the default terminal size.
/// It is used when the size cannot be determined by the `terminal_size` crate.
const DEFAULT_TERM_WIDTH: usize = 80;

/// Define the minimum size of a shrinked cell.
/// TODO: make this customizable.
const MAX_SHRINK_WIDTH: usize = 5;

/// Wrapper around [ANSI escape codes] for styling cells.
///
/// [ANSI escape codes]: https://en.wikipedia.org/wiki/ANSI_escape_code
#[derive(Debug)]
pub struct Style(
    /// The style/color code.
    u8,
    /// The brightness code.
    u8,
    /// The shade code.
    u8,
);

impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Style(color, bright, shade) = self;
        let mut style = String::new();

        // Push first the style/color code.
        style.push_str(&color.to_string());

        // Then push the brightness code if exist.
        if *bright > 0 {
            style.push_str(";");
            style.push_str(&bright.to_string());
        };

        // Then push the shade code if exist.
        if *shade > 0 {
            style.push_str(";");
            style.push_str(&shade.to_string());
        };

        write!(f, "\x1b[{}m", style)
    }
}

/// Representation of a table cell.
#[derive(Debug)]
pub struct Cell {
    /// The list of style applied to the cell.
    styles: Vec<Style>,
    /// The content of the cell.
    value: String,
    /// Allow/disallow the cell to shrink when the table exceeds the container width.
    shrinkable: bool,
}

impl Cell {
    pub fn new<T: AsRef<str>>(value: T) -> Self {
        Self {
            styles: Vec::new(),
            value: String::from(value.as_ref()).replace(&['\r', '\n', '\t'][..], ""),
            shrinkable: false,
        }
    }

    /// Return the unicode width of the cell's value.
    pub fn unicode_width(&self) -> usize {
        UnicodeWidthStr::width(self.value.as_str())
    }

    /// Make the cell shrinkable. If the table exceeds the terminal width, this cell will be the
    /// one to shrink in order to prevent the table to overflow.
    pub fn shrinkable(mut self) -> Self {
        self.shrinkable = true;
        self
    }

    /// Return the shrinkable state of a cell.
    pub fn is_shrinkable(&self) -> bool {
        self.shrinkable
    }

    /// Apply the bold style to the cell.
    pub fn bold(mut self) -> Self {
        self.styles.push(Style(1, 0, 0));
        self
    }

    /// Apply the bold style to the cell conditionally.
    pub fn bold_if(self, predicate: bool) -> Self {
        if predicate {
            self.bold()
        } else {
            self
        }
    }

    /// Apply the underline style to the cell.
    pub fn underline(mut self) -> Self {
        self.styles.push(Style(4, 0, 0));
        self
    }

    /// Apply the red color to the cell.
    pub fn red(mut self) -> Self {
        self.styles.push(Style(31, 0, 0));
        self
    }

    /// Apply the green color to the cell.
    pub fn green(mut self) -> Self {
        self.styles.push(Style(32, 0, 0));
        self
    }

    /// Apply the yellow color to the cell.
    pub fn yellow(mut self) -> Self {
        self.styles.push(Style(33, 0, 0));
        self
    }

    /// Apply the blue color to the cell.
    pub fn blue(mut self) -> Self {
        self.styles.push(Style(34, 0, 0));
        self
    }

    /// Apply the white color to the cell.
    pub fn white(mut self) -> Self {
        self.styles.push(Style(37, 0, 0));
        self
    }

    /// Apply the custom shade color to the cell.
    pub fn ext(mut self, shade: u8) -> Self {
        self.styles.push(Style(38, 5, shade));
        self
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.styles.is_empty() {
            write!(f, "{}", self.value)?;
        } else {
            for style in &self.styles {
                write!(f, "{}", style)?;
            }
            write!(f, "{}", self.value)?;
            // Apply the reset style in order to avoid style overlapping between cells.
            write!(f, "{}", Style(0, 0, 0))?;
        }

        Ok(())
    }
}

/// Representation of a table row.
#[derive(Debug)]
pub struct Row(
    /// A row contains a list of cells.
    pub Vec<Cell>,
);

impl Row {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn cell(mut self, cell: Cell) -> Self {
        self.0.push(cell);
        self
    }
}

/// Abstract representation of a table.
pub trait Table
where
    Self: Sized,
{
    fn head() -> Row;
    fn row(&self) -> Row;

    /// Determine the max width of the table.
    /// The default implementation takes the terminal width as
    /// the maximum width of the table.
    fn max_width() -> usize {
        terminal_size::terminal_size()
            .map(|(w, _)| w.0 as usize)
            .unwrap_or(DEFAULT_TERM_WIDTH)
    }

    /// Apply styles to cells and return a list of list of printable styled cells.
    /// TODO: find a way to build an unstyled version of cells.
    fn build(items: &[Self]) -> Vec<Vec<String>> {
        let mut table = vec![Self::head()];
        let mut cell_widths: Vec<usize> =
            table[0].0.iter().map(|cell| cell.unicode_width()).collect();
        table.extend(
            items
                .iter()
                .map(|item| {
                    let row = item.row();
                    row.0.iter().enumerate().for_each(|(i, cell)| {
                        cell_widths[i] = cell_widths[i].max(cell.unicode_width());
                    });
                    row
                })
                .collect::<Vec<_>>(),
        );
        trace!("cell_widths: {:?}", cell_widths);

        let spaces_plus_separators_len = cell_widths.len() * 2 - 1;
        let table_width = cell_widths.iter().sum::<usize>() + spaces_plus_separators_len;
        trace!("table_width: {}", table_width);

        table
            .iter_mut()
            .map(|row| {
                trace!("processing row: {:?}", row);
                row.0
                    .iter_mut()
                    .enumerate()
                    .map(|(i, cell)| {
                        trace!("processing cell: {:?}", cell);
                        trace!("table_width: {}", table_width);
                        trace!("max_width: {}", Self::max_width());

                        let table_is_overflowing = table_width > Self::max_width();
                        if table_is_overflowing && cell.is_shrinkable() {
                            trace!("table is overflowing and cell is shrinkable");

                            let shrink_width = table_width - Self::max_width();
                            trace!("shrink_width: {}", shrink_width);
                            let cell_width = if shrink_width + MAX_SHRINK_WIDTH < cell_widths[i] {
                                cell_widths[i] - shrink_width
                            } else {
                                MAX_SHRINK_WIDTH
                            };
                            trace!("cell_width: {}", cell_width);
                            trace!("cell unicode_width: {}", cell.unicode_width());

                            let cell_is_overflowing = cell.unicode_width() > cell_width;
                            if cell_is_overflowing {
                                trace!("cell is overflowing");

                                let mut value = String::new();
                                let mut chars_width = 0;

                                for c in cell.value.chars() {
                                    let char_width = UnicodeWidthStr::width(c.to_string().as_str());
                                    if chars_width + char_width >= cell_width {
                                        break;
                                    }

                                    chars_width += char_width;
                                    value.push(c);
                                }

                                value.push_str("… ");
                                trace!("chars_width: {}", chars_width);
                                trace!("shrinked value: {}", value);
                                let spaces_count = cell_width - chars_width - 1;
                                trace!(
                                    "number of spaces added to shrinked value: {}",
                                    spaces_count
                                );
                                value.push_str(&" ".repeat(spaces_count));
                                cell.value = value;
                                cell.to_string()
                            } else {
                                trace!("cell is not overflowing");
                                let spaces_count = cell_width - cell.unicode_width() + 1;
                                trace!("number of spaces added to value: {}", spaces_count);
                                cell.value.push_str(&" ".repeat(spaces_count));
                                cell.to_string()
                            }
                        } else {
                            trace!("table is not overflowing or cell is not shrinkable");
                            trace!("cell_width: {}", cell_widths[i]);
                            trace!("cell unicode_width: {}", cell.unicode_width());
                            let spaces_count = cell_widths[i] - cell.unicode_width() + 1;
                            trace!("number of spaces added to value: {}", spaces_count);
                            cell.value.push_str(&" ".repeat(spaces_count));
                            cell.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    }

    /// Render the final printable table as a string.
    fn render(items: &[Self]) -> String {
        Self::build(items)
            .iter()
            // Join cells with grey pipes.
            // TODO: make this customizable
            .map(|row| row.join(&Cell::new("│").ext(8).to_string()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Item {
        id: u16,
        name: String,
        desc: String,
    }

    impl<'a> Item {
        pub fn new(id: u16, name: &'a str, desc: &'a str) -> Self {
            Self {
                id,
                name: String::from(name),
                desc: String::from(desc),
            }
        }
    }

    impl Table for Item {
        fn head() -> Row {
            Row::new()
                .cell(Cell::new("ID"))
                .cell(Cell::new("NAME").shrinkable())
                .cell(Cell::new("DESC"))
        }

        fn row(&self) -> Row {
            Row::new()
                .cell(Cell::new(self.id.to_string()))
                .cell(Cell::new(self.name.as_str()).shrinkable())
                .cell(Cell::new(self.desc.as_str()))
        }

        fn max_width() -> usize {
            // Use a fixed max width instead of terminal size for testing.
            20
        }
    }

    #[test]
    fn row_smaller_than_head() {
        let items = vec![
            Item::new(1, "a", "aa"),
            Item::new(2, "b", "bb"),
            Item::new(3, "c", "cc"),
        ];

        let table = vec![
            vec!["ID ", "NAME ", "DESC "],
            vec!["1  ", "a    ", "aa   "],
            vec!["2  ", "b    ", "bb   "],
            vec!["3  ", "c    ", "cc   "],
        ];

        assert_eq!(table, Table::build(&items));
    }

    #[test]
    fn row_bigger_than_head() {
        let items = vec![
            Item::new(1, "a", "aa"),
            Item::new(2222, "bbbbb", "bbbbb"),
            Item::new(3, "c", "cc"),
        ];

        let table = vec![
            vec!["ID   ", "NAME  ", "DESC  "],
            vec!["1    ", "a     ", "aa    "],
            vec!["2222 ", "bbbbb ", "bbbbb "],
            vec!["3    ", "c     ", "cc    "],
        ];

        assert_eq!(table, Table::build(&items));

        let items = vec![
            Item::new(1, "a", "aa"),
            Item::new(2222, "bbbbb", "bbbbb"),
            Item::new(3, "cccccc", "cc"),
        ];

        let table = vec![
            vec!["ID   ", "NAME   ", "DESC  "],
            vec!["1    ", "a      ", "aa    "],
            vec!["2222 ", "bbbbb  ", "bbbbb "],
            vec!["3    ", "cccccc ", "cc    "],
        ];

        assert_eq!(table, Table::build(&items));
    }

    #[test]
    fn basic_shrink() {
        let items = vec![
            Item::new(1, "", "desc"),
            Item::new(2, "short", "desc"),
            Item::new(3, "loooooong", "desc"),
            Item::new(4, "shriiiiink", "desc"),
            Item::new(5, "shriiiiiiiiiink", "desc"),
            Item::new(6, "😍😍😍😍", "desc"),
            Item::new(7, "😍😍😍😍😍", "desc"),
            Item::new(8, "!😍😍😍😍😍", "desc"),
        ];

        let table = vec![
            vec!["ID ", "NAME      ", "DESC "],
            vec!["1  ", "          ", "desc "],
            vec!["2  ", "short     ", "desc "],
            vec!["3  ", "loooooong ", "desc "],
            vec!["4  ", "shriiiii… ", "desc "],
            vec!["5  ", "shriiiii… ", "desc "],
            vec!["6  ", "😍😍😍😍  ", "desc "],
            vec!["7  ", "😍😍😍😍… ", "desc "],
            vec!["8  ", "!😍😍😍…  ", "desc "],
        ];

        assert_eq!(table, Table::build(&items));
    }

    #[test]
    fn max_shrink_width() {
        let items = vec![
            Item::new(1111, "shriiiiiiiink", "desc very looong"),
            Item::new(2222, "shriiiiiiiink", "desc very loooooooooong"),
        ];

        let table = vec![
            vec!["ID   ", "NAME  ", "DESC                    "],
            vec!["1111 ", "shri… ", "desc very looong        "],
            vec!["2222 ", "shri… ", "desc very loooooooooong "],
        ];

        assert_eq!(table, Table::build(&items));
    }
}
