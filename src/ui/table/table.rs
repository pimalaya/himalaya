//! Toolbox for building responsive tables.
//! A table is composed of rows, a row is composed of cells.
//! The toolbox uses the [builder design pattern].
//!
//! [builder design pattern]: https://refactoring.guru/design-patterns/builder

use color_eyre::{eyre::Context, Result};
use email::email::config::EmailTextPlainFormat;
use termcolor::{Color, ColorSpec};
use terminal_size::terminal_size;
use tracing::trace;
use unicode_width::UnicodeWidthStr;

use crate::printer::{Print, PrintTableOpts, WriteColor};

/// Defines the default terminal size.
/// This is used when the size cannot be determined by the `terminal_size` crate.
/// TODO: make this customizable.
pub const DEFAULT_TERM_WIDTH: usize = 80;

/// Defines the minimum size of a shrunk cell.
/// TODO: make this customizable.
pub const MAX_SHRINK_WIDTH: usize = 5;

/// Represents a cell in a table.
#[derive(Debug, Default)]
pub struct Cell {
    /// Represents the style of the cell.
    style: ColorSpec,
    /// Represents the content of the cell.
    value: String,
    /// (Dis)allowes the cell to shrink when the table exceeds the container width.
    shrinkable: bool,
}

impl Cell {
    pub fn new<T: AsRef<str>>(value: T) -> Self {
        Self {
            // Removes carriage returns, new line feeds, tabulations
            // and [variation selectors].
            //
            // [variation selectors]: https://en.wikipedia.org/wiki/Variation_Selectors_(Unicode_block)
            value: String::from(value.as_ref()).replace(
                |c| ['\r', '\n', '\t', '\u{fe0e}', '\u{fe0f}'].contains(&c),
                "",
            ),
            ..Self::default()
        }
    }

    /// Returns the unicode width of the cell's value.
    pub fn unicode_width(&self) -> usize {
        UnicodeWidthStr::width(self.value.as_str())
    }

    /// Makes the cell shrinkable. If the table exceeds the terminal width, this cell will be the
    /// one to shrink in order to prevent the table to overflow.
    pub fn shrinkable(mut self) -> Self {
        self.shrinkable = true;
        self
    }

    /// Returns the shrinkable state of a cell.
    pub fn is_shrinkable(&self) -> bool {
        self.shrinkable
    }

    /// Applies the bold style to the cell.
    pub fn bold(mut self) -> Self {
        self.style.set_bold(true);
        self
    }

    /// Applies the bold style to the cell conditionally.
    pub fn bold_if(self, predicate: bool) -> Self {
        if predicate {
            self.bold()
        } else {
            self
        }
    }

    /// Applies the underline style to the cell.
    pub fn underline(mut self) -> Self {
        self.style.set_underline(true);
        self
    }

    /// Applies the red color to the cell.
    pub fn red(mut self) -> Self {
        self.style.set_fg(Some(Color::Red));
        self
    }

    /// Applies the green color to the cell.
    pub fn green(mut self) -> Self {
        self.style.set_fg(Some(Color::Green));
        self
    }

    /// Applies the yellow color to the cell.
    pub fn yellow(mut self) -> Self {
        self.style.set_fg(Some(Color::Yellow));
        self
    }

    /// Applies the blue color to the cell.
    pub fn blue(mut self) -> Self {
        self.style.set_fg(Some(Color::Blue));
        self
    }

    /// Applies the white color to the cell.
    pub fn white(mut self) -> Self {
        self.style.set_fg(Some(Color::White));
        self
    }

    /// Applies the custom ansi color to the cell.
    pub fn ansi_256(mut self, code: u8) -> Self {
        self.style.set_fg(Some(Color::Ansi256(code)));
        self
    }
}

/// Makes the cell printable.
impl Print for Cell {
    fn print(&self, writer: &mut dyn WriteColor) -> Result<()> {
        // Applies colors to the cell
        writer
            .set_color(&self.style)
            .context(format!(r#"cannot apply colors to cell "{}""#, self.value))?;

        // Writes the colorized cell to stdout
        write!(writer, "{}", self.value)
            .context(format!(r#"cannot print cell "{}""#, self.value))?;
        Ok(writer.reset()?)
    }
}

/// Represents a row in a table.
#[derive(Debug, Default)]
pub struct Row(
    /// Represents a list of cells.
    pub Vec<Cell>,
);

impl Row {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cell(mut self, cell: Cell) -> Self {
        self.0.push(cell);
        self
    }
}

/// Represents a table abstraction.
pub trait Table
where
    Self: Sized,
{
    /// Defines the header row.
    fn head() -> Row;

    /// Defines the row template.
    fn row(&self) -> Row;

    /// Writes the table to the writer.
    fn print(writer: &mut dyn WriteColor, items: &[Self], opts: PrintTableOpts) -> Result<()> {
        let is_format_flowed = matches!(opts.format, EmailTextPlainFormat::Flowed);
        let max_width = match opts.format {
            EmailTextPlainFormat::Fixed(width) => opts.max_width.unwrap_or(*width),
            EmailTextPlainFormat::Flowed => 0,
            EmailTextPlainFormat::Auto => opts
                .max_width
                .or_else(|| terminal_size().map(|(w, _)| w.0 as usize))
                .unwrap_or(DEFAULT_TERM_WIDTH),
        };
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
        trace!("cell widths: {:?}", cell_widths);

        let spaces_plus_separators_len = cell_widths.len() * 2 - 1;
        let table_width = cell_widths.iter().sum::<usize>() + spaces_plus_separators_len;
        trace!("table width: {}", table_width);

        for row in table.iter_mut() {
            let mut glue = Cell::default();
            for (i, cell) in row.0.iter_mut().enumerate() {
                glue.print(writer)?;

                let table_is_overflowing = table_width > max_width;
                if table_is_overflowing && !is_format_flowed && cell.is_shrinkable() {
                    trace!("table is overflowing and cell is shrinkable");

                    let shrink_width = table_width - max_width;
                    trace!("shrink width: {}", shrink_width);
                    let cell_width = if shrink_width + MAX_SHRINK_WIDTH < cell_widths[i] {
                        cell_widths[i] - shrink_width
                    } else {
                        MAX_SHRINK_WIDTH
                    };
                    trace!("cell width: {}", cell_width);
                    trace!("cell unicode width: {}", cell.unicode_width());

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
                        trace!("chars width: {}", chars_width);
                        trace!("shrunk value: {}", value);
                        let spaces_count = cell_width - chars_width - 1;
                        trace!("number of spaces added to shrunk value: {}", spaces_count);
                        value.push_str(&" ".repeat(spaces_count));
                        cell.value = value;
                    } else {
                        trace!("cell is not overflowing");
                        let spaces_count = cell_width - cell.unicode_width() + 1;
                        trace!("number of spaces added to value: {}", spaces_count);
                        cell.value.push_str(&" ".repeat(spaces_count));
                    }
                } else {
                    trace!("table is not overflowing or cell is not shrinkable");
                    trace!("cell width: {}", cell_widths[i]);
                    trace!("cell unicode width: {}", cell.unicode_width());
                    let spaces_count = cell_widths[i] - cell.unicode_width() + 1;
                    trace!("number of spaces added to value: {}", spaces_count);
                    cell.value.push_str(&" ".repeat(spaces_count));
                }
                cell.print(writer)?;
                glue = Cell::new("│").ansi_256(8);
            }
            writeln!(writer)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use email::email::config::EmailTextPlainFormat;
    use std::io;

    use super::*;

    #[derive(Debug, Default)]
    struct StringWriter {
        content: String,
    }

    impl io::Write for StringWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.content
                .push_str(&String::from_utf8(buf.to_vec()).unwrap());
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.content = String::default();
            Ok(())
        }
    }

    impl termcolor::WriteColor for StringWriter {
        fn supports_color(&self) -> bool {
            false
        }

        fn set_color(&mut self, _spec: &ColorSpec) -> io::Result<()> {
            io::Result::Ok(())
        }

        fn reset(&mut self) -> io::Result<()> {
            io::Result::Ok(())
        }
    }

    impl WriteColor for StringWriter {}

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
    }

    macro_rules! write_items {
        ($writer:expr, $($item:expr),*) => {
            Table::print($writer, &[$($item,)*], PrintTableOpts { format: &EmailTextPlainFormat::Auto, max_width: Some(20) }).unwrap();
        };
    }

    #[test]
    fn row_smaller_than_head() {
        let mut writer = StringWriter::default();
        write_items![
            &mut writer,
            Item::new(1, "a", "aa"),
            Item::new(2, "b", "bb"),
            Item::new(3, "c", "cc")
        ];

        let expected = concat![
            "ID │NAME │DESC \n",
            "1  │a    │aa   \n",
            "2  │b    │bb   \n",
            "3  │c    │cc   \n",
        ];
        assert_eq!(expected, writer.content);
    }

    #[test]
    fn row_bigger_than_head() {
        let mut writer = StringWriter::default();
        write_items![
            &mut writer,
            Item::new(1, "a", "aa"),
            Item::new(2222, "bbbbb", "bbbbb"),
            Item::new(3, "c", "cc")
        ];

        let expected = concat![
            "ID   │NAME  │DESC  \n",
            "1    │a     │aa    \n",
            "2222 │bbbbb │bbbbb \n",
            "3    │c     │cc    \n",
        ];
        assert_eq!(expected, writer.content);

        let mut writer = StringWriter::default();
        write_items![
            &mut writer,
            Item::new(1, "a", "aa"),
            Item::new(2222, "bbbbb", "bbbbb"),
            Item::new(3, "cccccc", "cc")
        ];

        let expected = concat![
            "ID   │NAME   │DESC  \n",
            "1    │a      │aa    \n",
            "2222 │bbbbb  │bbbbb \n",
            "3    │cccccc │cc    \n",
        ];
        assert_eq!(expected, writer.content);
    }

    #[test]
    fn basic_shrink() {
        let mut writer = StringWriter::default();
        write_items![
            &mut writer,
            Item::new(1, "", "desc"),
            Item::new(2, "short", "desc"),
            Item::new(3, "loooooong", "desc"),
            Item::new(4, "shriiiiink", "desc"),
            Item::new(5, "shriiiiiiiiiink", "desc"),
            Item::new(6, "😍😍😍😍", "desc"),
            Item::new(7, "😍😍😍😍😍", "desc"),
            Item::new(8, "!😍😍😍😍😍", "desc")
        ];

        let expected = concat![
            "ID │NAME      │DESC \n",
            "1  │          │desc \n",
            "2  │short     │desc \n",
            "3  │loooooong │desc \n",
            "4  │shriiiii… │desc \n",
            "5  │shriiiii… │desc \n",
            "6  │😍😍😍😍  │desc \n",
            "7  │😍😍😍😍… │desc \n",
            "8  │!😍😍😍…  │desc \n",
        ];
        assert_eq!(expected, writer.content);
    }

    #[test]
    fn max_shrink_width() {
        let mut writer = StringWriter::default();
        write_items![
            &mut writer,
            Item::new(1111, "shriiiiiiiink", "desc very looong"),
            Item::new(2222, "shriiiiiiiink", "desc very loooooooooong")
        ];

        let expected = concat![
            "ID   │NAME  │DESC                    \n",
            "1111 │shri… │desc very looong        \n",
            "2222 │shri… │desc very loooooooooong \n",
        ];
        assert_eq!(expected, writer.content);
    }
}
